// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rule that no two names a forge document declares become one member of
//! the code generated for it (`docs/SCE_ACCEPTED_SUBSET.md` §2.14).
//!
//! An author's names are case-sensitive, and `minRpm` and `min_rpm` are two
//! of them. A backend then spells a name in its own convention, and a
//! convention that folds two names together declares one parameter, field or
//! method twice: Rust, C11 and Python write a parameter or a codec field
//! snake_case, Go writes a codec member Pascal. The result is a
//! duplicate-argument `SyntaxError` in Python, a duplicate field in Rust and
//! two values read into one variable, with nothing in the document saying
//! so. A document SCE accepts must not generate code that does not build, or
//! that builds and means something else.
//!
//! **Refused for every backend at once**, as a reserved word is
//! (`validation/reserved-code-identifier`) and as a statechart's names are
//! ([`crate::member_names`]): a document can be generated in any of the six,
//! and whether SCE accepts it must not depend on which one a deployment
//! builds.
//!
//! **The spelling is the templates' own.** Each [`Slot`] records how every
//! backend spells one kind of declaration, measured from the generator and
//! its templates, and
//! `sce-build/tests/a_forge_declaration_is_spelled_the_way_the_collision_rule_says.rs`
//! generates every committed forge document in every backend with each of its
//! declarations renamed and reads the name back, so a template that changes
//! its convention fails there rather than here.
//!
//! **A scope is a namespace, not a document.** Two names collide only where
//! the generated code puts them in one place. A Rust struct keeps a field and
//! a method of one name apart and a Python class does not, so a codec member
//! and a flag's accessor are compared only in the backends where they share
//! a namespace ([`SCOPES`]). The first version of a rule like this folds every
//! name to snake_case and compares them all, and refuses a document that
//! builds in every language.
//!
//! **A spelling can depend on the kind and the role.** A forge `<data id>` is
//! a snake_case parameter in a transform's input and a member as written in
//! an event schema's payload, and Python writes a filter's input as written
//! where Rust writes it snake_case. A slot therefore names the kinds it holds
//! for and, where the role matters, the `sce:direction`, and claims nothing
//! about the rest.
//!
//! What this does not cover is stated in `docs/SCE_ACCEPTED_SUBSET.md`: only
//! the declarations in [`SCOPES`] are compared, and a declaration that is not
//! listed there is not known to be safe, only not measured.

use std::collections::HashMap;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::generator::Language;
use crate::reader_names::Case;
use crate::scxml_identifier::Dialect;

/// The namespace an element is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ns {
    /// W3C's `<data>`.
    Scxml,
    /// SCE's `sce:` elements.
    Sce,
}

impl Ns {
    pub fn uri(self) -> &'static str {
        match self {
            Ns::Scxml => crate::model::SCXML_NAMESPACE,
            Ns::Sce => crate::forge::model::SCE_NAMESPACE,
        }
    }
}

/// One attribute that declares a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub ns: Ns,
    pub element: &'static str,
    pub attr: &'static str,
}

/// One kind of declaration, and how every backend spells it.
#[derive(Debug)]
pub struct Slot {
    /// What a message calls one — `codec member`, `flag`.
    pub noun: &'static str,
    /// The attributes that declare one.
    pub rows: &'static [Row],
    /// The kinds of document (`sce:kind`) it holds for; empty is every kind.
    pub kinds: &'static [&'static str],
    /// The `sce:direction` the declaring element carries, where the role
    /// decides the spelling; `None` is any.
    pub direction: Option<&'static str>,
    /// How each backend spells it, in [`Language::ALL`] order. `None` is a
    /// backend that does not put this declaration in the scope's namespace:
    /// it emits nothing for it, writes it as written, or emits it somewhere
    /// the rest of the scope is not.
    pub cases: [Option<Case>; 6],
}

/// The declarations the generated code puts in one namespace.
#[derive(Debug)]
pub struct Scope {
    /// What the scope is, for a reader of the table and of the tests.
    pub id: &'static str,
    pub slots: &'static [Slot],
}

const S: Option<Case> = Some(Case::Snake);
const C: Option<Case> = Some(Case::Camel);
const P: Option<Case> = Some(Case::Pascal);
const V: Option<Case> = Some(Case::Verbatim);
const U: Option<Case> = Some(Case::UpperSnake);
const N: Option<Case> = None;

/// Every kind of document.
const ANY: &[&str] = &[];

const DATA_ID: Row = Row {
    ns: Ns::Scxml,
    element: "data",
    attr: "id",
};

