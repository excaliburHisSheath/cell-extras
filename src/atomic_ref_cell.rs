use std::cell::UnsafeCell;
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
/// # Examples
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

impl<T> AtomicRefCell<T> where T: Send + Sync {
    pub fn new(value: T) -> AtomicRefCell<T> {
        AtomicRefCell {
            borrow: AtomicUsize::new(UNUSED),
            value: UnsafeCell::new(value),
        }
    }

    pub fn into_inner(self) -> T {
        debug_assert!(self.borrow.load(Ordering::SeqCst) == UNUSED);
        unsafe { self.value.into_inner() }
    }

    pub fn borrow(&self) -> AtomicRef<T> {
        self.try_borrow().expect("Already mutably borrowed")
    }

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
                return Some(AtomicRef(self))
            }
        }
    }

    pub fn try_borrow_mut(&self) -> Option<AtomicRefMut<T>> {
        if self.borrow.compare_and_swap(UNUSED, WRITING, Ordering::SeqCst) == UNUSED {
            return Some(AtomicRefMut(self))
        } else {
            return None
        }
    }

    pub fn borrow_mut(&self) -> AtomicRefMut<T> {
        self.try_borrow_mut().expect("Already immutably borrowed")
    }
}

unsafe impl<T> Send for AtomicRefCell<T> where T: Send {}
unsafe impl<T> Sync for AtomicRefCell<T> where T: Sync {}

pub struct AtomicRef<'a, T: 'a>(&'a AtomicRefCell<T>);

impl<'a, T: 'a> Drop for AtomicRef<'a, T> {
    fn drop(&mut self) {
        let last = self.0.borrow.fetch_sub(1, Ordering::SeqCst);
        debug_assert!(last != WRITING && last != UNUSED, "Last borrow state was invalid: {:?}", last);
    }
}

impl<'a, T: 'a> Deref for AtomicRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

pub struct AtomicRefMut<'a, T: 'a>(&'a AtomicRefCell<T>);

impl<'a, T: 'a> Drop for AtomicRefMut<'a, T> {
    fn drop(&mut self) {
        let last = self.0.borrow.swap(UNUSED, Ordering::SeqCst);
        debug_assert!(last == WRITING);
    }
}

impl<'a, T: 'a> Deref for AtomicRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<'a, T: 'a> DerefMut for AtomicRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}
