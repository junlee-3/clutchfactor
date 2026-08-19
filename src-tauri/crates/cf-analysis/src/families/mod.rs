//! Rule families (docs/spec/death-taxonomy.md §2). One module per family;
//! each implements `Detector` and is registered in `all()`.

use crate::Detector;

pub mod h16;
pub mod h2;
pub mod h3;
pub mod h4;

pub fn all() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(h2::H2TradeSpacing),
        Box::new(h3::H3UtilityVulnerability),
        Box::new(h4::H4Exposure),
        Box::new(h16::H16UtilityDamage),
    ]
}
