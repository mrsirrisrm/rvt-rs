//! MEP spaces (probe): what the `OST_MEPSpaces` records carry.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_mep_spaces -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::{partition_room_parameters, partition_schema_mvp as mvp};

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("MODEL.rvt");
    let mut f = RevitFile::open(&path)?;
    let v = f.layout_release().unwrap_or(0);
    let levels = mvp::level_element_ids(&mut f, v)?;
    let spaces =
        mvp::instances_from_partition_category_records(&mut f, v, -2_003_600, "Space", &levels)?;
    let rooms =
        mvp::instances_from_partition_category_records(&mut f, v, -2_000_160, "Room", &levels)?;
    println!(
        "release {v}, levels {}, spaces {}, rooms {}",
        levels.len(),
        spaces.len(),
        rooms.len()
    );
    let ids: BTreeSet<u32> = spaces.iter().filter_map(|s| s.id).collect();
    let params =
        partition_room_parameters::scan_room_parameters(&mut f, v, &ids).unwrap_or_default();
    println!("parameter blocks {}", params.len());
    let entries = partition_room_parameters::scan_room_parameter_entries(&mut f, v, &ids);
    println!("parameter entries {}", entries.len());
    for (id, e) in entries.iter().take(4) {
        println!("  {id}: {e:?}");
    }
    for s in spaces.iter().take(4) {
        let g = |n: &str| {
            s.fields
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, v)| format!("{v:?}"))
                .unwrap_or_default()
        };
        println!(
            "{:?} at ({}, {}, {}) box ({}, {}, {}) level {} {:?}",
            s.id,
            g("m_locationX"),
            g("m_locationY"),
            g("m_locationZ"),
            g("m_bboxWidth"),
            g("m_bboxDepth"),
            g("m_bboxHeight"),
            g("m_levelId"),
            s.id.and_then(|i| params.get(&i))
                .map(|b| (&b.name, &b.number))
        );
    }
    Ok(())
}
