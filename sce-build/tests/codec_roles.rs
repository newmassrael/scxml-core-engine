// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which element of a codec a value presents and which the encoding derives
//! (docs/adr/0058 §1), held against the documents.
//!
//! The classification is computed from the parsed model
//! (`sce_build::forge::codec_roles`). These tests ask three things of it over
//! every codec document in `tests/forge/resources`: that it finds no element
//! derived twice and none derived that the codec does not have; that it agrees
//! with a second reader that never sees the model, one that reads the
//! declarations out of the document's XML; and that the tables of the shapes a
//! consumer sets by hand come out as they are pinned here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sce_build::forge::codec_roles::{CodecRoles, ElementId, Role};
use sce_build::{codec_roles_file, ForgeCompileOptions};

const SCE_NS: &str = "http://sce.dev/ext";

fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// Every codec document of the resources, with its roles.
fn codecs() -> Vec<(PathBuf, CodecRoles)> {
    let mut found = Vec::new();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(resources())
        .expect("the resources directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    for path in paths {
        let roles = codec_roles_file(&path, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|err| panic!("{}: {err:?}", path.display()));
        if let Some(roles) = roles {
            found.push((path, roles));
        }
    }
    found
}

fn roles_of(stem: &str) -> CodecRoles {
    codecs()
        .into_iter()
        .find(|(path, _)| path.file_stem().is_some_and(|s| s == stem))
        .unwrap_or_else(|| panic!("no codec document {stem}"))
        .1
}

#[test]
fn the_resources_hold_codecs_of_every_shape_that_derives() {
    let all = codecs();
    assert!(
        all.len() > 60,
        "only {} codec documents were found",
        all.len()
    );
    let table: String = all.iter().map(|(_, roles)| roles.table()).collect();
    for shape in [
        "length-of(",
        "count-of(",
        "encoded-length-of(",
        "variant-tag",
        "parent-tag(",
        "chain-continuation(",
    ] {
        assert!(
            table.contains(shape),
            "no codec document of the resources derives by `{shape}`"
        );
    }
}

#[test]
fn no_element_is_derived_twice_or_derived_that_is_not_there() {
    for (path, roles) in codecs() {
        assert!(
            roles.unresolved.is_empty(),
            "{}: {:?}",
            path.display(),
            roles.unresolved
        );
        assert!(
            roles.ambiguous().is_empty(),
            "{}: derived by two declarations: {:?}",
            path.display(),
            roles.ambiguous()
        );
    }
}

/// How a gate reads, taken from the attribute's text alone: the one flag it
/// names when it is the plain `carrier.flag` or `!carrier.flag` form, else
/// nothing (a disjunction, a flag-input's bare name, a chain's `has(..)`).
fn flag_of_simple_gate(gate: &str) -> Option<ElementId> {
    let gate = gate.trim();
    if gate.contains("||") {
        return None;
    }
    let (field, flag) = gate
        .strip_prefix('!')
        .unwrap_or(gate)
        .trim()
        .split_once('.')?;
    if flag.contains('(') {
        return None;
    }
    Some(ElementId::Flag {
        field: field.trim().to_string(),
        flag: flag.trim().to_string(),
    })
}

/// The fields the document derives whole: those a `sce:length-field`, a
/// `sce:length-from`, a `count=` or the tag of a variant of its own names without
/// a dot, read out of the XML. A field derived whole is gated by a flag and says
/// nothing back to it.
fn fields_derived_whole(doc: &roxmltree::Document) -> BTreeSet<String> {
    let peeks: BTreeSet<String> = doc
        .descendants()
        .filter(|n| is_sce(n, "peek-byte"))
        .filter_map(|n| n.attribute("id").map(str::to_string))
        .collect();
    let mut whole = BTreeSet::new();
    let mut take = |value: &str| {
        if !value.contains('.') {
            whole.insert(value.trim().to_string());
        }
    };
    for node in doc.descendants().filter(|n| n.is_element()) {
        if let Some(value) = node.attribute((SCE_NS, "length-field")) {
            take(value);
        }
        if let Some(value) = node.attribute((SCE_NS, "length-from")) {
            take(value);
        }
        if is_sce(&node, "repeat") {
            if let Some(value) = node.attribute("count") {
                take(value);
            }
        }
        if is_sce(&node, "variant") {
            if let Some(tag) = node.attribute("tag") {
                if !peeks.contains(tag.split('.').next().unwrap_or(tag)) {
                    take(tag);
                }
            }
        }
    }
    whole
}

