extern crate cell_extras;

use cell_extras::InitCell;

fn main() {
    thread_local! {
        static STATIC_DATA: InitCell<String> = InitCell::new();
    }

    // Normal usage of `InitCell<T>`.
    let cell = InitCell::<usize>::new();
    println!("cell before init: {:?}", cell.get());
    cell.init(10);
    println!("cell after init: {:?}", cell.get());

    // Use `InitCell<T>` in a static.
    STATIC_DATA.with(|static_data| static_data.init("foo".into()));
    // STATIC_DATA.with(|static_data| println!("STATIC_DATA: {:?}", static_data));

    let empty_cell = InitCell::<f32>::new();
    println!("empty cell: {:?}", &*empty_cell);
}
