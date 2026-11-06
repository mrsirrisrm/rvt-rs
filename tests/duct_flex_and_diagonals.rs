//! Snowdon Towers' HVAC sample (Revit 2024): its flexible ducts, each in its
//! system with a box, and its straight ducts running across the plan, whose
//! ends the connector entries give even where a second entry lies 0.001 ft
//! off one of them.
//!
//! Runs against `RVT_REVIT2024_SAMPLES_DIR` (`snowdon_towers_sample_hvac.rvt`).
//! Skips what is absent.

use rvt::RevitFile;
use rvt::walker;
use std::path::PathBuf;

fn sample() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_REVIT2024_SAMPLES_DIR").map(PathBuf::from);
    let Some(dir) = dir else {
        eprintln!("skipping: RVT_REVIT2024_SAMPLES_DIR is not set");
        return None;
    };
    let path = dir.join("snowdon_towers_sample_hvac.rvt");
    path.exists().then_some(path)
}

#[test]
fn snowdon_flex_ducts_and_diagonal_ducts() {
    let Some(path) = sample() else { return };
    let mut rf = RevitFile::open(&path).expect("open");
    let has = |fields: &[(String, walker::InstanceField)], prefix: &str| {
        fields.iter().any(|(n, _)| n.starts_with(prefix))
    };
    let (mut flex, mut ducts, mut without_ends) = (0, 0, Vec::new());
    for e in walker::iter_elements(&mut rf).expect("iter_elements") {
        match e.class.as_str() {
            "FlexDuct" => {
                flex += 1;
                assert!(has(&e.fields, "m_mep_system_"), "{:?}", e.id);
                assert!(has(&e.fields, "m_bboxWidth"), "{:?}", e.id);
            }
            "Duct" => {
                ducts += 1;
                if !has(&e.fields, "m_duct_start_x") {
                    without_ends.push(e.id);
                }
            }
            _ => {}
        }
    }
    assert_eq!(flex, 31);
    assert_eq!(ducts, 1053);
    // Every straight has its ends.
    assert_eq!(without_ends, Vec::<Option<u32>>::new());
}
