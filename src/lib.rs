//! More shareable, mutable containers.
//!
//! [`Cell<T>`][cell] and [`RefCell<T>`][refcell] are a good start, but they
//! don't represent all the forms of interior mutability that developers might
//! need. This crate provides a more comprehensive suite of cell types to cover
//! the cases not solved by the standard library. For more information on cells
//! and interior mutability in general please see [the std::cell module's
//! documentation][std::cell].
//!
//! # When Should You Use Which Cell?
//!
//! Here's a quick breakdown of each cell type and when it would be helpful:
//!
//! ### Use a `CloneCell<T>` when:
//!
//! - You want a [`Cell<T>`][cell] that can support [`Clone`][clone] and [`Drop`][drop] types.
//! - Are willing to pay a small amount of runtime overhead (the same as [`RefCell<T>`][refcell])
//!   over [`Cell<T>`][cell] for that functionality.
//!
//! ### Use an `AtomicCell<T>` when:
//!
//! - You want to use a thread-safe [`Cell<T>`][cell].
//! - You want to use a thread-safe [`CloneCell<T>`][clonecell].
//!
//! ### Use an `InitCell<T>` when:
//!
//! - You have a non-optional value that can't be initialized when the object
//!   is created.
//! - You want to ensure that the value is never accessed in an unitialized
//!   state.
//! - You don't need the dynamically checked borrow rules of a
//!   [`RefCell<T>`][refcell]
//! - You have a thread-local static that needs to be lazily initialzed at
//!   startup, but you want to access it without checking if it's initialized.
//!
//! ### Use an `AtomicInitCell<T>` when:
//!
//! - You have a static that needs to be lazily initialized, but you want to be
//!   able to access the data without checking if it's initialized.
//! - You want to use a thread-safe `InitCell<T>`.
//!
//! ### Use an `AtomicRefCell<T>` when:
//!
//! - You want to use a [`RefCell<T>`][refcell] but need to to be thread-safe.
//! - You want an [`RwLock<T>`][rwlock] that panics instead of blocking.
//!
//! [std::cell]: https://doc.rust-lang.org/std/cell/index.html
//! [cell]: https://doc.rust-lang.org/std/cell/struct.Cell.html
//! [refcell]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
//! [rwlock]: https://doc.rust-lang.org/std/sync/struct.RwLock.html
//! [clone]: https://doc.rust-lang.org/std/clone/trait.Clone.html
//! [drop]: https://doc.rust-lang.org/std/ops/trait.Drop.html

pub use atomic_ref_cell::AtomicRefCell;
pub use init_cell::InitCell;

pub mod atomic_ref_cell;
pub mod init_cell;
