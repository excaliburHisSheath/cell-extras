use std::cell::UnsafeCell;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut};

/// Cell that allows a value to be lazily initialized once.
///
/// This provides a lightweight abstraction over lazily initializing a value. The cell can be
/// created without an initial value and then later be initialized through a shared reference.
/// Once the cell has been initialized it can be treated like a `&T` or `&mut T`. In order to
/// maintain safety guarantees, the cell can only be initialized once. Subsequent attempts to
/// initialize it will result in a panic. Similarly, attempting to derefrence an uninitialized
/// cell will result in a panic.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use cell_extras::init_cell::InitCell;
///
/// // Create a new `InitCell<String>`.
/// let mut cell = InitCell::<String>::new();
///
/// // The cell starts out uninitialized.
/// assert_eq!(None, cell.get());
///
/// // We can initialize the cell with a string at some later point.
/// cell.init("foo".into());
///
/// // Once initialized we can mutate the contents of the cell without checking its initialized
/// // state (assuming we have mutable access to it).
/// cell.push_str("bar");
/// assert_eq!("foobar", &*cell);
/// ```
///
/// Lazily initialize a thread-local static:
///
/// ```
/// use cell_extras::init_cell::InitCell;
/// use std::thread;
///
/// thread_local! {
///     // The actual initial value won't be known until runtime, maybe it's a command line argument.
///     static LOCAL: InitCell<String> = InitCell::new();
/// }
///
/// for _ in 0..8 {
///     thread::spawn(|| {
///         // Initialize super important value when the thread starts.
///         LOCAL.with(|local| local.init("foobar".into()));
///
///         // ...
///
///         // Use the thread-local as if it were just a `String` without worrying about
///         // initialization status.
///         let local = LOCAL.with(|local| (&**local).clone());
///     });
/// }
/// ```
pub struct InitCell<T>(UnsafeCell<Option<T>>);

impl<T> InitCell<T> {
    /// Create a new, uninitialized `InitCell<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::init_cell::InitCell;
    ///
    /// let cell = InitCell::<usize>::new();
    /// ```
    #[inline]
    pub const fn new() -> InitCell<T> {
        InitCell(UnsafeCell::new(None))
    }

    /// Initialize the cell with the specified value.
    ///
    /// # Panics
    ///
    /// - If the cell has already been initialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::init_cell::InitCell;
    ///
    /// let cell = InitCell::<usize>::new();
    /// assert_eq!(None, cell.get());
    ///
    /// cell.init(7);
    /// assert_eq!(7, *cell);
    /// ```
    #[inline]
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
    ///
    /// This provides a way to safely get the value of a possibly uninitialized `InitCell<T>` without
    /// panicking. Returns `Some` with a reference to the data if `init()` has been called on
    /// the cell, otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::init_cell::InitCell;
    ///
    /// let cell = InitCell::<usize>::new();
    /// assert_eq!(None, cell.get());
    ///
    /// cell.init(7);
    /// assert_eq!(Some(&7), cell.get());
    /// ```
    pub fn get(&self) -> Option<&T> {
        let data = unsafe { &*self.0.get() };
        data.as_ref()
    }

    /// Get a mutable reference to the data if the cell has been initialized.
    ///
    /// This provides a way to safely get and mutate the value of a possibly uninitialized
    /// `InitCell<T>` without panicking. Returns `Some` with a reference to the data if `init()`
    /// has been called on the cell, otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cell_extras::init_cell::InitCell;
    ///
    /// let mut cell = InitCell::<usize>::new();
    /// assert_eq!(None, cell.get());
    ///
    /// cell.init(7);
    ///
    /// if let Some(data) = cell.get_mut() {
    ///     *data = 21;
    /// }
    ///
    /// assert_eq!(21, *cell);
    /// ```
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

impl<T> Debug for InitCell<T> where T: Debug {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "InitCell({:?})", &**self)
    }
}

unsafe impl<T> Send for InitCell<T> where T: Send {}

#[cfg(test)]
mod tests {
    use init_cell::InitCell;

    #[test]
    fn init() {
        let cell = InitCell::<usize>::new();
        assert_eq!(None, cell.get());

        cell.init(10);
        assert_eq!(Some(&10), cell.get());
    }
}
