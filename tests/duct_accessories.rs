//! Duct accessories: the dampers, silencers and filters in a duct run are
//! elements, and the network runs through them.
//!
//! On the BIM4LCA housing's HVAC model, Revit's IFC export writes its duct
//! fittings and accessories as `IfcDuctFitting`, `IfcDamper`,
//! `IfcDuctSilencer` and `IfcFilter`, by each family's own IFC class. The
//! walk must emit exactly those `Tag`s as `DuctFitting` and
//! `DuctAccessory` elements, and each accessory must name the elements its
//! connectors join: without them a branch ends at its first damper.
//!
//! Runs against `RVT_NORDIC_LCA_DIR` (the `.rvt` and its `.ifc`). Skips
//! what is absent.

use crate::is_external::{entities, split_args};
use rvt::RevitFile;
use rvt::walker::{self, InstanceField};
use std::collections::BTreeSet;

const MODEL: &str = "HVAC_NordicLCA_Housing_Concrete_BuildingPermit";

#[test]
fn nordic_hvac_accessories_are_the_exports() {
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
    let exported: BTreeSet<u32> = entities(&step)
        .values()
        .filter(|(entity, _)| {
            matches!(
                entity.as_str(),
                "IFCDUCTFITTING" | "IFCDAMPER" | "IFCDUCTSILENCER" | "IFCFILTER"
            )
        })
        .filter_map(|(_, args)| {
            split_args(args)
                .get(7)
                .and_then(|t| t.trim().trim_matches('\'').parse().ok())
        })
        .collect();

    let mut file = RevitFile::open(&rvt).expect("open model");
    let elements: Vec<_> = walker::iter_elements(&mut file)
        .expect("iter_elements")
        .collect();
    let of = |class: &str| -> BTreeSet<u32> {
        elements
            .iter()
            .filter(|e| e.class == class)
            .filter_map(|e| e.id)
            .collect()
    };
    let (fittings, accessories) = (of("DuctFitting"), of("DuctAccessory"));
    assert_eq!(accessories.len(), 162);
    assert!(fittings.is_disjoint(&accessories));
    let ours: BTreeSet<u32> = fittings.union(&accessories).copied().collect();
    assert_eq!(ours, exported);

    let unjoined: Vec<_> = elements
        .iter()
        .filter(|e| e.class == "DuctAccessory")
        .filter(|e| {
            !e.fields.iter().any(|(name, value)| {
                name.starts_with("m_connector_")
                    && name.ends_with("_element")
                    && matches!(value, InstanceField::ElementId { .. })
            })
        })
        .filter_map(|e| e.id)
        .collect();
    assert!(
        unjoined.is_empty(),
        "accessories joining nothing: {unjoined:?}"
    );
}
