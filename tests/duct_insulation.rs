//! Insulation and lining: each covering, the element it covers and its
//! thickness are Revit's.
//!
//! Revit's IFC export writes each insulation element as an `IfcCovering`
//! whose `Tag` is its ElementId, relates it to the element it covers with an
//! `IfcRelCoversBldgElements`, and, on the BIM4LCA housing's HVAC model,
//! gives it its thickness as the property `Eristeen paksuus` (Finnish for
//! insulation thickness). [`rvt::elements::insulation::coverings_lossy`] must
//! read exactly the export's coverings, each with the `Tag` of the element
//! the export relates it to as its host, and with the export's thickness to
//! 0.001 mm wherever the export writes one.
//!
//! Runs against `RVT_NORDIC_LCA_DIR`:
//! `HVAC_NordicLCA_Housing_Concrete_BuildingPermit.rvt` and its `.ifc`, from
//! the same VTT / Granlund release (CC BY-SA 4.0) as the architectural
//! models CI reads. Skips what is absent.

use crate::is_external::{entities, list, split_args};
use rvt::RevitFile;
use rvt::elements::insulation::{CoveringKind, coverings_lossy};
use std::collections::BTreeMap;

const MODEL: &str = "HVAC_NordicLCA_Housing_Concrete_BuildingPermit";
const THICKNESS: &str = "Eristeen paksuus";

/// The export's coverings: `Tag -> (host Tag, thickness in metres)`.
fn exported(step: &str) -> BTreeMap<u32, (Option<u32>, Option<f64>)> {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let tag = |id: u64| {
        ents.get(&id)
            .and_then(|(_, args)| split_args(args).get(7).map(|t| unquote(t)))
            .and_then(|t| t.parse::<u32>().ok())
    };
    let metres = ents
        .values()
        .find(|(e, a)| e == "IFCSIUNIT" && a.contains(".LENGTHUNIT."))
        .map_or(
            1.0,
            |(_, a)| if a.contains(".MILLI.") { 0.001 } else { 1.0 },
        );
    let mut out = BTreeMap::new();
    let mut by_entity: BTreeMap<u64, u32> = BTreeMap::new();
    for (id, (entity, _)) in &ents {
        if entity == "IFCCOVERING" {
            if let Some(own) = tag(*id) {
                out.insert(own, (None, None));
                by_entity.insert(*id, own);
            }
        }
    }
    for (entity, args) in ents.values() {
        let f = split_args(args);
        match entity.as_str() {
            "IFCRELCOVERSBLDGELEMENTS" => {
                let host = f
                    .get(4)
                    .and_then(|h| list(&format!("({h})")).first().copied());
                for covering in f.get(5).map(|c| list(c)).unwrap_or_default() {
                    if let Some(own) = by_entity.get(&covering) {
                        out.get_mut(own).expect("covering").0 = host.and_then(tag);
                    }
                }
            }
            "IFCRELDEFINESBYPROPERTIES" => {
                let objects = f.get(4).map(|o| list(o)).unwrap_or_default();
                let Some((set_entity, set_args)) = f
                    .get(5)
                    .and_then(|s| list(&format!("({s})")).first().copied())
                    .and_then(|s| ents.get(&s))
                else {
                    continue;
                };
                if set_entity != "IFCPROPERTYSET" {
                    continue;
                }
                let props = split_args(set_args)
                    .get(4)
                    .map(|p| list(p))
                    .unwrap_or_default();
                for prop in props {
                    let Some((_, prop_args)) = ents.get(&prop) else {
                        continue;
                    };
                    let pf = split_args(prop_args);
                    if pf.first().map(|n| unquote(n)).as_deref() != Some(THICKNESS) {
                        continue;
                    }
                    let value = pf.get(2).and_then(|v| {
                        let inner = v.split_once('(')?.1.trim_end_matches(')');
                        inner.trim().parse::<f64>().ok()
                    });
                    for object in &objects {
                        if let Some(own) = by_entity.get(object) {
                            out.get_mut(own).expect("covering").1 = value.map(|v| v * metres);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[test]
fn nordic_hvac_coverings_hosts_and_thicknesses_are_revits() {
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
    let revit = exported(&step);
    let mut file = RevitFile::open(&rvt).expect("open model");
    let decoded = coverings_lossy(&mut file);
    assert!(
        decoded.diagnostics.warnings.is_empty(),
        "{:?}",
        decoded.diagnostics
    );
    let ours = decoded.value;

    let mut kinds: BTreeMap<CoveringKind, usize> = BTreeMap::new();
    for covering in &ours {
        *kinds.entry(covering.kind).or_default() += 1;
    }
    assert_eq!(
        kinds,
        BTreeMap::from([
            (CoveringKind::DuctInsulation, 173),
            (CoveringKind::PipeInsulation, 1_190)
        ])
    );
    let tags: Vec<u32> = ours.iter().filter_map(|c| c.id.as_u32()).collect();
    assert_eq!(
        tags.iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        revit.keys().copied().collect(),
        "the coverings are not the export's"
    );
    let mut thicknesses = 0;
    for covering in &ours {
        let own = covering.id.as_u32().expect("id");
        let (host, thickness) = revit[&own];
        assert_eq!(
            covering.host_id.and_then(|h| h.as_u32()),
            host,
            "host of {own}"
        );
        if let Some(metres) = thickness {
            let ours = covering.thickness_feet.expect("thickness") * 0.3048;
            assert!(
                (ours - metres).abs() < 1e-6,
                "{own}: {ours} m, Revit {metres} m"
            );
            thicknesses += 1;
        }
    }
    assert_eq!(thicknesses, 1_306);
}
