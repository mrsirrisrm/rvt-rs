//! HVAC system classification (probe): where a duct system says whether it
//! supplies, returns or exhausts air.
//!
//! Revit names a new system after its type (`Mechanical Supply Air 3`,
//! `Mechanical Return Air 1`), and a type carries the system's
//! classification. For each `RbsHvacSystem` data object (RE-162) the probe
//! prints its name and every `u32` in its payload that is the ElementId of
//! another data object, with that object's class; then, for each system type
//! so referenced, its payload and the classification
//! `type_classification` reads from it. The schema's fields of
//! `RbsHvacSystemType` and its bases come first.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_hvac_system_types -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::research::partition_room_parameters::DATA_OBJECT_HEADER;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        println!("== {path}");
        let mut rf = RevitFile::open(&path)?;
        let release = rf.layout_release().unwrap_or(0);
        let registry = rvt::research::schema_registry::parse(
            &rvt::research::native_document::read_single(&mut rf, "Formats/Latest")?,
        )?;
        let mut class = registry.named("RbsHvacSystemType");
        while let Some(c) = class {
            let fields: Vec<String> = c
                .fields
                .iter()
                .map(|f| format!("{} ({:#x})", f.name, f.raw_descriptor))
                .collect();
            println!("class {} tag {:#x}: {}", c.name, c.tag, fields.join(", "));
            class = registry
                .class(c.parent_reference.tag)
                .filter(|p| p.tag != c.tag);
        }
        let class_names: BTreeMap<u32, String> = rf
            .schema_classes()?
            .classes
            .into_iter()
            .map(|class| (u32::from(class.tag), class.name))
            .collect();
        // Every data object: id -> (class name, payload), the last stream's.
        let mut objects: BTreeMap<u32, (String, Vec<u8>)> = BTreeMap::new();
        for stream in rf.partition_stream_names() {
            let (Ok(inflated), Ok(found)) = (
                rf.inflated_partition(&stream),
                rf.partition_data_objects(&stream),
            ) else {
                continue;
            };
            let buf = inflated.bytes();
            for (start, object) in found.iter() {
                let Some(payload) = buf.get(start + DATA_OBJECT_HEADER..object.end) else {
                    continue;
                };
                let class = class_names
                    .get(&(object.class & 0xffff))
                    .cloned()
                    .unwrap_or_else(|| format!("{:#x}", object.class));
                objects.insert(object.element_id, (class, payload.to_vec()));
            }
        }
        let systems = rvt::research::partition_mep_systems::scan_mep_systems(
            &mut rf,
            release,
            &BTreeSet::new(),
        )?;
        let mut referenced: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
        for system in systems.iter().filter(|s| s.class == "RbsHvacSystem") {
            let Some((_, payload)) = objects.get(&system.id) else {
                continue;
            };
            let refs: Vec<String> = (0..payload.len().saturating_sub(3))
                .filter_map(|q| {
                    let id = u32::from_le_bytes(payload[q..q + 4].try_into().unwrap());
                    let (class, _) = objects.get(&id).filter(|_| id != system.id && id > 100)?;
                    if class.starts_with("RbsHvac")
                        || class.contains("System")
                        || class.contains("Type")
                    {
                        referenced
                            .entry(id)
                            .or_default()
                            .insert(system.name.clone().unwrap_or_default());
                    }
                    Some(format!("+{q} {id} {class}"))
                })
                .collect();
            println!(
                "{} {:?} classification {:?} ({} B): {}",
                system.id,
                system.name,
                system.classification,
                payload.len(),
                refs.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
            );
        }
        println!("-- referenced objects");
        for (id, by) in &referenced {
            let (class, payload) = &objects[id];
            let hex: Vec<String> = payload
                .chunks(32)
                .map(|c| c.iter().map(|b| format!("{b:02x}")).collect())
                .collect();
            let by: Vec<&String> = by.iter().take(3).collect();
            println!("{id} {class} ({} B) by {:?}…", payload.len(), by);
            println!(
                "   type_classification: {:?}",
                rvt::research::partition_mep_systems::type_classification(
                    payload,
                    registry.named("SystemOverrides").map(|c| c.tag)
                )
            );
            for (k, line) in hex.iter().enumerate() {
                println!("   {:4} {line}", k * 32);
            }
        }
    }
    Ok(())
}
