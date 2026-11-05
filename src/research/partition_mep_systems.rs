//! MEP systems and their members (RE-162, #528).
//!
//! An MEP system is an element of its own, a data object (RE-153) of class
//! `RbsHvacSystem`, `RbsPipingSystem` or `RbsElectricalSystem`. An HVAC or
//! piping system's name is the first string of printable ASCII in its
//! payload; an electrical system's is its circuit number
//! (RE-162 §3). Its members are the elements whose own data objects
//! hold the system's ElementId. On the RE1 Mechanical, Plumbing and
//! Electrical models (Revit 2025, MIT), against Revit's own IFC: every one of
//! Revit's 26 `IfcSystem`s is one such object of the same name, and every
//! element Revit groups in it holds its id.
//!
//! An HVAC system's object also holds the id of its type, an
//! `RbsHvacSystemType` data object, and the type holds the system's
//! classification: Revit's `MEPSystemClassification`, 1 supply air, 2 return
//! air, 3 exhaust air (see [`type_classification`]). On Autodesk's Snowdon
//! Towers HVAC sample (Revit 2024), the RME advanced sample (Revit 2025) and
//! the BIM4LCA housing's MagiCAD model (Revit 2024), every type's reads as
//! its systems' names say (`tests/hvac_system_classification.rs`).
// Tracker: B33.

use crate::le_bytes;
use crate::partition_room_parameters::{DATA_OBJECT_HEADER, verified_data_object};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// The schema classes of MEP systems.
pub const SYSTEM_CLASSES: [&str; 3] = ["RbsHvacSystem", "RbsPipingSystem", "RbsElectricalSystem"];
/// The schema class of an HVAC system's type.
pub const HVAC_SYSTEM_TYPE_CLASS: &str = "RbsHvacSystemType";
/// The schema class of the object a system type's overrides point to.
const SYSTEM_OVERRIDES_CLASS: &str = "SystemOverrides";
/// Revit's `MEPSystemClassification` for supply, return and exhaust air,
/// the values read here.
pub const SUPPLY_AIR: u32 = 1;
pub const RETURN_AIR: u32 = 2;
pub const EXHAUST_AIR: u32 = 3;
/// The largest classification taken as one: Revit's enumeration is small.
const MAX_CLASSIFICATION: u32 = 64;
/// Longest system name read, in UTF-16 units.
const MAX_NAME_UNITS: usize = 256;

/// One MEP system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MepSystem {
    /// Its ElementId.
    pub id: u32,
    /// Its schema class, one of [`SYSTEM_CLASSES`].
    pub class: String,
    /// Its name, where its object holds one.
    pub name: Option<String>,
    /// The elements of `elements` (see [`scan_mep_systems`]) whose own data
    /// objects hold its id.
    pub members: BTreeSet<u32>,
    /// An HVAC system's classification, from its type: [`SUPPLY_AIR`],
    /// [`RETURN_AIR`], [`EXHAUST_AIR`] or another of Revit's values.
    pub classification: Option<u32>,
}

/// Data objects are read on Revit 2024 and later (RE-153).
pub fn supports_revit_version(revit_version: u32) -> bool {
    revit_version >= 2024
}

/// The `u32 n · UTF-16 × n` string at `q` of `bytes`, `n` at most
/// [`MAX_NAME_UNITS`].
fn string_at(bytes: &[u8], q: usize) -> Option<Vec<u16>> {
    let n = le_bytes::u32_at(bytes, q)? as usize;
    if n > MAX_NAME_UNITS {
        return None;
    }
    Some(le_bytes::u16_units(bytes.get(q + 4..q + 4 + 2 * n)?).collect())
}

/// The first non-empty `u32 n · UTF-16 × n` string in `payload` of printable
/// ASCII: an HVAC or piping system's name. Shorter strings of other units
/// come first, a `1` followed by half an ElementId reading as one.
fn first_string(payload: &[u8]) -> Option<String> {
    (0..payload.len()).find_map(|q| {
        let units = string_at(payload, q)?;
        (!units.is_empty() && units.iter().all(|u| (0x20..0x7f).contains(u)))
            .then(|| String::from_utf16_lossy(&units))
    })
}

/// An HVAC system type's classification, from its payload: after its
/// abbreviation (`m_abbreviation`, `u32 n · UTF-16 × n`) comes its overrides
/// pointer, `ffffffff`; where the type has overrides, the `SystemOverrides`
/// object's class tag (`overrides_tag`) and a `u32` (1 or `ffffffff`)
/// follow; then the classification as a `u32`. The first such run in the
/// payload is taken.
///
/// ```text
/// RME (2025), Supply Air:            00000000 ffffffff                 01000000 02000000 …
/// Snowdon (2024), Supply Air:        00000000 ffffffff e90f ffffffff   01000000 02000000 …
/// MagiCAD (2024), JI:       02000000 4a004900 ffffffff e90f 01000000   03000000 05000000 …
/// ```
pub fn type_classification(payload: &[u8], overrides_tag: Option<u16>) -> Option<u32> {
    (0..payload.len()).find_map(|q| {
        let units = string_at(payload, q)?;
        if !units.iter().all(|&u| u >= 0x20) {
            return None;
        }
        let mut p = q + 4 + 2 * units.len();
        if le_bytes::u32_at(payload, p)? != u32::MAX {
            return None;
        }
        p += 4;
        if overrides_tag.is_some() && le_bytes::u16_at(payload, p) == overrides_tag {
            p += 6;
        }
        le_bytes::u32_at(payload, p).filter(|c| (1..=MAX_CLASSIFICATION).contains(c))
    })
}

