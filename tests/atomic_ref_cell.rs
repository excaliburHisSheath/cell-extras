extern crate cell_extras;

use cell_extras::AtomicRefCell;

#[test]
fn non_send_type() {
    AtomicRefCell::new(::std::ptr::null::<usize>());
}

#[test]
fn non_sync_type() {
    AtomicRefCell::new(::std::ptr::null::<usize>());
}
