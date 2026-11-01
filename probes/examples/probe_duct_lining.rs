//! Insulation and lining (probe): where a covering's thickness, type and host
//! are stored.
//!
//! FACT (BIM4LCA housing HVAC, Revit 2024): `RbsDuctInsulation`,
//! `RbsPipeInsulation` and `RbsDuctLining` derive from
//! `RbsInsulationLiningBase`, whose one field `m_thickness` is the thickness
//! in feet; `m_idType` names the type, and each `Connector` object's
//! `m_arrRefs` names the covered element. The 173 + 1,190 records are the
//! export's 1,363 `IfcCovering` Tags, with Revit's host and thickness
//! (`tests/duct_insulation.rs`). Neither thickness nor host is a saved
//! parameter. No measured model holds an `RbsDuctLining` record.
//!
//! Prints each class's base chain, a `row` per record (id, class,
//! thickness, type, the ids its connectors name), a `count` per class, the
//! object graphs of the first `N` records of each class (default 3) and the
//! saved parameters of those records and of the elements they cover.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_duct_lining -- MODEL.rvt [N]

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

const CLASSES: [&str; 6] = [
    "RbsDuctLining",
    "RbsDuctInsulation",
    "RbsPipeInsulation",
    "RbsInsulation",
    "RbsInsulationLiningBase",
    "FabricationPartInsulationLining",
];

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: MODEL.rvt [N]");
    let shown: usize = args.next().map_or(3, |n| n.parse().expect("N"));
    let mut rf = RevitFile::open(&path)?;
    let schema = rf.schema_classes()?;
    for name in CLASSES
        .iter()
        .chain(&["RbsDuctLiningType", "RbsDuctInsulationType"])
    {
        for class in schema.classes.iter().filter(|c| c.name == *name) {
            let mut chain = vec![format!(
                "{}({}, {} fields)",
                class.name, class.tag, class.field_count
            )];
            let mut base = class.base;
            while let Some(b) = base.and_then(|t| schema.by_tag(t)) {
                chain.push(format!("{}({}, {} fields)", b.name, b.tag, b.field_count));
                base = b.base;
            }
            println!("schema {}", chain.join(" < "));
        }
    }
    let options = Options {
        selected_classes: CLASSES.iter().map(|c| c.to_string()).collect(),
        ..Options::default()
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut printed: BTreeMap<String, usize> = BTreeMap::new();
    let mut ids: BTreeSet<u64> = BTreeSet::new();
    let mut shown_hosts: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let class = record.class_name.clone().unwrap_or_default();
        *counts.entry(class.clone()).or_default() += 1;
        ids.insert(record.identity.element_id);
        if let Some(graph) = &record.graph {
            let root = graph.objects.first().map(|o| &o.fields);
            let thickness = root.and_then(|f| f["m_thickness"].as_f64());
            let type_id = root.and_then(|f| f["m_idType"]["m_id"]["m_id64"].as_i64());
            let hosts: BTreeSet<u64> = graph
                .objects
                .iter()
                .filter(|o| o.class_name == "Connector")
                .flat_map(|o| {
                    o.fields["m_arrRefs"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                })
                .filter_map(|r| r["m_id"]["m_id"]["m_id64"].as_u64())
                .collect();
            println!(
                "row {} {class} thickness_ft {thickness:?} type {type_id:?} hosts {hosts:?}",
                record.identity.element_id
            );
        }
        let n = printed.entry(class.clone()).or_default();
        if *n < shown {
            *n += 1;
            println!(
                "record {} {class} status {} diag {:?}",
                record.identity.element_id, record.status, record.diagnostic
            );
            if let Some(graph) = &record.graph {
                let hosts = shown_hosts.entry(record.identity.element_id).or_default();
                for object in graph.objects.iter().filter(|o| o.class_name == "Connector") {
                    for r in object.fields["m_arrRefs"].as_array().into_iter().flatten() {
                        if let Some(id) = r["m_id"]["m_id"]["m_id64"].as_u64() {
                            hosts.insert(id);
                        }
                    }
                }
                for object in &graph.objects {
                    println!("  obj {} {}", object.class_name, object.fields);
                }
            }
        }
        Ok(())
    })?;
    for (class, n) in &counts {
        println!("count {class} {n}");
    }
    // Saved parameters of the first shown records and of the elements
    // their connectors name.
    let mut wanted: BTreeSet<u64> = BTreeSet::new();
    for (id, hosts) in shown_hosts.iter() {
        wanted.insert(*id);
        wanted.extend(hosts.iter().copied());
    }
    let read = rvt::research::native_element_parameters::read(&mut rf, &wanted)?;
    for id in &wanted {
        for p in read.elements.get(id).into_iter().flatten() {
            println!(
                "param {id} {} {:?} {} {} si {:?} {:?} from {} {}",
                p.parameter_id,
                p.name,
                p.storage_type,
                p.value,
                p.si_value,
                p.si_unit,
                p.source_element_id,
                p.source_class
            );
        }
    }
    Ok(())
}
