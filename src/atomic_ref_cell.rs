use std::cell::UnsafeCell;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};

const UNUSED: usize = 0;
const WRITING: usize = !0;

/// A thread-safe, mutable memory location with dynamically checked borrow rules.
///
/// `AtomicRefCell` behaves the same as [`RefCell`][refcell] except that it internally tracks
/// borrow state with an atomic counter, making it safe to share between threads. This is useful
/// for cases where you want to share some data between theads, but know in advance that only one
/// thread will try to mutate it at a time. In this case, `AtomicRefCell` is more appropriate than
/// a [`Mutex`][mutex] or [`RwLock`][rwlock] because it will never block if two threads try to
/// access it mutably at once. Instead it will panic, which will help track down any bugs that
/// would cause that to happen.
///
/// The `borrow()` and `borrow_mut()` methods allow you to create a runtime borrow of the cell's
/// data. `borrow()` returns an `AtomicRef<T>` which can be treated as a `&T`, and `borrow_mut()`
/// returns an `AtomicRefMut<T>` which can be treated as a `&mut T`. The borrow rules for
/// `AtomicRefCell` are the same as the rules enforced by the compiler: You may have either (0 or
/// more immutable borrows) OR (0 or 1 mutable borrow). A call to `borrow()` while there
/// is an active mutable borrow, or a call to `borrow_mut()` while there is an active immutable
/// borrow, will result in a panic. If panicking is not desirable, `try_borrow()` and
/// `try_borrow_mut()` return an `Option<AtomicRef<T>>` and an `Option<AtomicRefMut<T>>`,
/// respectively, both returning `None` if the borrow is not possible at that time.
///
/// [refcell]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
/// [mutex]: https://doc.rust-lang.org/std/sync/struct.Mutex.html
/// [rwlock]: https://doc.rust-lang.org/std/sync/struct.RwLock.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use cell_extras::AtomicRefCell;
/// use std::sync::Arc;
/// use std::thread;
///
/// let cell = Arc::new(AtomicRefCell::new("foo".to_string()));
///
/// let clone = cell.clone();
/// thread::spawn(move || {
///     let mut string = clone.borrow_mut();
///     string.push_str("bar");
///
///     assert_eq!("foobar", &*string);
/// }).join().unwrap();
///
/// let mut string = cell.borrow_mut();
/// string.push_str("baz");
/// assert_eq!("foobarbaz", &*string);
/// ```
pub struct AtomicRefCell<T> {
    borrow: AtomicUsize,
    value: UnsafeCell<T>
}

impl<T> AtomicRefCell<T> {
    /// Create a new `AtomicRefCell` containing `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    ///
    /// let cell = AtomicRefCell::new(5);
    /// ```
    pub fn new(value: T) -> AtomicRefCell<T> {
        AtomicRefCell {
            borrow: AtomicUsize::new(UNUSED),
            value: UnsafeCell::new(value),
        }
    }

    /// Consumes the `AtomicRefCell`, returning the wrapped value.
    ///
    /// This is always safe to do because you must consume the `AtomicRefCell`, which cannot happen
    /// if there are any active borrows of its value.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    ///
    /// let cell = AtomicRefCell::new(5);
    ///
    /// let inner = cell.into_inner();
    /// assert_eq!(5, inner);
    /// ```
    pub fn into_inner(self) -> T {
        debug_assert!(self.borrow.load(Ordering::SeqCst) == UNUSED);
        unsafe { self.value.into_inner() }
    }

    /// Immutably borrow the wrapped value.
    ///
    /// The borrow lasts until the returned `AtomicRef` exits scope or is otherwise dropped.
    /// Leaking the returned `AtomicRef` will result in the borrow never ending, so don't do that.
    ///
    /// # Panics
    ///
    /// - If the value is currently mutably borrowed. For a non-panicking variant, use `try_borrow()`.
    ///
    /// # Examples
    ///
    /// Borrow the cell multiple times across multiple threads:
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let cell = Arc::new(AtomicRefCell::new(7));
    ///
    /// let borrow0 = cell.borrow();
    ///
    /// let clone = cell.clone();
    /// thread::spawn(move || {
    ///     let borrow1 = clone.borrow();
    ///     let borrow2 = clone.borrow();
    ///
    ///     // `borrow1` and `borrow2` end here.
    /// }).join().unwrap();
    ///
    /// let borrow1 = cell.borrow();
    /// let borrow2 = cell.borrow();
    /// ```
    ///
    /// An example of panic:
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let cell = Arc::new(AtomicRefCell::new(7));
    ///
    /// let borrow0 = cell.borrow_mut();
    ///
    /// let clone = cell.clone();
    /// let result = thread::spawn(move || {
    ///     let borrow1 = clone.borrow(); // This causes a panic.
    /// }).join();
    ///
    /// assert!(result.is_err());
    /// ```
    pub fn borrow(&self) -> AtomicRef<T> {
        self.try_borrow().expect("Already mutably borrowed")
    }

