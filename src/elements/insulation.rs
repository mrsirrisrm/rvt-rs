//! Duct and pipe insulation and duct lining: the element each one covers and
//! its thickness.
//!
//! Revit models insulation and lining as elements of their own, hosted by a
//! duct, pipe, fitting or accessory. Their native records are
//! `RbsDuctInsulation`, `RbsPipeInsulation` and `RbsDuctLining`, all of
//! which derive from `RbsInsulationLiningBase` through `RbsReference`. The
//! base declares one field, `m_thickness`, the thickness in feet; the record
//! names its type in `m_idType`, and each of its two `Connector` objects
//! names the element it covers in `m_arrRefs`. None of these is a saved
//! parameter, so [`crate::elements::parameters::saved_parameters_lossy`] does
//! not carry them.
//!
//! Measured on the BIM4LCA housing's HVAC model
//! (`HVAC_NordicLCA_Housing_Concrete_BuildingPermit.rvt`, Revit 2024, VTT /
//! Granlund, CC BY-SA 4.0) against Revit's own IFC export of it: its 173
//! duct and 1,190 pipe insulation records are the export's 1,363
//! `IfcCovering` (`.INSULATION.`) `Tag`s exactly; each record's connectors
//! name one element, and it is the `Tag` of the element the export's
//! `IfcRelCoversBldgElements` relates the covering to, for all 1,363; and
//! `m_thickness` is the covering's insulation thickness property (`Eristeen
//! paksuus`, 18 to 119 mm) on all 1,306 coverings that carry one. That model
//! holds no lining: `RbsDuctLining` shares the base, so its thickness and
//! host are read the same way, which no model has measured yet.

use crate::ElementId;
use crate::parse_mode::{Decoded, Diagnostics, Warning};
use std::collections::BTreeSet;

/// Which kind of covering a record is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum CoveringKind {
    /// `RbsDuctInsulation`: insulation outside a duct, fitting or accessory.
    DuctInsulation,
    /// `RbsDuctLining`: acoustic or thermal lining inside a duct or fitting.
    DuctLining,
    /// `RbsPipeInsulation`: insulation outside a pipe, fitting or accessory.
    PipeInsulation,
}

/// The native record classes read, with the kind each is.
pub const COVERING_CLASSES: [(&str, CoveringKind); 3] = [
    ("RbsDuctInsulation", CoveringKind::DuctInsulation),
    ("RbsDuctLining", CoveringKind::DuctLining),
    ("RbsPipeInsulation", CoveringKind::PipeInsulation),
];

/// One insulation or lining element.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Covering {
    /// The covering's own ElementId.
    pub id: ElementId,
    pub kind: CoveringKind,
    /// The element it covers: the one element its connectors name, `None`
    /// when they name none or more than one.
    pub host_id: Option<ElementId>,
    /// Its type (`RbsDuctInsulationType`, `RbsDuctLiningType` or
    /// `RbsPipeInsulationType`).
    pub type_id: Option<ElementId>,
    /// `m_thickness`, in feet.
    pub thickness_feet: Option<f64>,
}

/// Every duct insulation, duct lining and pipe insulation element of `file`,
/// in record order. On a release the native records are not read for (before
/// Revit 2023), or when they cannot be read, the list is empty and the
/// diagnostics carry a `coverings-unread` warning.
pub fn coverings_lossy(file: &mut crate::RevitFile) -> Decoded<Vec<Covering>> {
    match read(file) {
        Ok(coverings) => Decoded::complete(coverings),
        Err(error) => {
            let mut diagnostics = Diagnostics::default();
            diagnostics.warn(Warning::new(
                "coverings-unread",
                format!("insulation and lining not read: {error:#}"),
            ));
            Decoded::partial(Vec::new(), diagnostics)
        }
    }
}

fn read(file: &mut crate::RevitFile) -> anyhow::Result<Vec<Covering>> {
    let options = crate::native_document::Options {
        selected_classes: COVERING_CLASSES
            .iter()
            .map(|(class, _)| (*class).to_string())
            .collect(),
        ..crate::native_document::Options::default()
    };
    let mut out = Vec::new();
    crate::native_document::extract_graphs(file, &options, |record| {
        let Some(kind) = record.class_name.as_deref().and_then(|name| {
            COVERING_CLASSES
                .iter()
                .find(|(class, _)| *class == name)
                .map(|(_, kind)| *kind)
        }) else {
            return Ok(());
        };
        let Some(id) = ElementId::from_raw(record.identity.element_id) else {
            return Ok(());
        };
        let graph = record.graph.as_ref();
        let root = graph.and_then(|graph| graph.objects.first());
        let field_id = |name: &str| {
            root.and_then(|root| root.fields[name]["m_id"]["m_id64"].as_u64())
                .and_then(ElementId::from_raw)
        };
        let hosts: BTreeSet<ElementId> = graph
            .into_iter()
            .flat_map(|graph| &graph.objects)
            .filter(|object| object.class_name == "Connector")
            .filter_map(|object| object.fields["m_arrRefs"].as_array())
            .flatten()
            .filter_map(|reference| reference["m_id"]["m_id"]["m_id64"].as_u64())
            .filter_map(ElementId::from_raw)
            .collect();
        out.push(Covering {
            id,
            kind,
            host_id: (hosts.len() == 1)
                .then(|| hosts.into_iter().next())
                .flatten(),
            type_id: field_id("m_idType"),
            thickness_feet: root.and_then(|root| root.fields["m_thickness"].as_f64()),
        });
        Ok(())
    })?;
    Ok(out)
}
