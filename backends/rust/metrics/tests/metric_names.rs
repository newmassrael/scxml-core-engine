// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The registry of names held to the vector both lanes read (docs/adr/0109, D16):
//! tests/metrics/metric_names.json names every family, its type, its unit, its labels and each
//! label's closed set of values, and the lanes that report it. The C++ header host/Metrics.h is
//! held to the families listed for `cpp` by tests/integration/AStandardHostMetricsTest.cpp, so
//! the two lanes write one surface in the same bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sce_rust_metrics::{
    AuthenticatorKind, Counter, Eviction, ExchangeOutcome, Family, Gauge, Histogram, HostFailure,
    HostOperation, PoolShed, ResponseStatus, RestoredFrom, ServerShed, FAMILIES, METRICS_INFO,
    REGISTRY,
};
use serde_json::Value;

fn vector() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/metrics/metric_names.json");
    let text = std::fs::read_to_string(&path).expect("the vector file is readable");
    serde_json::from_str(&text).expect("the vector file is JSON")
}

/// Every counter series of the registry, each label set taken whole.
fn every_counter() -> Vec<Counter> {
    let mut all = Vec::new();
    for status in ResponseStatus::ALL {
        all.push(Counter::ServerExchanges { status: *status });
    }
    for status in ExchangeOutcome::ALL {
        all.push(Counter::ClientExchanges { status: *status });
    }
    for reason in ServerShed::ALL {
        all.push(Counter::ServerSheds { reason: *reason });
    }
    for authenticator in AuthenticatorKind::ALL {
        all.push(Counter::AuthenticationFailures {
            authenticator: *authenticator,
        });
    }
    for checkpoint in RestoredFrom::ALL {
        all.push(Counter::Restores {
            checkpoint: *checkpoint,
        });
    }
    for reason in Eviction::ALL {
        all.push(Counter::Evictions { reason: *reason });
    }
    for reason in PoolShed::ALL {
        all.push(Counter::PoolSheds { reason: *reason });
    }
    for operation in HostOperation::ALL {
        for code in HostFailure::ALL {
            all.push(Counter::HostOperationFailures {
                operation: *operation,
                code: *code,
            });
        }
    }
    all.extend(Counter::UNLABELLED);
    all
}

/// What the crate's series carry, by family name: each label's name, in order, with the set of
/// values some series gives it.
type Carried = BTreeMap<&'static str, Vec<(&'static str, BTreeSet<&'static str>)>>;

/// Every series of the crate, gathered by family. The info family carries the registry's name.
fn carried() -> Carried {
    let mut carried: Carried = BTreeMap::new();
    let mut note = |family: &'static Family, labels: &[(&'static str, &'static str)]| {
        let held = carried.entry(family.name).or_insert_with(|| {
            labels
                .iter()
                .map(|(name, _)| (*name, BTreeSet::new()))
                .collect()
        });
        assert_eq!(
            held.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            labels.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            "{}: two series name their labels differently",
            family.name
        );
        for ((_, values), (_, value)) in held.iter_mut().zip(labels) {
            values.insert(*value);
        }
    };
    for counter in every_counter() {
        note(counter.family(), counter.labels().as_slice());
    }
    for gauge in Gauge::ALL {
        note(gauge.family(), gauge.labels().as_slice());
    }
    for histogram in Histogram::UNLABELLED {
        note(histogram.family(), histogram.labels().as_slice());
    }
    for operation in HostOperation::ALL {
        let histogram = Histogram::HostOperationDuration {
            operation: *operation,
        };
        note(histogram.family(), histogram.labels().as_slice());
    }
    note(&METRICS_INFO, &[("registry", REGISTRY)]);
    carried
}

fn text(value: &Value, member: &str) -> String {
    value[member]
        .as_str()
        .unwrap_or_else(|| panic!("{member} is a text: {value}"))
        .to_string()
}

/// The vector's registry is the crate's; every family the vector lists for `rust` is one of
/// `FAMILIES` with the same type, unit and label names, and each label's values are the closed
/// set the crate's series carry; and the crate reports no family the vector does not list for it.
/// Red if a family, a label or a value is renamed, added or removed in the crate and not in the
/// vector, or the reverse.
#[test]
fn every_family_rust_reports_is_the_vectors() {
    let vector = vector();
    assert_eq!(text(&vector, "registry"), REGISTRY);
    let carried = carried();
    let mut listed: BTreeSet<String> = BTreeSet::new();
    for family in vector["families"].as_array().expect("families") {
        let name = text(family, "name");
        let lanes: Vec<&str> = family["lanes"]
            .as_array()
            .expect("lanes")
            .iter()
            .map(|lane| lane.as_str().expect("a lane"))
            .collect();
        assert!(
            lanes.iter().all(|lane| ["rust", "cpp"].contains(lane)),
            "{name}: {lanes:?}"
        );
        if !lanes.contains(&"rust") {
            continue;
        }
        assert!(listed.insert(name.clone()), "{name} is listed twice");
        let ours = FAMILIES
            .iter()
            .find(|ours| ours.name == name)
            .unwrap_or_else(|| panic!("{name} is listed for rust and is no family of the crate"));
        assert_eq!(text(family, "type"), ours.kind.name(), "{name}");
        assert_eq!(text(family, "unit"), ours.unit, "{name}");
        let labels = family["labels"].as_array().expect("labels");
        let names: Vec<String> = labels.iter().map(|label| text(label, "name")).collect();
        assert_eq!(names, ours.labels, "{name}");
        let series = carried
            .get(ours.name)
            .unwrap_or_else(|| panic!("{name}: no series of the crate names it"));
        assert_eq!(series.len(), labels.len(), "{name}");
        for (label, (carried_name, values)) in labels.iter().zip(series) {
            let written = label["values"].as_array().expect("values");
            let listed_values: BTreeSet<String> = written
                .iter()
                .map(|value| value.as_str().expect("a value").to_string())
                .collect();
            assert_eq!(
                written.len(),
                listed_values.len(),
                "{name}.{carried_name}: a value listed twice"
            );
            let carried_values: BTreeSet<String> =
                values.iter().map(|value| value.to_string()).collect();
            assert_eq!(listed_values, carried_values, "{name}.{carried_name}");
        }
    }
    let ours: BTreeSet<String> = FAMILIES
        .iter()
        .map(|family| family.name.to_string())
        .collect();
    assert_eq!(listed, ours, "the families the vector lists for rust");
}