    /// Immutably borrow the wrapped value if it's not currently borrowed mutably.
    ///
    /// The borrow lasts until the returned `AtomicRef` exits scope or is otherwise dropped.
    /// Leaking the returned `AtomicRef` will result in the borrow never ending, so don't do that.
    ///
    /// This is the non-panicking version of `borrow()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    ///
    /// let cell = AtomicRefCell::new(5);
    ///
    /// {
    ///     let borrow = cell.borrow();
    ///     assert!(cell.try_borrow().is_some());
    /// }
    ///
    /// {
    ///     let mut_borrow = cell.borrow_mut();
    ///     assert!(cell.try_borrow().is_none());
    /// }
    /// ```
    pub fn try_borrow(&self) -> Option<AtomicRef<T>> {
        // NOTE: We can't just do `self.borrow.fetch_add(1) != WRITING` because `WRITING` is
        // `usize::MAX`, and adding 1 to it would overflow the value to `UNUSED`, potentially
        // allowing another thread to mutably or immutably borrow the the cell while it's already
        // mutably borrowed. Instead we have to compare-and-swap loop until we can be sure we
        // set the borrow counter correctly.
        loop {
            let borrow = self.borrow.load(Ordering::SeqCst);
            if borrow == WRITING { return None }

            if self.borrow.compare_and_swap(borrow, borrow + 1, Ordering::SeqCst) == borrow {
                return Some(AtomicRef {
                    value: unsafe { &*self.value.get() },
                    borrow: BorrowGuard(&self.borrow),
                })
            }
        }
    }

    /// Mutably borrow the wrapped value.
    ///
    /// The borrow lasts until the returned `AtomicRefMut` exits scope or is otherwise dropped.
    /// Leaking the returned `AtomicRefMut` will result in the borrow never ending. This is even
    /// worse than leaking an `AtomicRef` because it will no longer be possible to borrow the
    /// contents mutable or immutably, the `AtomicRefCell` will efectively become fused. So don't
    /// do it.
    ///
    /// # Panics
    ///
    /// - If the value is currently immutably borrowed. For a non-panicking variant, use `try_borrow_mut()`.
    ///
    /// # Examples
    ///
    /// Borrow the cell across multiple threads:
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let cell = Arc::new(AtomicRefCell::new(7));
    ///
    /// let clone = cell.clone();
    /// thread::spawn(move || {
    ///     let mut borrow1 = clone.borrow_mut();
    ///     *borrow1 += 12;
    ///
    ///     // `borrow1` ends here.
    /// }).join().unwrap();
    ///
    /// let borrow = cell.borrow();
    /// assert_eq!(19, *borrow);
    /// ```
    ///
    /// An example of panic:
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// let cell = Arc::new(AtomicRefCell::new(7));
    ///
    /// let borrow0 = cell.borrow();
    ///
    /// let clone = cell.clone();
    /// let result = thread::spawn(move || {
    ///     let borrow1 = clone.borrow_mut(); // This causes a panic.
    /// }).join();
    ///
    /// assert!(result.is_err());
    /// ```
    pub fn borrow_mut(&self) -> AtomicRefMut<T> {
        self.try_borrow_mut().expect("Already immutably borrowed")
    }

    /// Mutably borrow the wrapped value if it's not currently borrowed.
    ///
    /// The borrow lasts until the returned `AtomicRefMut` exits scope or is otherwise dropped.
    /// Leaking the returned `AtomicRefMut` will result in the borrow never ending. This is even
    /// worse than leaking an `AtomicRef` because it will no longer be possible to borrow the
    /// contents mutable or immutably, the `AtomicRefCell` will efectively become fused. So don't
    /// do it.
    ///
    /// This is the non-panicking version of `borrow_mut()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::AtomicRefCell;
    ///
    /// let cell = AtomicRefCell::new(5);
    ///
    /// assert!(cell.try_borrow_mut().is_some());
    ///
    /// {
    ///     let borrow = cell.borrow();
    ///     assert!(cell.try_borrow_mut().is_none());
    /// }
    ///
    /// {
    ///     let mut_borrow = cell.borrow_mut();
    ///     assert!(cell.try_borrow().is_none());
    /// }
    /// ```
    pub fn try_borrow_mut(&self) -> Option<AtomicRefMut<T>> {
        if self.borrow.compare_and_swap(UNUSED, WRITING, Ordering::SeqCst) == UNUSED {
            return Some(AtomicRefMut {
                value: unsafe { &mut *self.value.get() },
                borrow: MutBorrowGuard(&self.borrow),
            });
        } else {
            return None
        }
    }
}

