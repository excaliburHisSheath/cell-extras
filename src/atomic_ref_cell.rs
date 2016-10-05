use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};

const UNUSED: usize = 0;
const WRITING: usize = !0;

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

    pub fn try_borrow(&self) -> Option<AtomicRef<T>> {
        loop {
            let borrow = self.borrow.load(Ordering::SeqCst);
            if borrow == WRITING { return None }

            if self.borrow.compare_and_swap(borrow, borrow + 1, Ordering::SeqCst) == borrow {
                return Some(AtomicRef(self))
            }
        }
    }

    pub fn borrow_mut(&self) -> AtomicRefMut<T> {
        unimplemented!();
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
