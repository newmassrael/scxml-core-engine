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

use sce_build::forge::codec_roles::ElementId;
use sce_build::forge::encode_plan::{
    Absent, Automaton, Bound, Carrier, CborValue, Clause, CodecPlan, Derive, EncodePlan, Guard,
    LawInput, LineShape, Measured, Narrow, Number, NumberInto, Offset, Presence, Value, Write,
};
use sce_build::{encode_plan_file, ForgeCompileOptions};

fn resource(stem: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/forge/resources")
        .join(format!("{stem}.scxml"))
}

/// What `read` answers of the plan of the codec document `stem`, whatever its
/// encoding.
fn with_codec_plan<R>(stem: &str, read: impl FnOnce(&CodecPlan<'_>) -> R) -> R {
    encode_plan_file(&resource(stem), &[], &ForgeCompileOptions::default(), read)
        .unwrap_or_else(|err| panic!("{stem}: {err:?}"))
        .unwrap_or_else(|| panic!("{stem} is not a codec"))
}

/// What `read` answers of the plan of the positional codec document `stem`.
fn with_plan<R>(stem: &str, read: impl FnOnce(&EncodePlan<'_>) -> R) -> R {
    with_codec_plan(stem, |plan| match plan {
        CodecPlan::Positional(plan) => read(plan),
        _ => panic!("{stem} is not a positional codec"),
    })
}

/// Every codec document of the resources, by path.
fn resource_documents() -> Vec<PathBuf> {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&resources)
        .expect("the resources directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    paths
}

/// A CBOR map's entries are written in ascending key order, and only an enum
/// whose carrier is signed can be refused (SCE_FORGE.md §4.6.1).
#[test]
fn a_cbor_map_is_written_in_key_order() {
    with_codec_plan("codec_cbor_map", |plan| {
        let CodecPlan::Cbor(plan) = plan else {
            panic!("the codec is a CBOR map");
        };
        let keys: Vec<u8> = plan
            .by_key
            .iter()
            .map(|&index| plan.entries[index].entry.key)
            .collect();
        assert!(!keys.is_empty());
        assert!(
            keys.windows(2).all(|w| w[0] < w[1]),
            "keys out of order: {keys:?}"
        );
        assert_eq!(
            plan.totality.is_total(),
            !plan.entries.iter().any(|item| matches!(
                item.value,
                CborValue::Enum {
                    carrier: Carrier::Signed
                }
            ))
        );
    });
}

/// Every content-line property has one shape, of many lines exactly where it is
/// repeated, and the parameters it writes are its own (SCE_FORGE.md §4.6.4).
#[test]
fn every_content_line_property_has_one_shape_and_its_parameters() {
    let mut properties = 0;
    for path in resource_documents() {
        encode_plan_file(&path, &[], &ForgeCompileOptions::default(), |plan| {
            let CodecPlan::ContentLine(plan) = plan else {
                return;
            };
            for property in &plan.properties {
                properties += 1;
                assert!(property.entry.param.is_none());
                let repeated = property.entry.repeated;
                match property.shape {
                    LineShape::Records { .. } | LineShape::Repeated { .. } => assert!(repeated),
                    LineShape::One { .. } => assert!(!repeated),
                }
                for param in &property.params {
                    assert_eq!(
                        param.entry.property,
                        property.entry.property,
                        "{}: a parameter of another property",
                        path.display()
                    );
                }
            }
        })
        .unwrap_or_else(|err| panic!("{}: {err:?}", path.display()));
    }
    assert!(properties > 10, "only {properties} properties were read");
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
        // The bounds are the protocol's (docs/adr/0077 §7): a zbuf of a `u32`
        // length, and slices of no count it states.
        let Value::Bytes(bound) = plan.value(field(plan, "value")) else {
            panic!("`value` is not bytes");
        };
        assert_eq!(bound.most, Some(4_294_967_295));
        let Value::List { bound, .. } = plan.value(field(plan, "slices")) else {
            panic!("`slices` is not a list");
        };
        assert_eq!(bound.most, None);
    });
}

