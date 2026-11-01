//! MEP spaces: each placed space with its number, name and box.
//!
//! Autodesk's Snowdon Towers HVAC sample (Revit 2024) is an MEP model with
//! no rooms of its own: its 67 spaces are what its systems serve.
//! [`rvt::elements::spaces::spaces_lossy`] must read all 67, name 66 (one
//! has no parameter entries), and give boxes the model's air terminals
//! stand in: a diffuser hangs in its space's ceiling, so its location lies
//! within the box in plan and between its floor and top. 429 of the 509
//! do; of the rest, 52 stand where no space is in plan (floors the model
//! leaves unspaced) and 28 over a space but above its top or on a floor
//! between spaces.
//!
//! Runs against `RVT_REVIT2024_SAMPLES_DIR`
//! (`snowdon_towers_sample_hvac.rvt`). Skips when absent.

use rvt::RevitFile;
use rvt::elements::spaces::spaces_lossy;
use rvt::walker::{self, InstanceField};
use std::path::PathBuf;

#[test]
fn snowdon_hvac_spaces_hold_its_air_terminals() {
    let Some(dir) = std::env::var_os("RVT_REVIT2024_SAMPLES_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_REVIT2024_SAMPLES_DIR is not set");
        return;
    };
    let path = dir.join("snowdon_towers_sample_hvac.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    let mut rf = RevitFile::open(&path).expect("open");
    let decoded = spaces_lossy(&mut rf);
    assert!(
        decoded.diagnostics.warnings.is_empty(),
        "{:?}",
        decoded.diagnostics
    );
    let spaces = decoded.value;
    assert_eq!(spaces.len(), 67);
    assert_eq!(spaces.iter().filter(|s| s.name.is_some()).count(), 66);
    assert!(
        spaces
            .iter()
            .all(|s| (0..3).all(|k| s.bbox_feet[k + 3] > s.bbox_feet[k]))
    );

    let terminals: Vec<[f64; 3]> = walker::iter_elements(&mut rf)
        .expect("iter_elements")
        .filter(|e| e.class == "DuctTerminal")
        .filter_map(|e| {
            let f = |name: &str| {
                e.fields.iter().find_map(|(field, value)| match value {
                    InstanceField::Float { value, .. } if field == name => Some(*value),
                    _ => None,
                })
            };
            Some([f("m_locationX")?, f("m_locationY")?, f("m_locationZ")?])
        })
        .collect();
    let inside = terminals
        .iter()
        .filter(|p| {
            spaces
                .iter()
                .any(|s| (0..3).all(|k| s.bbox_feet[k] <= p[k] && p[k] <= s.bbox_feet[k + 3]))
        })
        .count();
    let in_plan = terminals
        .iter()
        .filter(|p| {
            spaces
                .iter()
                .any(|s| (0..2).all(|k| s.bbox_feet[k] <= p[k] && p[k] <= s.bbox_feet[k + 3]))
        })
        .count();
    assert!(!terminals.is_empty());
    assert_eq!((inside, in_plan, terminals.len()), (429, 457, 509));
}