/// The fields whose gate names no one flag, read out of the XML.
fn fields_with_a_gate_that_is_checked(path: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(path).expect("the document");
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    let whole = fields_derived_whole(&doc);
    doc.descendants()
        .filter(|n| n.is_element())
        .filter_map(|n| Some((n.attribute("id")?, n.attribute((SCE_NS, "present-if"))?)))
        .filter(|(id, gate)| flag_of_simple_gate(gate).is_none() && !whole.contains(*id))
        .map(|(id, _)| id.to_string())
        .collect()
}

/// What the document's XML declares as derived, read without the model: the
/// elements named by `sce:length-field`, by `count=` on a `<sce:repeat>`, by
/// `sce:length-from`, by the tag of a `<sce:variant>` that has a tag of its
/// own, the flag a `<sce:tlv-chain terminate-on="entry-flag">` ends on, and the
/// flag a field is gated on in the plain `sce:present-if` form.
fn declared_in_xml(path: &Path) -> BTreeSet<ElementId> {
    let text = std::fs::read_to_string(path).expect("the document");
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    let peeks: BTreeSet<String> = doc
        .descendants()
        .filter(|n| is_sce(n, "peek-byte"))
        .filter_map(|n| n.attribute("id").map(str::to_string))
        .collect();
    let reference = |value: &str| match value.split_once('.') {
        Some((field, flag)) => ElementId::Flag {
            field: field.trim().to_string(),
            flag: flag.trim().to_string(),
        },
        None => ElementId::Field(value.trim().to_string()),
    };
    let whole = fields_derived_whole(&doc);
    let mut declared = BTreeSet::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        if let Some(length) = node.attribute((SCE_NS, "length-field")) {
            declared.insert(reference(length));
        }
        if let Some(length) = node.attribute((SCE_NS, "length-from")) {
            declared.insert(reference(length));
        }
        // A flag is derived from the gated fields that are presented.
        if let Some(flag) = node
            .attribute((SCE_NS, "present-if"))
            .and_then(flag_of_simple_gate)
            .filter(|_| !node.attribute("id").is_some_and(|id| whole.contains(id)))
        {
            declared.insert(flag);
        }
        if is_sce(&node, "repeat") {
            if let Some(count) = node.attribute("count") {
                declared.insert(reference(count));
            }
        }
        if is_sce(&node, "variant") {
            if let Some(tag) = node.attribute("tag") {
                if !peeks.contains(tag.split('.').next().unwrap_or(tag)) {
                    declared.insert(reference(tag));
                }
            }
        }
        if is_sce(&node, "tlv-chain") && node.attribute("terminate-on") == Some("entry-flag") {
            declared.insert(ElementId::Flag {
                field: node.attribute("id").expect("a chain has an id").to_string(),
                flag: node
                    .attribute("entry-flag-name")
                    .expect("an entry-flag chain names its flag")
                    .to_string(),
            });
        }
    }
    declared
}

fn is_sce(node: &roxmltree::Node, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace() == Some(SCE_NS)
}

/// The elements a parent codec's `<sce:variant-dispatch flag=>` names.
fn dispatched_in_xml(path: &Path) -> BTreeSet<ElementId> {
    let text = std::fs::read_to_string(path).expect("the document");
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    doc.descendants()
        .filter(|n| is_sce(n, "variant-dispatch"))
        .filter_map(|n| n.attribute("flag"))
        .filter_map(|flag| flag.split_once('.'))
        .map(|(field, flag)| ElementId::Flag {
            field: field.trim().to_string(),
            flag: flag.trim().to_string(),
        })
        .collect()
}

#[test]
fn the_classification_agrees_with_the_declarations_in_the_document() {
    for (path, roles) in codecs() {
        let derived: BTreeSet<ElementId> = roles.derived().map(|(id, _)| id.clone()).collect();
        let mut declared = declared_in_xml(&path);
        declared.extend(dispatched_in_xml(&path));
        assert_eq!(
            derived,
            declared,
            "{}: the model derives what the document does not declare, or the reverse",
            path.display()
        );
        let checked: BTreeSet<String> = roles.checks.iter().map(|c| c.field.clone()).collect();
        assert_eq!(
            checked,
            fields_with_a_gate_that_is_checked(&path),
            "{}: the gates held as checks are not the ones that name no one flag",
            path.display()
        );
    }
}

