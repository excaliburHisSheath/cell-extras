use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

/// Cell that allows a value to be lazily initialized once.
///
///
pub struct InitCell<T>(UnsafeCell<Option<T>>);

impl<T> InitCell<T> {
    /// Create a new, uninitialized `InitCell<T>`.
    pub const fn new() -> InitCell<T> {
        InitCell(UnsafeCell::new(None))
    }

    /// Initialize the cell with the specified value.
    ///
    /// # Panics
    ///
    /// - If the cell has already been initialized.
    pub fn init(&self, value: T) {
        // It's safe to take a mutable reference to the data in here because
        // reasons:
        //
        // - If the data hasn't been initialized, then attempts to take a
        //   reference to the data would have panicked, so we can assume there
        //   are no references to the data.
        // - If the data has been initialized, then we're about to panic anyway
        //   so whatever.
        let data = unsafe { &mut *self.0.get() };

        assert!(data.is_none(), "Cannot initialize InitCell more than once");

        *data = Some(value);
    }

    /// Get a reference to the data if the cell has been initialized.
    pub fn get(&self) -> Option<&T> {
        let data = unsafe { &*self.0.get() };
        data.as_ref()
    }

    /// Get a mutable reference to the data if the cell has been initialized.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        let data = unsafe { &mut *self.0.get() };
        data.as_mut()
    }
}

impl<T> Deref for InitCell<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.get().expect("Cannot deref `InitCell` if it hasn't been initialized")
    }
}

impl<T> DerefMut for InitCell<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.get_mut().expect("Cannot deref `InitCell` if it hasn't been initialized")
    }
}

unsafe impl<T: Send> Send for InitCell<T> {}
