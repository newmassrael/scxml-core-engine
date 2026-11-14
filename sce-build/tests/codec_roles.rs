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

/// Whether the document at `path` says its flag input `input`, read out of the XML
/// (docs/adr/0065 §6 R4a, R4b, R4d): the fields it presents gated on the input
/// alone, one or a group of them, and nothing else names it; or no field is gated
/// on it and it is handed on once, to an import held by one embed that is always
/// there, which says it; or it is handed to every arm of one variant, which each
/// says the input it is handed.
fn says_input_in_xml(path: &Path, input: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    let whole = fields_derived_whole(&doc);
    let names_input = |gate: &str| {
        gate.split("||")
            .any(|clause| clause.trim().trim_start_matches('!').trim() == input)
    };
    let mut alone = 0;
    let mut elsewhere = false;
    for node in doc.descendants().filter(|n| n.is_element()) {
        let (Some(id), Some(gate)) = (node.attribute("id"), node.attribute((SCE_NS, "present-if")))
        else {
            continue;
        };
        if !names_input(gate) {
            continue;
        }
        if gate.contains("||") {
            elsewhere = true;
        } else if !whole.contains(id) {
            alone += 1;
        }
    }
    let forwards: Vec<(roxmltree::Node, &str)> = doc
        .descendants()
        .filter(|n| is_sce(n, "flag-bind") && n.attribute("source") == Some(input))
        .filter_map(|n| Some((n.parent()?, n.attribute("input")?)))
        .collect();
    match (alone, forwards.as_slice(), elsewhere) {
        // One field says it, or the group of the fields it decides does
        // (docs/adr/0065 §6 R4b).
        (n, [], false) if n >= 1 => true,
        (0, [(import, bound)], false) if held_by_one_embed_always_there(&doc, import) => {
            import.attribute("src").is_some_and(|src| {
                says_input_in_xml(&path.parent().expect("a directory").join(src), bound)
            })
        }
        (0, handed, false) if !handed.is_empty() => said_by_every_arm(&doc, path, handed),
        _ => false,
    }
}

/// Whether the codec `import` names is held by one embed, with no gate.
fn held_by_one_embed_always_there(doc: &roxmltree::Document, import: &roxmltree::Node) -> bool {
    let Some(alias) = import.attribute("as") else {
        return false;
    };
    let holders: Vec<roxmltree::Node> = doc
        .descendants()
        .filter(|n| n.is_element() && n.attribute("type") == Some(alias))
        .collect();
    matches!(holders.as_slice(), [embed] if is_sce(embed, "embed")
        && embed.attribute((SCE_NS, "present-if")).is_none())
}

/// docs/adr/0065 §6 R4d: an input the codec hands to every arm of one variant,
/// the default among them, says the input when each arm's codec says the input it
/// is handed, since a value holds one arm. The arms may name the input otherwise;
/// two binds of the input on one arm's import are not this rule, and an import
/// held anywhere but the variant's arms is not either.
fn said_by_every_arm(
    doc: &roxmltree::Document,
    path: &Path,
    handed: &[(roxmltree::Node, &str)],
) -> bool {
    let dir = path.parent().expect("a directory");
    let mut arms_codecs: Vec<(&str, &str, &str)> = Vec::new();
    for (import, bound) in handed {
        let (Some(alias), Some(src)) = (import.attribute("as"), import.attribute("src")) else {
            return false;
        };
        if arms_codecs.iter().any(|(seen, _, _)| *seen == alias) {
            return false;
        }
        arms_codecs.push((alias, *bound, src));
    }
    doc.descendants()
        .filter(|n| is_sce(n, "variant"))
        .any(|variant| {
            let arms: Vec<roxmltree::Node> = variant
                .children()
                .filter(|n| is_sce(n, "arm") || is_sce(n, "default"))
                .collect();
            let every_arm_says = !arms.is_empty()
                && arms.iter().all(|arm| {
                    arm.attribute("type").is_some_and(|ty| {
                        arms_codecs.iter().any(|(alias, bound, src)| {
                            *alias == ty && says_input_in_xml(&dir.join(src), bound)
                        })
                    })
                });
            let held_only_by_its_arms = arms_codecs.iter().all(|(alias, _, _)| {
                doc.descendants()
                    .filter(|n| n.is_element() && n.attribute("type") == Some(*alias))
                    .all(|n| n.parent() == Some(variant))
            });
            every_arm_says && held_only_by_its_arms
        })
}

