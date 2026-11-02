// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The interface of a codec's encode plan (docs/adr/0081 §1), held against the
//! documents that hold each shape.
//!
//! The Rust backend prints the plan, and a regeneration that changes no file is
//! what shows the plan holds what the Rust encode decided. These tests pin what
//! the plan says of each shape in terms of the document, so that a backend that
//! prints it next reads the same answer.

use std::path::{Path, PathBuf};

use sce_build::forge::encode_plan::{Automaton, Bound, EncodePlan, LawInput, Presence, Value};
use sce_build::{encode_plan_file, ForgeCompileOptions};

fn resource(stem: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/forge/resources")
        .join(format!("{stem}.scxml"))
}

/// What `read` answers of the plan of the codec document `stem`.
fn with_plan<R>(stem: &str, read: impl FnOnce(&EncodePlan<'_>) -> R) -> R {
    encode_plan_file(&resource(stem), &[], &ForgeCompileOptions::default(), read)
        .unwrap_or_else(|err| panic!("{stem}: {err:?}"))
        .unwrap_or_else(|| panic!("{stem} is not a codec"))
}

/// The ids of the elements the plan of `stem` presents alone, in order.
fn element_ids(plan: &EncodePlan<'_>) -> Vec<String> {
    plan.elements.iter().map(|e| e.field.id.clone()).collect()
}

fn field<'p>(plan: &EncodePlan<'p>, id: &str) -> &'p sce_build::forge::model::CodecField {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("no field {id}"))
}

/// A flag set is presented as a field, and a value that sets the flag a gate
/// derives sets nothing: the encode writes it (docs/adr/0058 §1).
#[test]
fn a_count_is_derived_and_not_presented() {
    with_plan("codec_repeat_present_if_basic", |plan| {
        assert_eq!(element_ids(plan), ["carrier", "elems"]);
        assert_eq!(plan.presence(field(plan, "elems")), Presence::Gated);
    });
}

#[test]
fn a_list_whose_entries_say_the_flag_it_hands_them_follows_a_uniform_law() {
    with_plan("codec_repeat_flag_carrier", |plan| {
        assert_eq!(element_ids(plan), ["head", "entries"]);
        let Value::List { codec, bound, law } = plan.value(field(plan, "entries")) else {
            panic!("`entries` is not a list");
        };
        assert_eq!(codec, "codec_chain_flag_entry");
        assert_eq!(
            bound,
            Bound {
                least: 0,
                most: Some(4)
            }
        );
        assert_eq!(
            law.inputs,
            [LawInput {
                input: "wide".to_string(),
                automaton: Automaton::Uniform,
            }]
        );
    });
}

#[test]
fn a_chain_whose_entries_say_the_entry_before_follows_that_law_and_holds_one_entry() {
    with_plan("codec_chain_prev_gated", |plan| {
        let entries = field(plan, "entries");
        assert!(plan.presents_alone(entries));
        assert_eq!(plan.presence(entries), Presence::Gated);
        let Value::List { bound, law, .. } = plan.value(entries) else {
            panic!("`entries` is not a list");
        };
        // A chain closed by an entry flag holds one entry or more (docs/adr/0065
        // §6 R7), and no more than its depth.
        assert_eq!(
            bound,
            Bound {
                least: 1,
                most: Some(4)
            }
        );
        // Every entry says the flag the chain hands all of them, and each says
        // whether the entry before it is the marker.
        assert_eq!(
            law.inputs,
            [
                LawInput {
                    input: "tagged".to_string(),
                    automaton: Automaton::Uniform,
                },
                LawInput {
                    input: "after_marker".to_string(),
                    automaton: Automaton::AfterKey { id: 0x4 },
                }
            ]
        );
    });
}

/// The fields a gate decides are presented by their group, not alone, and keep
/// their bounds there (docs/adr/0065 §6 R4).
#[test]
fn the_fields_a_gate_decides_are_presented_by_their_group_within_their_bounds() {
    with_plan("codec_zenoh_value_zbuf", |plan| {
        assert!(element_ids(plan).is_empty());
        let [group] = plan.groups() else {
            panic!("the codec holds one group");
        };
        for id in ["value", "slices"] {
            assert!(group.contains(id), "the group does not hold `{id}`");
        }
        let Value::Bytes(bound) = plan.value(field(plan, "value")) else {
            panic!("`value` is not bytes");
        };
        assert_eq!(bound.most, Some(32));
        let Value::List { bound, .. } = plan.value(field(plan, "slices")) else {
            panic!("`slices` is not a list");
        };
        assert_eq!(bound.most, Some(4));
    });
}

#[test]
fn every_element_is_one_field_once_and_in_declaration_order() {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&resources)
        .expect("the resources directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    let mut codecs = 0;
    for path in paths {
        let checked = encode_plan_file(&path, &[], &ForgeCompileOptions::default(), |plan| {
            let order: Vec<usize> = plan
                .elements
                .iter()
                .map(|e| {
                    plan.model
                        .fields
                        .iter()
                        .position(|f| std::ptr::eq(f, e.field))
                        .expect("an element is a field of the codec")
                })
                .collect();
            assert!(
                order.windows(2).all(|w| w[0] < w[1]),
                "{}: the elements are not in declaration order: {order:?}",
                path.display()
            );
            for element in &plan.elements {
                assert!(
                    plan.groups().iter().all(|g| !g.contains(&element.field.id)),
                    "{}: `{}` is presented alone and by a group",
                    path.display(),
                    element.field.id
                );
                assert_eq!(
                    element.presence == Presence::Gated,
                    element.field.present_if.is_some(),
                    "{}: `{}`",
                    path.display(),
                    element.field.id
                );
            }
        })
        .unwrap_or_else(|err| panic!("{}: {err:?}", path.display()));
        codecs += usize::from(checked.is_some());
    }
    assert!(codecs > 60, "only {codecs} codec documents were read");
}