/// The derived number of `element` in the plan of `stem`, given to `read`.
fn with_number<R>(stem: &str, element: ElementId, read: impl FnOnce(&Number<'_>) -> R) -> R {
    with_plan(stem, |plan| {
        let numbers: Vec<&Number<'_>> = plan
            .derivations
            .iter()
            .filter(|d| d.element == element)
            .filter_map(|d| match &d.by {
                Derive::Number(number) => Some(number),
                _ => None,
            })
            .collect();
        let [number] = numbers.as_slice() else {
            panic!("{stem}: `{element}` is not one derived number");
        };
        read(number)
    })
}

fn field_element(id: &str) -> ElementId {
    ElementId::Field(id.to_string())
}

#[test]
fn a_count_of_a_gated_list_is_zero_where_the_list_is_not_there() {
    with_number(
        "codec_repeat_present_if_basic",
        field_element("num_elems"),
        |number| {
            let [measure] = number.measures.as_slice() else {
                panic!("the count measures one list");
            };
            assert!(matches!(measure.of, Measured::Entries { list } if list.id == "elems"));
            assert!(measure.gated);
            assert!(number.free());
            // At most 32 entries fit a `uint8` with nothing to check.
            assert!(matches!(
                number.into,
                NumberInto::Field {
                    narrow: Narrow::Cast,
                    absent: Absent::Zero,
                    ..
                }
            ));
        },
    );
}

/// The wire holds the length less one, and the type of the content holds a
/// byte or more (docs/adr/0077 §3), so the subtraction is never negative.
#[test]
fn a_length_less_one_of_content_that_is_never_empty_needs_no_check() {
    with_number(
        "codec_zenoh_scout",
        ElementId::Flag {
            field: "cbyte".to_string(),
            flag: "zid_len_m1".to_string(),
        },
        |number| {
            let [measure] = number.measures.as_slice() else {
                panic!("the length measures one field");
            };
            let Measured::Bytes { field, offset } = &measure.of else {
                panic!("the length measures bytes");
            };
            assert_eq!(field.id, "zid");
            assert_eq!(*offset, Offset::Less { by: 1, held: true });
            assert!(matches!(number.into, NumberInto::Flag));
        },
    );
}

#[test]
fn an_encoded_length_measures_the_body_and_widens_into_a_64_bit_field() {
    with_number(
        "codec_zenoh_decl_ext_keyexpr",
        field_element("total_length"),
        |number| {
            let [measure] = number.measures.as_slice() else {
                panic!("the length measures one body");
            };
            assert!(matches!(
                measure.of,
                Measured::Encoded { body, .. } if body.id == "inner"
            ));
            assert!(!measure.gated);
            assert!(number.fixed());
            assert!(matches!(
                number.into,
                NumberInto::Field {
                    narrow: Narrow::Widened,
                    absent: Absent::Never,
                    ..
                }
            ));
        },
    );
}

/// A flag bound to an input that some arms of the variant say is what the arm
/// the value holds says, where it holds one of them (docs/adr/0065 §6 R4a), and
/// the tag the arms are chosen by is the held arm's.
#[test]
fn a_flag_some_arms_say_is_the_held_arms_and_the_tag_is_derived_from_the_arm() {
    with_number(
        "codec_transport_envelope",
        ElementId::Flag {
            field: "header".to_string(),
            flag: "S".to_string(),
        },
        |number| {
            let [measure] = number.measures.as_slice() else {
                panic!("the flag measures the arms once");
            };
            let Measured::ArmSays { arms } = &measure.of else {
                panic!("the flag is what an arm says");
            };
            let said: Vec<(usize, &str)> = arms.iter().map(|a| (a.arm, a.codec.as_str())).collect();
            assert_eq!(
                said,
                [(0, "codec_zenoh_init_body"), (6, "codec_zenoh_join")]
            );
            assert!(measure.gated, "five arms say nothing of it");
            assert!(matches!(number.into, NumberInto::Flag));
        },
    );
    with_plan("codec_transport_envelope", |plan| {
        // Seven arms by tag, then the default, which keeps the tag it reads.
        assert_eq!(plan.arms.len(), 8);
        assert!(plan.arms[..7].iter().all(|arm| arm.tag.is_some()));
        assert_eq!(plan.arms[7].tag, None);
        let tag = ElementId::Flag {
            field: "header".to_string(),
            flag: "mid".to_string(),
        };
        assert!(plan
            .derivations
            .iter()
            .any(|d| d.element == tag && matches!(d.by, Derive::VariantTag)));
    });
}

/// An input the parent forwards to a list whose entries say it is said by the
/// value (docs/adr/0065 §6 R8a): the encode is handed nothing, and the entries'
/// codec is handed nothing for it.
#[test]
fn an_input_a_list_says_is_said_and_handed_to_no_one() {
    with_plan("codec_chain_flag_forward", |plan| {
        assert!(plan.given.is_empty());
        let said: Vec<&str> = plan.said.iter().map(|s| s.input.name.as_str()).collect();
        assert_eq!(said, ["wide"]);
        assert!(plan.handed["codec_chain_flag_entry"].is_empty());
    });
}

/// A gate of two flags is two clauses, joined as its author joined them, and
/// the group it decides holds the field, so nothing is checked
/// (docs/adr/0065 §6 R4c).
#[test]
fn a_disjunction_is_two_clauses_and_its_group_leaves_nothing_to_check() {
    with_plan("codec_present_if_disjunction", |plan| {
        let gate = field(plan, "seq")
            .present_if
            .as_ref()
            .expect("`seq` is gated");
        let bits: Vec<(u32, bool)> = plan
            .clauses(gate)
            .iter()
            .map(|clause| match clause {
                Clause::Flag {
                    carrier,
                    bit,
                    negate,
                } => {
                    assert_eq!(carrier.id, "flags");
                    (*bit, *negate)
                }
                _ => panic!("each clause reads a flag of `flags`"),
            })
            .collect();
        assert_eq!(bits, [(0, false), (1, false)]);
        assert!(plan.gate_checks.is_empty());
    });
}

/// A codec whose encode cannot refuse plans no check (docs/adr/0065): a check
/// is a refusal.
#[test]
fn a_total_codec_plans_no_check() {
    let mut total = 0;
    for path in resource_documents() {
        encode_plan_file(&path, &[], &ForgeCompileOptions::default(), |plan| {
            let CodecPlan::Positional(plan) = plan else {
                return;
            };
            if plan.totality.is_total() {
                total += 1;
                assert!(
                    plan.input_checks.is_empty() && plan.gate_checks.is_empty(),
                    "{}: a total codec plans a check",
                    path.display()
                );
            }
        })
        .unwrap_or_else(|err| panic!("{}: {err:?}", path.display()));
    }
    assert!(total > 60, "only {total} total codecs were read");
}

/// Fields of a positional layout that share a byte are written as one byte.
#[test]
fn fields_that_share_a_byte_are_packed_into_one_write() {
    with_plan("codec_subbyte", |plan| {
        let [Write::Byte { at: 0, pieces }] = plan.program.as_slice() else {
            panic!("the codec is one packed byte");
        };
        let ids: Vec<&str> = pieces.iter().map(|(f, _)| f.id.as_str()).collect();
        assert_eq!(ids, ["priority", "channel", "direction"]);
    });
}

/// A field whose bits run past its first byte is written a byte at a time,
/// each byte the piece of the field's word it holds (docs/adr/0095 §2): the
/// 12-bit big-endian `reading` four bits into byte 1 is all of byte 1 and the
/// high nibble of byte 2, after the whole flag carrier at byte 0.
#[test]
fn a_field_across_two_bytes_is_written_a_byte_at_a_time() {
    with_plan("codec_optional_field_attributes", |plan| {
        let [Write::Whole(status), Write::Byte {
            at: 1,
            pieces: high,
        }, Write::Byte { at: 2, pieces: low }] = plan.program.as_slice()
        else {
            panic!("a whole carrier and two bytes of the reading");
        };
        assert_eq!(status.id, "status");
        let [(reading, first)] = high.as_slice() else {
            panic!("byte 1 holds the reading alone");
        };
        assert_eq!(reading.id, "reading");
        assert_eq!((first.from, first.mask, first.to), (4, 0xFF, 0));
        let [(_, second)] = low.as_slice() else {
            panic!("byte 2 holds the reading alone");
        };
        assert_eq!((second.from, second.mask, second.to), (0, 0x0F, 4));
    });
}

/// A gated chain closed by its entries' flag is written where the value holds
/// it, through its law, each entry but the last told another follows.
#[test]
fn a_gated_chain_is_written_where_present_through_its_law_and_its_end_flag() {
    with_plan("codec_chain_prev_gated", |plan| {
        let writes: Vec<(&str, Guard)> = plan
            .program
            .iter()
            .map(|write| match write {
                Write::Streamed { field, guard, .. } => (field.id.as_str(), *guard),
                _ => panic!("the codec is streamed"),
            })
            .collect();
        assert_eq!(
            writes,
            [("head", Guard::Always), ("entries", Guard::Present)]
        );
        let Some(Write::Streamed {
            list: Some(list), ..
        }) = plan.program.last()
        else {
            panic!("the chain is walked");
        };
        assert_eq!(list.ends, Some(("header".to_string(), "Z".to_string())));
        assert!(list.lawful);
        assert_eq!(list.slots, None);
    });
}

#[test]
fn every_element_is_one_field_once_and_in_declaration_order() {
    let mut codecs = 0;
    for path in resource_documents() {
        let checked = encode_plan_file(&path, &[], &ForgeCompileOptions::default(), |plan| {
            let CodecPlan::Positional(plan) = plan else {
                return false;
            };
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
            true
        })
        .unwrap_or_else(|err| panic!("{}: {err:?}", path.display()));
        codecs += usize::from(checked == Some(true));
    }
    assert!(codecs > 60, "only {codecs} positional codecs were read");
}
