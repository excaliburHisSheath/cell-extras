use atomic_ref_cell::{AtomicRefCell, AtomicRef, AtomicRefMut};
use std::fmt::{self, Debug, Formatter};

pub struct AtomicInitCell<T>(AtomicRefCell<Option<T>>);

impl<T> AtomicInitCell<T> {
    pub const fn new() -> AtomicInitCell<T> {
        AtomicInitCell(AtomicRefCell::new(None))
    }

    pub fn init(&self, value: T) {
        let mut borrow = self.0.borrow_mut();
        assert!(borrow.is_none(), "`AtomicInitCell` is already initialized");
        *borrow = Some(value);
    }

    pub fn borrow(&self) -> AtomicRef<T> {
        let borrow = self.0.borrow();
        AtomicRef::map(borrow, |maybe| maybe.as_ref().expect("Cannot borrow uninitialized `AtomicInitCell`"))
    }

    pub fn borrow_mut(&self) -> AtomicRefMut<T> {
        let borrow = self.0.borrow_mut();
        AtomicRefMut::map(borrow, |maybe| maybe.as_mut().expect("Cannot borrow uninitialized `AtomicRefCell`"))
    }
}

impl<T> Debug for AtomicInitCell<T> where T: Debug {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        let inner = self.borrow();
        write!(formatter, "InitCell({:?})", &*inner)
    }
}