/// The carrier flags bound to an input that a body or an arm says, read out of the
/// XML: derived from it, as a parent tag is (docs/adr/0065 §6 R4a). A flag bound to
/// the entries of a list that say it is derived too, as the list's start (§6 R8a),
/// unless the list is a chain a gate reads the membership of, whose flag is checked.
fn bound_to_a_said_input_in_xml(path: &Path) -> BTreeSet<ElementId> {
    let text = std::fs::read_to_string(path).expect("the document");
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    let mut bound = BTreeSet::new();
    for bind in doc.descendants().filter(|n| is_sce(n, "flag-bind")) {
        let (Some(source), Some(input)) = (bind.attribute("source"), bind.attribute("input"))
        else {
            continue;
        };
        let Some((carrier, flag)) = source.split_once('.') else {
            continue;
        };
        let Some(import) = bind.parent() else {
            continue;
        };
        let (Some(alias), Some(src)) = (import.attribute("as"), import.attribute("src")) else {
            continue;
        };
        let held_by_a_body_or_an_arm = doc.descendants().any(|n| {
            n.is_element()
                && n.attribute("type") == Some(alias)
                && (is_sce(&n, "embed") || is_sce(&n, "arm") || is_sce(&n, "default"))
        });
        // A gate that reads a chain's membership names it as `{chain}.has(`.
        let a_lawful_list_holds_it = doc.descendants().any(|n| {
            n.is_element()
                && n.attribute("type") == Some(alias)
                && (is_sce(&n, "repeat") || is_sce(&n, "tlv-chain"))
                && n.attribute("id").is_some_and(|list| {
                    let member = format!("{list}.has(");
                    !doc.descendants().any(|g| {
                        g.attribute((SCE_NS, "present-if"))
                            .is_some_and(|gate| gate.contains(&member))
                    })
                })
        });
        if (held_by_a_body_or_an_arm || a_lawful_list_holds_it)
            && says_input_in_xml(&path.parent().expect("a directory").join(src), input)
        {
            bound.insert(ElementId::Flag {
                field: carrier.trim().to_string(),
                flag: flag.trim().to_string(),
            });
        }
    }
    bound
}

/// The flags of a disjunction that decides its fields together, read out of the
/// XML (docs/adr/0065 §6 R4c): every clause of the gate reads a one-bit flag of a
/// carrier of the document, a different one each, and no other gate of the
/// document reads any of them. Each is derived from the arm the value holds.
fn disjunction_flags_in_xml(path: &Path) -> BTreeSet<ElementId> {
    let text = std::fs::read_to_string(path).expect("the document");
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    let whole = fields_derived_whole(&doc);
    let one_bit: BTreeSet<(String, String)> = doc
        .descendants()
        .filter(|n| is_sce(n, "flags"))
        .flat_map(|carrier| {
            let id = carrier.attribute("id").unwrap_or_default().to_string();
            carrier
                .children()
                .filter(|f| is_sce(f, "flag"))
                .filter(|f| f.attribute("width").is_none_or(|w| w == "1"))
                .filter_map(move |f| Some((id.clone(), f.attribute("name")?.to_string())))
                .collect::<Vec<_>>()
        })
        .collect();
    let gates: Vec<(&str, &str)> = doc
        .descendants()
        .filter(|n| n.is_element())
        .filter_map(|n| Some((n.attribute("id")?, n.attribute((SCE_NS, "present-if"))?)))
        .collect();
    let clauses_of = |gate: &str| -> Vec<String> {
        gate.split("||")
            .map(|c| c.trim().trim_start_matches('!').trim().to_string())
            .collect()
    };
    let reads = |flag: &str| {
        gates
            .iter()
            .flat_map(|(_, g)| clauses_of(g))
            .filter(|c| c == flag)
            .count()
    };
    let mut derived = BTreeSet::new();
    for (id, gate) in &gates {
        let clauses = clauses_of(gate);
        if clauses.len() < 2 || whole.contains(*id) {
            continue;
        }
        let flags: Option<Vec<(String, String)>> = clauses
            .iter()
            .map(|c| {
                let (carrier, flag) = c.split_once('.')?;
                let pair = (carrier.to_string(), flag.to_string());
                one_bit.contains(&pair).then_some(pair)
            })
            .collect();
        let Some(flags) = flags else {
            continue;
        };
        let distinct: BTreeSet<&(String, String)> = flags.iter().collect();
        // Fields under the same gate read its flags once each among them.
        let under_the_same_gate = gates
            .iter()
            .filter(|(_, g)| clauses_of(g) == clauses)
            .count();
        if distinct.len() == flags.len() && clauses.iter().all(|c| reads(c) == under_the_same_gate)
        {
            derived.extend(
                flags
                    .into_iter()
                    .map(|(field, flag)| ElementId::Flag { field, flag }),
            );
        }
    }
    derived
}

