//! MEP spaces: each space's number, name, level and bounding box.
//!
//! An MEP model holds the spaces its systems serve, not the architect's
//! rooms, which live in a linked model. Revit stores a space as a partition
//! element record of `OST_MEPSpaces`, the shape rooms have: the #211
//! instance rule selects the placed spaces, the record box gives their
//! extent, and the room parameter entries (RE-117) their number and name.
//!
//! Measured on Autodesk's Snowdon Towers HVAC sample (Revit 2024): 67
//! spaces, 66 of them named. Revit's IFC export of it is not to hand, so
//! the set is not yet held to one; its air terminals stand inside the
//! boxes (`tests/mep_spaces.rs`).

use crate::ElementId;
use crate::parse_mode::{Decoded, Diagnostics, Warning};
use crate::walker::InstanceField;
use std::collections::BTreeSet;

/// Autodesk `BuiltInCategory.OST_MEPSpaces`.
pub const OST_MEP_SPACES: i64 = -2_003_600;

/// One MEP space.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Space {
    pub id: ElementId,
    /// Its number and name, where its parameter entries give them.
    pub number: Option<String>,
    pub name: Option<String>,
    /// The Level it is on, when its record names exactly one.
    pub level_id: Option<ElementId>,
    /// Its bounding box in model coordinates, feet: minimum x, y, z, then
    /// maximum x, y, z.
    pub bbox_feet: [f64; 6],
}

/// Every placed MEP space of `file`. On a release whose element records
/// are not read (before Revit 2024), or when they cannot be read, the list
/// is empty and the diagnostics carry a `spaces-unread` warning.
pub fn spaces_lossy(file: &mut crate::RevitFile) -> Decoded<Vec<Space>> {
    let unread = |why: String| {
        let mut diagnostics = Diagnostics::default();
        diagnostics.warn(Warning::new(
            "spaces-unread",
            format!("MEP spaces not read: {why}"),
        ));
        Decoded::partial(Vec::new(), diagnostics)
    };
    let release = file.layout_release().unwrap_or(0);
    if !crate::partition_element_records::supports_revit_version(release) {
        return unread(format!("Revit {release} element records are not read"));
    }
    match read(file, release) {
        Ok(spaces) => Decoded::complete(spaces),
        Err(error) => unread(format!("{error:#}")),
    }
}

fn read(file: &mut crate::RevitFile, release: u32) -> anyhow::Result<Vec<Space>> {
    use crate::partition_schema_mvp as mvp;
    let levels = mvp::level_element_ids(file, release)?;
    let records = mvp::instances_from_partition_category_records(
        file,
        release,
        OST_MEP_SPACES,
        "Space",
        &levels,
    )?;
    let ids: BTreeSet<u32> = records.iter().filter_map(|r| r.id).collect();
    let entries =
        crate::partition_room_parameters::scan_room_parameter_entries(file, release, &ids);
    Ok(records
        .iter()
        .filter_map(|record| {
            let raw = record.id?;
            let float = |name: &str| {
                record.fields.iter().find_map(|(field, value)| match value {
                    InstanceField::Float { value, .. } if field == name => Some(*value),
                    _ => None,
                })
            };
            let (x, y, z) = (
                float("m_locationX")?,
                float("m_locationY")?,
                float("m_locationZ")?,
            );
            let (w, d, h) = (
                float("m_bboxWidth")?,
                float("m_bboxDepth")?,
                float("m_bboxHeight")?,
            );
            let level_id = record.fields.iter().find_map(|(field, value)| match value {
                InstanceField::ElementId { id, .. }
                    if field == crate::element_record_level_refs::LEVEL_REFERENCE_FIELD =>
                {
                    Some(ElementId::from(*id))
                }
                _ => None,
            });
            let entry = entries.get(&raw);
            Some(Space {
                id: ElementId::from(raw),
                number: entry.map(|(number, _)| number.clone()),
                name: entry.map(|(_, name)| name.clone()),
                level_id,
                bbox_feet: [x - w / 2.0, y - d / 2.0, z, x + w / 2.0, y + d / 2.0, z + h],
            })
        })
        .collect())
}
