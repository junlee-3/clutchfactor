//! Rule families (docs/spec/death-taxonomy.md §2). One module per family;
//! each implements `Detector` and is registered in `all()`.

use crate::Detector;

pub fn all() -> Vec<Box<dyn Detector>> {
    vec![]
}