const PARAM_NAME: Row = Row {
    ns: Ns::Sce,
    element: "param",
    attr: "name",
};

const CODEC_MEMBER_ROWS: &[Row] = &[
    Row {
        ns: Ns::Sce,
        element: "field",
        attr: "id",
    },
    Row {
        ns: Ns::Sce,
        element: "flags",
        attr: "id",
    },
    Row {
        ns: Ns::Sce,
        element: "repeat",
        attr: "id",
    },
    Row {
        ns: Ns::Sce,
        element: "tlv-chain",
        attr: "id",
    },
    Row {
        ns: Ns::Sce,
        element: "embed",
        attr: "id",
    },
];

/// The members a C++ struct holds as written and a decode function binds as
/// written: every one but an embed, whose decode local is snake_case beside
/// a member written as written (measured), which is two spellings and not
/// one, so nothing is claimed for it in C++.
const CODEC_MEMBER_ROWS_BUT_EMBED: &[Row] = &[
    Row {
        ns: Ns::Sce,
        element: "field",
        attr: "id",
    },
    Row {
        ns: Ns::Sce,
        element: "flags",
        attr: "id",
    },
    Row {
        ns: Ns::Sce,
        element: "repeat",
        attr: "id",
    },
    Row {
        ns: Ns::Sce,
        element: "tlv-chain",
        attr: "id",
    },
];

const EMBED_ROW: Row = Row {
    ns: Ns::Sce,
    element: "embed",
    attr: "id",
};

const FLAG_ROWS: &[Row] = &[Row {
    ns: Ns::Sce,
    element: "flag",
    attr: "name",
}];

const FLAG_INPUT_ROWS: &[Row] = &[Row {
    ns: Ns::Sce,
    element: "flag-input",
    attr: "name",
}];

const CONST_ROWS: &[Row] = &[Row {
    ns: Ns::Sce,
    element: "const",
    attr: "name",
}];

const HELPER_ROWS: &[Row] = &[Row {
    ns: Ns::Sce,
    element: "helper",
    attr: "name",
}];