/// The marker an electrical system's circuit number is found from:
/// `0xff` × 8 then `01`.
const CIRCUIT_MARKER: [u8; 9] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01];
/// Bytes from the marker's last byte to the count of the entries before the
/// circuit number.
const CIRCUIT_LIST_OFFSET: usize = 20;
/// Bytes of each of those entries.
const CIRCUIT_ENTRY_BYTES: usize = 32;
/// The name Revit gives a circuit with no circuit number.
pub const UNNAMED_CIRCUIT: &str = "<unnamed>";

/// An electrical system's name, its circuit number (RE-162): after the first
/// [`CIRCUIT_MARKER`] in its object, [`CIRCUIT_LIST_OFFSET`] bytes on, a
/// `u32` count of [`CIRCUIT_ENTRY_BYTES`]-byte entries, then the number as `u32
/// n · UTF-16 × n`. An empty number is Revit's [`UNNAMED_CIRCUIT`].
fn circuit_number(object: &[u8]) -> Option<String> {
    let marker = memchr::memmem::find(object, &CIRCUIT_MARKER)? + CIRCUIT_MARKER.len() - 1;
    let list = marker + CIRCUIT_LIST_OFFSET;
    let count = le_bytes::u32_at(object, list)? as usize;
    let units = string_at(object, list + 4 + count.checked_mul(CIRCUIT_ENTRY_BYTES)?)?;
    if units.is_empty() {
        return Some(UNNAMED_CIRCUIT.into());
    }
    units
        .iter()
        .all(|u| (0x20..0x7f).contains(u))
        .then(|| String::from_utf16_lossy(&units))
}

/// Every MEP system in the file, with its members among `elements`. A system
/// whose objects in two streams disagree on its name keeps no name. Empty for
/// a release before 2024 or a schema with none of [`SYSTEM_CLASSES`].
pub fn scan_mep_systems(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &BTreeSet<u32>,
) -> Result<Vec<MepSystem>> {
    if !supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let classes = rf.memoised_schema_classes()?;
    let tag_of = |name: &str| {
        classes
            .classes
            .iter()
            .find(|class| class.name == name)
            .map(|class| class.tag)
    };
    let overrides_tag = tag_of(SYSTEM_OVERRIDES_CLASS);
    let mut tags: BTreeMap<u32, String> = classes
        .classes
        .iter()
        .filter(|class| SYSTEM_CLASSES.contains(&class.name.as_str()))
        .map(|class| (u32::from(class.tag), class.name.clone()))
        .collect();
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(tag) = tag_of(HVAC_SYSTEM_TYPE_CLASS) {
        tags.insert(u32::from(tag), HVAC_SYSTEM_TYPE_CLASS.into());
    }
    let mut systems: BTreeMap<u32, MepSystem> = BTreeMap::new();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    // HVAC system types' classifications, and the words of each HVAC
    // system's payload, its type's id among them.
    let mut type_classes: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    let mut hvac_words: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let streams = rf.partition_stream_names();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (&tag, class) in &tags {
            for hit in memchr::memmem::find_iter(buf, &tag.to_le_bytes()) {
                let Some(p) = hit.checked_sub(16) else {
                    continue;
                };
                let Some(object) = verified_data_object(buf, p) else {
                    continue;
                };
                if object.class & 0xffff != tag {
                    continue;
                }
                let payload = &buf[p + DATA_OBJECT_HEADER..object.end];
                if class == HVAC_SYSTEM_TYPE_CLASS {
                    type_classes.insert(
                        object.element_id,
                        type_classification(payload, overrides_tag),
                    );
                    continue;
                }
                if class == "RbsHvacSystem" {
                    hvac_words.insert(
                        object.element_id,
                        payload
                            .windows(4)
                            .map(|w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
                            .collect(),
                    );
                }
                let name = if class == "RbsElectricalSystem" {
                    circuit_number(&buf[p..object.end])
                } else {
                    first_string(&buf[p + DATA_OBJECT_HEADER..object.end - 4])
                };
                match names.get_mut(&object.element_id) {
                    None => {
                        names.insert(object.element_id, name);
                    }
                    Some(held) => {
                        if *held != name {
                            *held = None;
                        }
                    }
                }
                systems
                    .entry(object.element_id)
                    .or_insert_with(|| MepSystem {
                        id: object.element_id,
                        class: class.clone(),
                        name: None,
                        members: BTreeSet::new(),
                        classification: None,
                    });
            }
        }
    }
    // Tracker: B70.
    // One pass per stream over the objects of `elements`, looking in each
    // payload for a system's id, instead of a backward search for the object
    // around every place a system's id occurs.
    let (Some(&low), Some(&high)) = (systems.keys().next(), systems.keys().next_back()) else {
        return Ok(Vec::new());
    };
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let Ok(objects) = rf.memoised_partition_data_objects(stream) else {
            continue;
        };
        for &(p, object) in objects.iter() {
            if !elements.contains(&object.element_id) {
                continue;
            }
            for word in buf[p + DATA_OBJECT_HEADER..object.end].windows(4) {
                let id = u32::from_le_bytes(word.try_into().expect("4 bytes"));
                if !(low..=high).contains(&id) || id == object.element_id {
                    continue;
                }
                if let Some(system) = systems.get_mut(&id) {
                    system.members.insert(object.element_id);
                }
            }
        }
    }
    for system in systems.values_mut() {
        system.name = names.remove(&system.id).flatten();
        system.classification = hvac_words
            .get(&system.id)
            .and_then(|words| words.iter().find_map(|w| type_classes.get(w).copied()))
            .flatten();
    }
    Ok(systems.into_values().collect())
}
