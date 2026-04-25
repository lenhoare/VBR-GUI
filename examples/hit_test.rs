use crate::runtime::VbrRuntime;
use std::path::Path;

fn main() {
    let runtime = VbrRuntime::new(Path::new("vbr_ui.svg")).expect("runtime");
    println!("Hit table has {} widgets", runtime.hit_table.len());
    for w in &runtime.hit_table {
        println!("  id={} type={} action={:?} bounds={:?}", w.id, w.widget_type, w.action, w.bounds);
    }
}