/// The flags joined with a chain's membership in a gate, read out of the XML
/// (docs/adr/0065 §6 R6b): a one-bit flag of a carrier of the document that a
/// gate names beside `<chain>.has(…)`, where every gate that names it names
/// nothing but one-bit flags and that chain's memberships. Each is derived from
/// the arm the value holds.
fn membership_flags_in_xml(path: &Path) -> BTreeSet<ElementId> {
    let text = std::fs::read_to_string(path).expect("the document");
    let doc = roxmltree::Document::parse(&text).expect("well-formed XML");
    let whole = fields_derived_whole(&doc);
    let one_bit: BTreeSet<String> = doc
        .descendants()
        .filter(|n| is_sce(n, "flags"))
        .flat_map(|carrier| {
            let id = carrier.attribute("id").unwrap_or_default().to_string();
            carrier
                .children()
                .filter(|f| is_sce(f, "flag"))
                .filter(|f| f.attribute("width").is_none_or(|w| w == "1"))
                .filter_map(move |f| Some(format!("{id}.{}", f.attribute("name")?)))
                .collect::<Vec<_>>()
        })
        .collect();
    let chains: BTreeSet<&str> = doc
        .descendants()
        .filter(|n| is_sce(n, "tlv-chain"))
        .filter_map(|n| n.attribute("id"))
        .collect();
    let clauses_of = |gate: &str| -> Vec<String> {
        gate.split("||")
            .map(|c| c.trim().trim_start_matches('!').trim().to_string())
            .collect()
    };
    let chain_of = |clause: &str| {
        clause
            .split_once(".has(")
            .map(|(chain, _)| chain.to_string())
            .filter(|chain| chains.contains(chain.as_str()))
    };
    let gates: Vec<Vec<String>> = doc
        .descendants()
        .filter(|n| n.is_element())
        .filter(|n| n.attribute("id").is_none_or(|id| !whole.contains(id)))
        .filter_map(|n| n.attribute((SCE_NS, "present-if")))
        .map(clauses_of)
        .collect();
    let mut derived = BTreeSet::new();
    for gate in &gates {
        let Some(chain) = gate.iter().find_map(|c| chain_of(c)) else {
            continue;
        };
        for flag in gate.iter().filter(|c| one_bit.contains(*c)) {
            let joined_only = gates.iter().filter(|g| g.contains(flag)).all(|g| {
                g.iter()
                    .all(|c| one_bit.contains(c) || chain_of(c).as_deref() == Some(&chain))
            });
            if joined_only {
                let (field, name) = flag.split_once('.').expect("a carrier's flag");
                derived.insert(ElementId::Flag {
                    field: field.to_string(),
                    flag: name.to_string(),
                });
            }
        }
    }
    derived
}

