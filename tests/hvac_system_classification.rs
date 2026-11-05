//! An HVAC system's classification, from its type: supply, return or
//! exhaust air, as its name says.
//!
//! Revit names a new system after its type (`Mechanical Supply Air 3`), and
//! MagiCAD names its systems in Finnish: tuloilma (supply) and ulkoilma or
//! raitisilma (outdoor air, a supply's intake), poistoilma (extract), and
//! jäteilma (exhaust, `JI`); `S1` is a transfer air system MagiCAD types as
//! a return. Every duct element a system groups must carry its
//! classification as its name gives it: on Autodesk's Snowdon Towers HVAC
//! sample (Revit 2024) and the BIM4LCA housing's HVAC model (Revit 2024,
//! MagiCAD).
//!
//! Runs against `RVT_REVIT2024_SAMPLES_DIR` (`snowdon_towers_sample_hvac.rvt`)
//! and `RVT_NORDIC_LCA_DIR`. Skips what is absent.

use rvt::RevitFile;
use rvt::walker::{self, InstanceField};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Every HVAC system the model's duct elements belong to: id -> (name,
/// classification).
fn systems(path: &Path) -> BTreeMap<u32, (String, Option<i64>)> {
    let mut rf = RevitFile::open(path).expect("open");
    let mut out: BTreeMap<u32, (String, Option<i64>)> = BTreeMap::new();
    let ducts = walker::iter_elements(&mut rf)
        .expect("iter_elements")
        .filter(|e| {
            matches!(
                e.class.as_str(),
                "Duct" | "DuctFitting" | "DuctAccessory" | "DuctTerminal"
            )
        });
    for element in ducts {
        for (field, value) in &element.fields {
            if let (Some(id), InstanceField::String(name)) =
                (field.strip_prefix("m_mep_system_"), value)
            {
                out.entry(id.parse().expect("id"))
                    .or_insert((name.clone(), None))
                    .0 = name.clone();
            }
            if let (Some(id), InstanceField::Integer { value, .. }) =
                (field.strip_prefix("m_mep_classification_"), value)
            {
                out.entry(id.parse().expect("id"))
                    .or_insert((String::new(), None))
                    .1 = Some(*value);
            }
        }
    }
    out
}

fn check(path: &Path, expected: impl Fn(&str) -> i64) -> usize {
    let systems = systems(path);
    for (id, (name, classification)) in &systems {
        assert_eq!(*classification, Some(expected(name)), "{id} {name}");
    }
    systems.len()
}

#[test]
fn snowdon_hvac_systems_classified_as_named() {
    let Some(dir) = std::env::var_os("RVT_REVIT2024_SAMPLES_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_REVIT2024_SAMPLES_DIR is not set");
        return;
    };
    let path = dir.join("snowdon_towers_sample_hvac.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    let systems = check(&path, |name| {
        [("Supply Air", 1), ("Return Air", 2), ("Exhaust Air", 3)]
            .iter()
            .find(|(word, _)| name.contains(word))
            .map(|&(_, c)| c)
            .unwrap_or_else(|| panic!("{name}"))
    });
    assert!(systems > 0);
}

#[test]
fn nordic_hvac_systems_classified_as_named() {
    let Some(dir) = crate::element_parameters::nordic_dir() else {
        return;
    };
    let path = dir.join("HVAC_NordicLCA_Housing_Concrete_BuildingPermit.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    let systems = check(&path, |name| match name {
        _ if name.starts_with("Tuloilma")
            || name.starts_with("Ulkoilma")
            || name.starts_with("Raitisilma") =>
        {
            1
        }
        _ if name.starts_with("Poistoilma") || name == "S1" => 2,
        _ if name.starts_with("JI") || name.starts_with("Jäteilma") => 3,
        _ => panic!("{name}"),
    });
    assert!(systems > 0);
}