#[test]
fn a_flag_that_gates_a_field_is_derived_from_the_presence() {
    assert_eq!(
        roles_of("codec_present_if_basic").table(),
        "codec codec_present_if_basic\n  \
         flags          presented\n  \
         flags.has_seq  derived  presence-of(seq)\n  \
         seq            presented\n"
    );
}

#[test]
fn a_negated_gate_is_derived_from_the_absence() {
    let table = roles_of("codec_present_if_negation").table();
    assert!(table.contains("absence-of("), "{table}");
}

#[test]
fn a_gate_of_several_flags_is_a_check_and_derives_no_flag() {
    let roles = roles_of("codec_present_if_disjunction");
    assert!(!roles.checks.is_empty(), "{}", roles.table());
    assert!(
        roles.table().contains("is present exactly when") && roles.table().contains("||"),
        "{}",
        roles.table()
    );
}

#[test]
fn a_header_flag_nothing_declares_is_presented() {
    // The flag a consumer clears by hand to say something its content does not
    // is a thing a value can say (ADR 0058 §1, the Interest case).
    let roles = roles_of("codec_flags_basic");
    assert!(
        roles
            .rows
            .iter()
            .any(|row| matches!(row.id, ElementId::Flag { .. })),
        "the fixture has a flag to look at"
    );
    assert!(roles.derived().next().is_none(), "{}", roles.table());
}

#[test]
fn the_table_of_a_length_prefixed_field() {
    assert_eq!(
        roles_of("codec_length_ref").table(),
        "codec codec_length_ref\n  \
         msgId    presented\n  \
         len      derived  length-of(payload)\n  \
         payload  presented\n"
    );
}

#[test]
fn a_count_two_lists_share_is_one_derivation_whose_sources_agree() {
    // zenoh's LinkState: `links_len` is written once and counts both `links` and
    // the gated `weights`. One derivation with two sources, not two derivations.
    let roles = roles_of("codec_repeat_unconditional_count");
    assert!(roles.ambiguous().is_empty(), "{}", roles.table());
    assert_eq!(
        roles.shared(),
        vec![&ElementId::Field("links_len".into())],
        "{}",
        roles.table()
    );
    assert_eq!(
        roles.table(),
        "codec codec_repeat_unconditional_count\n  \
         options    presented\n  \
         options.H  derived  presence-of(weights)\n  \
         links_len  derived  count-of(links, weights) [the sources agree in size]\n  \
         links      presented\n  \
         weights    presented\n"
    );
}

#[test]
fn a_count_is_derived_and_its_entries_are_presented() {
    let roles = roles_of("codec_repeat_basic");
    let count = roles
        .rows
        .iter()
        .find(|row| row.id == ElementId::Field("num_frags".into()))
        .expect("the count field");
    assert!(
        matches!(&count.role, Role::Derived(by) if by.len() == 1),
        "{}",
        roles.table()
    );
    assert_eq!(
        roles.role_of(&ElementId::Field("frags".into())),
        Some(&Role::Presented)
    );
}

#[test]
fn a_peek_byte_variant_has_no_tag_of_its_own() {
    let roles = roles_of("codec_variant_peek_basic");
    assert_eq!(
        roles.role_of(&ElementId::Variant),
        Some(&Role::Presented),
        "{}",
        roles.table()
    );
    assert!(roles.derived().next().is_none(), "{}", roles.table());
}

#[test]
fn a_dispatching_carrier_bit_is_derived_and_the_rest_of_the_carrier_is_not() {
    let roles = roles_of("codec_embed_variant_dispatch");
    let table = roles.table();
    assert!(table.contains("hdr.kind"), "{table}");
    assert!(table.contains("derived  parent-tag("), "{table}");
    assert_eq!(
        roles.role_of(&ElementId::Field("hdr".into())),
        Some(&Role::Presented),
        "the carrier byte is the value's, but for the bits the arm fills\n{table}"
    );
}