/// Every namespace a forge document's declarations share, in the order a
/// refusal prefers: the narrow scopes first, so that the message names the
/// pair as the author would.
///
/// ⚠ The rows are what the generator and its templates do, and the test named
/// in the module header holds them to it. Add a scope by measuring first
/// (`measure_the_spelling_of_every_declaring_attribute` in that test).
pub const SCOPES: &[Scope] = &[
    // The input of a forge document is a parameter of one function or a field
    // of one record, and Rust, C11 and Python spell it snake_case. Measured
    // per kind, because it is not one rule: a filter's Python input is
    // written as written, an event schema's payload field is written as
    // written in C11 and Python, and an output or an internal is spelled by
    // the role it plays, which the table does not claim.
    Scope {
        id: "forge-data",
        slots: &[
            Slot {
                noun: "data id",
                rows: &[DATA_ID],
                kinds: &[
                    "condition",
                    "lookup",
                    "observer",
                    "procedure",
                    "transform",
                    "validator",
                ],
                direction: Some("in"),
                cases: [S, N, N, N, S, S],
            },
            Slot {
                noun: "data id",
                rows: &[DATA_ID],
                kinds: &["filter"],
                direction: Some("in"),
                cases: [S, N, N, N, N, S],
            },
            Slot {
                noun: "data id",
                rows: &[DATA_ID],
                kinds: &["event-schema"],
                direction: Some("in"),
                cases: [S, N, N, N, N, N],
            },
            // An algorithm's parameter list.
            Slot {
                noun: "parameter",
                rows: &[PARAM_NAME],
                kinds: &["algorithm"],
                direction: None,
                cases: [S, N, N, N, S, S],
            },
        ],
    },
    // A codec's field, flags carrier, repeat, TLV chain and embed — and its
    // `<data>`, which a codec reads as a member — are each a member of the one
    // generated record. Rust, Python and C11 spell it snake_case and Go
    // Pascal; C++ and Kotlin write it as written, so they never fold two
    // names (measured: Kotlin does not camel-case a member).
    Scope {
        id: "codec-member",
        slots: &[
            Slot {
                noun: "codec member",
                rows: CODEC_MEMBER_ROWS,
                kinds: ANY,
                direction: None,
                cases: [S, N, N, P, S, S],
            },
            Slot {
                noun: "codec member",
                rows: &[DATA_ID],
                kinds: &["codec"],
                direction: None,
                cases: [S, N, N, P, S, S],
            },
        ],
    },
    // A flag's accessor, and its `set_` twin, are methods of that record.
    Scope {
        id: "codec-flag",
        slots: &[Slot {
            noun: "flag",
            rows: FLAG_ROWS,
            kinds: ANY,
            direction: None,
            cases: [S, S, C, P, S, N],
        }],
    },
    // A flag-input is a parameter of the generated `decode` and `encode`.
    Scope {
        id: "codec-flag-input",
        slots: &[Slot {
            noun: "flag input",
            rows: FLAG_INPUT_ROWS,
            kinds: ANY,
            direction: None,
            cases: [S, S, C, C, S, S],
        }],
    },
    // A const is a module-level name, upper-snake in every backend.
    Scope {
        id: "const",
        slots: &[Slot {
            noun: "const",
            rows: CONST_ROWS,
            kinds: ANY,
            direction: None,
            cases: [U, U, U, U, U, U],
        }],
    },
    // A procedure keeps its data and its helpers as members of one record: a
    // field in Rust and C11, `self._<name>` in Python. They share that
    // namespace whatever a datum's direction, and an input and a helper are
    // also two parameters of the generated `execute`. `seedKey` and `seed_key`
    // are one Python attribute, so the helper replaced the datum and the call
    // `seed_key(seedKey)` called the helper with itself. C++, Go and Kotlin
    // write both as written, and two names that differ are two members there.
    Scope {
        id: "procedure-member",
        slots: &[
            Slot {
                noun: "data id",
                rows: &[DATA_ID],
                kinds: &["procedure"],
                direction: None,
                cases: [S, N, N, N, S, S],
            },
            Slot {
                noun: "helper",
                rows: HELPER_ROWS,
                kinds: &["procedure"],
                direction: None,
                cases: [S, N, N, N, S, S],
            },
        ],
    },
    // In C++, Go and Python a member and a method of one name are one name:
    // the class (the struct) holds both in one table. Rust and Kotlin keep a
    // field and a function apart, so they are not here.
    Scope {
        id: "codec-class",
        slots: &[
            Slot {
                noun: "codec member",
                rows: CODEC_MEMBER_ROWS_BUT_EMBED,
                kinds: ANY,
                direction: None,
                cases: [N, V, N, P, S, N],
            },
            // An embed is a Python attribute like the rest; C++ is left out.
            Slot {
                noun: "codec member",
                rows: &[EMBED_ROW],
                kinds: ANY,
                direction: None,
                cases: [N, N, N, N, S, N],
            },
            Slot {
                noun: "codec member",
                rows: &[DATA_ID],
                kinds: &["codec"],
                direction: None,
                cases: [N, V, N, P, S, N],
            },
            Slot {
                noun: "flag",
                rows: FLAG_ROWS,
                kinds: ANY,
                direction: None,
                cases: [N, S, N, P, S, N],
            },
        ],
    },
];

/// The kind the document that holds `node` declares (`sce:kind`), if any.
fn kind_of<'a>(node: &roxmltree::Node<'a, '_>) -> Option<&'a str> {
    node.document()
        .root_element()
        .attribute((crate::forge::model::SCE_NAMESPACE, "kind"))
}

/// Whether `node`'s attribute `attr` declares a name of `slot`'s kind: the
/// one predicate the rule and the test that holds the table to the templates
/// both ask, so that what is compared is what is measured.
pub fn declares(slot: &Slot, node: &roxmltree::Node, attr: &str, dialect: Dialect) -> bool {
    let named = slot.rows.iter().find(|row| {
        node.tag_name().namespace() == Some(row.ns.uri())
            && node.tag_name().name() == row.element
            && attr == row.attr
    });
    let Some(row) = named else {
        return false;
    };
    // A statechart's `<data id>` is W3C's name for a data-model location, and
    // its reader is escaped instead (`reader_names`).
    if row.ns == Ns::Scxml && row.element == "data" && dialect != Dialect::Forge {
        return false;
    }
    // Inside `<sce:peek-byte>` a flag is only a mask lookup.
    if row.ns == Ns::Sce
        && row.element == "flag"
        && node
            .parent_element()
            .is_some_and(|p| p.tag_name().name() == "peek-byte")
    {
        return false;
    }
    if !slot.kinds.is_empty() && !kind_of(node).is_some_and(|k| slot.kinds.contains(&k)) {
        return false;
    }
    match slot.direction {
        Some(direction) => {
            node.attribute((crate::forge::model::SCE_NAMESPACE, "direction")) == Some(direction)
        }
        None => true,
    }
}

/// What the first declaration of a spelling is, so that the second can be
/// compared with it.
struct Earlier {
    value: String,
    scope: usize,
    slot: usize,
}