impl<T> Debug for AtomicRefCell<T> where T: Debug {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        if let Some(value) = self.try_borrow() {
            write!(formatter, "AtomicRefCell {{ value: {:?} }}", value)
        } else {
            write!(formatter, "AtomicRefCell {{ value: <borrowed> }}")
        }
    }
}

unsafe impl<T> Send for AtomicRefCell<T> where T: Send + Sync {}
unsafe impl<T> Sync for AtomicRefCell<T> where T: Sync + Sync {}

pub struct AtomicRef<'a, T: 'a> {
    value: &'a T,
    borrow: BorrowGuard<'a>,
}

impl<'a, T: 'a> AtomicRef<'a, T> {
    /// Make a new `AtomicRef` for a component of the borrowed data.
    ///
    /// The `AtomicRefCell` is already immutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as `AtomicRef::map(...)`.
    /// A method would interfere with methods of the same name on the contents
    /// of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Example
    ///
    /// ```
    /// use cell_extras::atomic_ref_cell::{AtomicRefCell, AtomicRef};
    ///
    /// let c = AtomicRefCell::new((5, 'b'));
    /// let b1: AtomicRef<(u32, char)> = c.borrow();
    /// let b2: AtomicRef<u32> = AtomicRef::map(b1, |t| &t.0);
    /// assert_eq!(*b2, 5)
    /// ```
    #[inline]
    pub fn map<U, F>(orig: AtomicRef<'a, T>, f: F) -> AtomicRef<'a, U>
        where F: FnOnce(&T) -> &U
    {
        AtomicRef {
            value: f(orig.value),
            borrow: orig.borrow,
        }
    }
}

impl<'a, T: 'a> Deref for AtomicRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: 'a> Debug for AtomicRef<'a, T> where T: Debug {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        (&**self).fmt(formatter)
    }
}

pub struct AtomicRefMut<'a, T: 'a> {
    value: &'a mut T,
    borrow: MutBorrowGuard<'a>,
}

impl<'a, T: 'a> AtomicRefMut<'a, T> {
    /// Make a new `AtomicRefMut` for a component of the borrowed data, e.g. an enum
    /// variant.
    ///
    /// The `AtomicRefCell` is already mutably borrowed, so this cannot fail.
    ///
    /// This is an associated function that needs to be used as
    /// `AtomicRefMut::map(...)`.  A method would interfere with methods of the same
    /// name on the contents of a `AtomicRefCell` used through `Deref`.
    ///
    /// # Example
    ///
    /// ```
    /// use cell_extras::atomic_ref_cell::{AtomicRefCell, AtomicRefMut};
    ///
    /// let c = AtomicRefCell::new((5, 'b'));
    /// {
    ///     let b1: AtomicRefMut<(u32, char)> = c.borrow_mut();
    ///     let mut b2: AtomicRefMut<u32> = AtomicRefMut::map(b1, |t| &mut t.0);
    ///     assert_eq!(*b2, 5);
    ///     *b2 = 42;
    /// }
    /// assert_eq!(*c.borrow(), (42, 'b'));
    /// ```
    #[inline]
    pub fn map<U, F>(orig: AtomicRefMut<'a, T>, f: F) -> AtomicRefMut<'a, U>
        where F: FnOnce(&mut T) -> &mut U
    {
        AtomicRefMut {
            value: f(orig.value),
            borrow: orig.borrow,
        }
    }
}

impl<'a, T: 'a> Deref for AtomicRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T { self.value }
}

impl<'a, T: 'a> DerefMut for AtomicRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T { self.value }
}

impl<'a, T: 'a> Debug for AtomicRefMut<'a, T> where T: Debug {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        (&**self).fmt(formatter)
    }
}

struct BorrowGuard<'a>(&'a AtomicUsize);

impl<'a> Drop for BorrowGuard<'a> {
    fn drop(&mut self) {
        let last = self.0.fetch_sub(1, Ordering::SeqCst);
        debug_assert!(last != WRITING && last != UNUSED, "Last borrow state was invalid: {:?}", last);
    }
}

struct MutBorrowGuard<'a>(&'a AtomicUsize);

impl<'a> Drop for MutBorrowGuard<'a> {
    fn drop(&mut self) {
        let last = self.0.swap(UNUSED, Ordering::SeqCst);
        debug_assert!(last == WRITING);
    }
}
