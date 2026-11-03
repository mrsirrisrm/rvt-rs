//! RE-134 on Revit 2024: a duct's curve fields read as on 2025.
//!
//! The BIM4LCA housing's HVAC model (Revit 2024, MagiCAD) exports each duct
//! with MagiCAD's own properties: `Pituus` (its length), and `Leveys` and
//! `Korkeus` (a rectangular duct's width and height; 0 on a round one), in
//! millimetres. Wherever the walk gives a duct a length, it must be the
//! export's `Pituus` to 1 mm, and a rectangular duct's width and height its
//! `Leveys` and `Korkeus`. On this model all 701 ducts' curve fields are
//! found (each in the duct's own data object) and all 701 have a length:
//! along a model axis from the box, and off the axes, for a round duct, from
//! the cylinder its connector entries prove. 6 are rectangular. Round ducts'
//! diameters agree with Revit's extrusions in the export too (to 2 mm on the
//! 697 the duct-inverse pipeline compares), but no property here carries
//! them.
//!
//! Runs against `RVT_NORDIC_LCA_DIR` (the `.rvt` and its `.ifc`). Skips
//! what is absent.

use crate::is_external::{entities, list, split_args};
use rvt::RevitFile;
use rvt::walker::{self, InstanceField};
use std::collections::BTreeMap;

const MODEL: &str = "HVAC_NordicLCA_Housing_Concrete_BuildingPermit";
const FOOT_MM: f64 = 304.8;

/// The export's ducts: `Tag -> property -> millimetres`, for the properties
/// named above.
fn exported(step: &str) -> BTreeMap<u32, BTreeMap<String, f64>> {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let tags: BTreeMap<u64, u32> = ents
        .iter()
        .filter(|(_, (entity, _))| entity == "IFCDUCTSEGMENT")
        .filter_map(|(id, (_, args))| Some((*id, unquote(split_args(args).get(7)?).parse().ok()?)))
        .collect();
    let mut out: BTreeMap<u32, BTreeMap<String, f64>> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(args);
        let objects: Vec<u32> = f
            .get(4)
            .map(|o| list(o))
            .unwrap_or_default()
            .iter()
            .filter_map(|o| tags.get(o).copied())
            .collect();
        if objects.is_empty() {
            continue;
        }
        let Some((_, set)) = f
            .get(5)
            .and_then(|s| list(&format!("({s})")).first().copied())
            .and_then(|s| ents.get(&s))
            .filter(|(entity, _)| entity == "IFCPROPERTYSET")
        else {
            continue;
        };
        for property in split_args(set).get(4).map(|p| list(p)).unwrap_or_default() {
            let Some((entity, args)) = ents.get(&property) else {
                continue;
            };
            if entity != "IFCPROPERTYSINGLEVALUE" {
                continue;
            }
            let p = split_args(args);
            let name = unquote(&p[0]);
            if !matches!(name.as_str(), "Pituus" | "Leveys" | "Korkeus") {
                continue;
            }
            let value = p.get(2).and_then(|v| {
                let inner = v.split_once('(')?.1.trim_end_matches(')');
                inner.trim().parse::<f64>().ok()
            });
            if let Some(value) = value {
                for object in &objects {
                    out.entry(*object).or_default().insert(name.clone(), value);
                }
            }
        }
    }
    out
}

#[test]
fn nordic_hvac_duct_lengths_and_sections_are_magicads() {
    let Some(dir) = crate::element_parameters::nordic_dir() else {
        return;
    };
    let (rvt, ifc) = (
        dir.join(format!("{MODEL}.rvt")),
        dir.join(format!("{MODEL}.ifc")),
    );
    if !rvt.exists() || !ifc.exists() {
        eprintln!("skipping {MODEL}: model or export absent");
        return;
    }
    let step = String::from_utf8_lossy(&std::fs::read(&ifc).expect("read export")).into_owned();
    let magicad = exported(&step);
    let mut file = RevitFile::open(&rvt).expect("open model");
    let ducts: Vec<_> = walker::iter_elements(&mut file)
        .expect("iter_elements")
        .filter(|e| e.class == "Duct")
        .collect();
    assert_eq!(ducts.len(), 701);
    let (mut sized, mut lengths, mut sections) = (0, 0, 0);
    for duct in &ducts {
        let id = duct.id.expect("duct id");
        let field = |name: &str| {
            duct.fields.iter().find_map(|(field, value)| match value {
                InstanceField::Float { value, .. } if field == name => Some(*value * FOOT_MM),
                _ => None,
            })
        };
        let props = &magicad[&id];
        if let Some(length) = field("m_duct_length") {
            assert!(
                (length - props["Pituus"]).abs() < 1.0,
                "{id}: length {length} mm, Pituus {}",
                props["Pituus"]
            );
            lengths += 1;
        }
        let (Some(width), Some(height)) = (field("m_curve_width"), field("m_curve_height")) else {
            continue;
        };
        sized += 1;
        if props["Leveys"] > 0.0 {
            assert!(
                (width - props["Leveys"]).abs() < 1.0,
                "{id}: width {width} mm, Leveys {}",
                props["Leveys"]
            );
            assert!(
                (height - props["Korkeus"]).abs() < 1.0,
                "{id}: height {height} mm, Korkeus {}",
                props["Korkeus"]
            );
            sections += 1;
        }
    }
    assert_eq!((sized, lengths, sections), (701, 701, 6));
}