/// Refuse the first declaration, in document order, that a backend spells
/// as an earlier one of the same scope.
///
/// `root` is the post-expansion tree, as for
/// [`crate::scxml_identifier::reject_malformed`], and the refusal sits on the
/// later declaration's own attribute value, which is what
/// `SCE_ERROR_CONTRACT.md` §3.1.1 requires of `actual`.
pub fn reject_colliding(
    root: &roxmltree::Node,
    doc_name: &str,
    dialect: Dialect,
) -> Result<(), Located<ForgeError>> {
    // (scope, backend, spelling) -> the declaration that made it first.
    let mut seen: HashMap<(usize, usize, String), Earlier> = HashMap::new();

    for node in root.descendants() {
        if !node.is_element() {
            continue;
        }
        for attribute in node.attributes() {
            if attribute.namespace().is_some() {
                continue;
            }
            for (scope_index, scope) in SCOPES.iter().enumerate() {
                for (slot_index, slot) in scope.slots.iter().enumerate() {
                    if !declares(slot, &node, attribute.name(), dialect) {
                        continue;
                    }
                    let value = attribute.value();
                    for (backend, case) in slot.cases.iter().enumerate() {
                        let Some(case) = case else { continue };
                        let key = (scope_index, backend, case.spell(value));
                        match seen.get(&key) {
                            Some(earlier) if earlier.value != value => {
                                let spellings = shared_spellings(earlier, value, slot_index);
                                let document = node.document();
                                let pos = document.text_pos_at(attribute.range_value().start);
                                let written =
                                    document.input_text()[attribute.range_value()].to_string();
                                let other_slot = &SCOPES[earlier.scope].slots[earlier.slot];
                                return Err(Located::new(
                                    ValidationError::CollidingCodeIdentifier {
                                        element: crate::scxml_identifier::spelled_tag(&node),
                                        attr: attribute.name().to_string(),
                                        value: written,
                                        noun: slot.noun,
                                        other: earlier.value.clone(),
                                        other_noun: other_slot.noun,
                                        spellings,
                                    }
                                    .into(),
                                    doc_name,
                                    Some(pos.row),
                                    Some(pos.col),
                                ));
                            }
                            Some(_) => {}
                            None => {
                                seen.insert(
                                    key,
                                    Earlier {
                                        value: value.to_string(),
                                        scope: scope_index,
                                        slot: slot_index,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Every backend that spells `value` as `earlier` — the first of them is
/// the one the refusal was found through — with the spelling they share, in
/// [`Language::ALL`] order. The repair, renaming one of the two, is the same
/// whichever backend an author builds, so a message that named one backend
/// would let them assume another is safe.
fn shared_spellings(
    earlier: &Earlier,
    value: &str,
    slot_index: usize,
) -> Vec<(&'static str, String)> {
    let scope = &SCOPES[earlier.scope];
    let this = &scope.slots[slot_index];
    let that = &scope.slots[earlier.slot];
    Language::ALL
        .iter()
        .enumerate()
        .filter_map(|(backend, language)| {
            let spelled = this.cases[backend]?.spell(value);
            (that.cases[backend]?.spell(&earlier.value) == spelled)
                .then(|| (language.canonical_name(), spelled))
        })
        .collect()
}

/// [`ValidationError::CollidingCodeIdentifier`]'s message.
///
/// Names the first backend's spelling and then every other backend that
/// folds the pair, because the repair is the same whichever backend an
/// author builds.
pub fn colliding_code_identifier_message(
    element: &str,
    attr: &str,
    value: &str,
    noun: &str,
    other: &str,
    other_noun: &str,
    spellings: &[(&'static str, String)],
) -> String {
    let Some(((language, spelled), rest)) = spellings.split_first() else {
        return format!(
            "<{element} {attr}=\"{value}\">: {noun} '{value}' and {other_noun} '{other}' \
             collide in the generated code — rename one of them"
        );
    };
    let others = match rest {
        [] => String::new(),
        [(l, s)] => format!(" (so does {l} '{s}')"),
        [init @ .., (l, s)] => {
            let listed: Vec<String> = init.iter().map(|(l, s)| format!("{l} '{s}'")).collect();
            format!(" (so do {} and {l} '{s}')", listed.join(", "))
        }
    };
    format!(
        "<{element} {attr}=\"{value}\">: {noun} '{value}' and {other_noun} '{other}' are two \
         names of this document, but the generated {language} code spells both '{spelled}'\
         {others}, so it would declare one name twice — rename one of them"
    )
}