#[test]
fn the_classification_agrees_with_the_declarations_in_the_document() {
    for (path, roles) in codecs() {
        let derived: BTreeSet<ElementId> = roles.derived().map(|(id, _)| id.clone()).collect();
        let mut declared = declared_in_xml(&path);
        declared.extend(dispatched_in_xml(&path));
        declared.extend(bound_to_a_said_input_in_xml(&path));
        declared.extend(disjunction_flags_in_xml(&path));
        declared.extend(membership_flags_in_xml(&path));
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
fn a_chains_membership_of_a_bare_entry_is_a_group_that_holds_the_chain() {
    // The marker 0x12 is all its value is, so the gate on it is a group whose
    // arms each hold the chain, refined, with the fields of each side, and no
    // check is left for a source (docs/adr/0065 §6 R6a). The check stays for an
    // encode that holds no group.
    let roles = roles_of("codec_chain_has_marker");
    let group = roles
        .groups
        .iter()
        .find(|g| g.chain().is_some())
        .unwrap_or_else(|| panic!("{}", roles.table()));
    assert_eq!(group.chain(), Some("extensions"));
    assert_eq!(group.members().collect::<Vec<_>>(), [(0, 0x12)]);
    assert!(group.member_is_bare());
    let fields: Vec<(&[bool], &[String])> = group
        .arms
        .iter()
        .map(|a| (a.values.as_slice(), a.fields.as_slice()))
        .collect();
    assert_eq!(
        fields,
        [
            (
                &[true][..],
                &["extensions".to_string(), "slices".to_string()][..]
            ),
            (
                &[false][..],
                &["extensions".to_string(), "payload".to_string()][..]
            ),
        ]
    );
    assert!(group.absent.is_none());
    assert!(roles.source_checks().next().is_none(), "{}", roles.table());
    assert!(!roles.checks.is_empty(), "{}", roles.table());
}

#[test]
fn a_chains_membership_of_an_entry_with_content_is_a_group_that_holds_it() {
    // `has(0x2)` names an entry by its 4-bit id alone, which leaves the entry
    // its mandatory bit and its body: the value holds the entry where it
    // stands, and the group holds the chain whole (docs/adr/0065 §6 R6b).
    let roles = roles_of("codec_chain_has_envelope");
    let group = roles
        .groups
        .iter()
        .find(|g| g.chain().is_some())
        .unwrap_or_else(|| panic!("{}", roles.table()));
    assert_eq!(group.chain(), Some("extensions"));
    assert_eq!(group.members().collect::<Vec<_>>(), [(0, 0x2)]);
    assert!(!group.member_is_bare());
    assert!(roles.source_checks().next().is_none(), "{}", roles.table());
}

#[test]
fn the_gates_on_a_chains_entries_and_the_flags_joined_with_them_are_one_group() {
    // `priority` is present where `entries.has(7) || header.wide`, `checksum`
    // where `!entries.has(9)`. The three atoms decide both fields together, so
    // they make one group of an arm for each assignment of them, each holding
    // the chain, which is always present, and the fields whose gates hold
    // (docs/adr/0065 §6 R6b). `wide` is derived from the arm, and no check is
    // left for a source.
    let roles = roles_of("codec_chain_has_tagged");
    let group = roles
        .groups
        .iter()
        .find(|g| g.chain().is_some())
        .unwrap_or_else(|| panic!("{}", roles.table()));
    assert_eq!(group.chain(), Some("entries"));
    assert_eq!(group.members().collect::<Vec<_>>(), [(0, 7), (1, 9)]);
    assert_eq!(group.gates[2].to_string(), "header.wide");
    assert!(group.absent.is_none());
    let field = |id: &str| id.to_string();
    let arms: Vec<(&[bool], &[String])> = group
        .arms
        .iter()
        .map(|a| (a.values.as_slice(), a.fields.as_slice()))
        .collect();
    let with_priority = [field("entries"), field("priority")];
    let with_checksum = [field("entries"), field("checksum")];
    let with_both = [field("entries"), field("priority"), field("checksum")];
    assert_eq!(
        arms,
        [
            (&[true, true, true][..], &with_priority[..]),
            (&[true, true, false][..], &with_priority[..]),
            (&[true, false, true][..], &with_both[..]),
            (&[false, true, true][..], &with_priority[..]),
            (&[true, false, false][..], &with_both[..]),
            (&[false, true, false][..], &[field("entries")][..]),
            (&[false, false, true][..], &with_both[..]),
            (&[false, false, false][..], &with_checksum[..]),
        ]
    );
    let (held, not) = group.kinds_in(&group.arms[2]);
    assert_eq!((held, not), (vec![7], vec![9]));
    assert!(
        matches!(
            roles.role_of(&ElementId::Flag {
                field: "header".into(),
                flag: "wide".into(),
            }),
            Some(Role::Derived(_))
        ),
        "{}",
        roles.table()
    );
    assert!(roles.source_checks().next().is_none(), "{}", roles.table());
}

#[test]
fn a_disjunction_of_the_codecs_own_flags_is_a_group_of_its_assignments() {
    // Each assignment that satisfies the gate is an arm, and each flag is
    // derived from the arm a value holds (docs/adr/0065 §6 R4c). The gate stays
    // a check for an encode that holds no group, and a source is not held to it.
    let roles = roles_of("codec_present_if_disjunction");
    assert!(!roles.checks.is_empty(), "{}", roles.table());
    assert!(roles.source_checks().next().is_none(), "{}", roles.table());
    assert_eq!(
        roles.table(),
        "codec codec_present_if_disjunction\n  \
         flags          presented\n  \
         flags.wants_a  derived  arm-of(flags.wants_a || flags.wants_b)\n  \
         flags.wants_b  derived  arm-of(flags.wants_a || flags.wants_b)\n  \
         seq            presented\n  \
         group  flags.wants_a || flags.wants_b  10 (seq)  01 (seq)  11 (seq)  00 (none)\n"
    );
}

#[test]
fn a_disjunction_one_of_whose_flags_another_gate_reads_is_a_check_a_source_is_held_to() {
    // `has_seq` is derived from whether the value holds `seq`, so it is not the
    // disjunction's to derive: the disjunction is no group (docs/adr/0065 §6
    // R4c) but a check, `wants_trailer` is the value's own, and the source is
    // held to the gate, which the encode refuses a value that contradicts.
    let roles = roles_of("codec_present_if_checked");
    assert!(roles.groups.is_empty(), "{}", roles.table());
    let flag = |flag: &str| ElementId::Flag {
        field: "flags".to_string(),
        flag: flag.to_string(),
    };
    assert!(
        matches!(roles.role_of(&flag("has_seq")), Some(Role::Derived(_))),
        "{}",
        roles.table()
    );
    assert_eq!(
        roles.role_of(&flag("wants_trailer")),
        Some(&Role::Presented),
        "{}",
        roles.table()
    );
    let held: Vec<&str> = roles.source_checks().map(|c| c.field.as_str()).collect();
    assert_eq!(held, ["trailer"], "{}", roles.table());
}

#[test]
fn more_gates_on_a_chain_than_a_value_says_stay_checks() {
    // A value holds an arm for each assignment of a chain's gates, and the
    // runtime's sets of kinds go up to eight (docs/adr/0065 §6 R6b). Up to that
    // the gates are a group, so the encode refuses nothing and the declaration
    // that it may is refused; past it they stay checks, which the declaration
    // states, and the document compiles.
    let compile = |kinds: u32| {
        let fields: String = (1..=kinds)
            .map(|k| {
                format!(
                    r#"<sce:field id="f{k}" sce:type="uint8" sce:byte="{k}" sce:bit-size="8"
                                  sce:present-if="entries.has({k})"/>"#
                )
            })
            .collect();
        let scxml = format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                      sce:kind="codec" sce:encode-may-fail="true" sce:default-endian="big" name="many_kinds">
                 <sce:import src="codec_chain_has_tagged_entry.scxml" kind="codec" as="codec_chain_has_tagged_entry"/>
                 <datamodel>
                   <sce:tlv-chain id="entries" type="codec_chain_has_tagged_entry" sce:byte="0"
                                  max-depth="3"                                  terminate-on="entry-flag" entry-flag-name="more"
                                  entry-id="entry_type"/>
                   {fields}
                 </datamodel>
               </scxml>"#
        );
        sce_build::compile_forge_with_imports(
            &scxml,
            sce_build::DocumentLabel::symmetric("many_kinds"),
            sce_build::generator::Language::Rust,
            &resources(),
            &ForgeCompileOptions::default(),
        )
    };
    let Err(refused) = compile(8) else {
        panic!("eight kinds are a group, so the declaration is refused");
    };
    assert!(
        refused.error.to_string().contains("refuses no value"),
        "{}",
        refused.error
    );
    compile(9).unwrap_or_else(|e| panic!("nine kinds stay checks: {}", e.error));
}

#[test]
fn a_disjunction_with_a_chains_membership_is_one_group_with_the_chain() {
    // `entries.has(7) || header.wide` is no disjunction of the codec's own
    // flags (R4c), and the chain's memberships decide it with `wide`: one group
    // of the three atoms, listed as atoms, not joined (docs/adr/0065 §6 R6b).
    // The checks stay for an encode that holds no group.
    let roles = roles_of("codec_chain_has_tagged");
    assert!(!roles.checks.is_empty(), "{}", roles.table());
    let table = roles.table();
    assert!(
        table.contains(
            "header.wide   derived  arm-of(entries.has(0x7), entries.has(0x9), header.wide)"
        ),
        "{table}"
    );
    assert!(
        table.contains(
            "group  entries.has(0x7), entries.has(0x9), header.wide  111 (entries, priority)"
        ),
        "{table}"
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
    // the gated `weights`. One derivation with two sources, not two derivations,
    // and a group of the flag holds the two as one value of one count, so the
    // sources cannot disagree where a group holds them (docs/adr/0065 §6 R5).
    let roles = roles_of("codec_repeat_unconditional_count");
    assert!(roles.ambiguous().is_empty(), "{}", roles.table());
    assert_eq!(
        roles.shared(),
        vec![&ElementId::Field("links_len".into())],
        "{}",
        roles.table()
    );
    assert!(roles.counted_with_another("weights"));
    assert!(!roles.counted_with_another("links"));
    assert_eq!(
        roles.table(),
        "codec codec_repeat_unconditional_count\n  \
         options    presented\n  \
         options.H  derived  presence-of(weights)\n  \
         links_len  derived  count-of(links, weights) [the sources agree in size]\n  \
         links      presented\n  \
         weights    presented\n  \
         group  options.H  1 (links, weights)  0 (links)  one count (links, weights)\n"
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
