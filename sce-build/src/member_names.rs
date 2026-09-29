// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-3.14: the one place a statechart's own names become members of the
//! code generated for it, and the rule that no two of them become the same
//! member.
//!
//! Ids are unique in a document and XML Names are case-sensitive, so `idle`
//! and `Idle` are two states, and `doorOpen` and `door_open` are two more.
//! Every backend then spells a name in its own convention — C++ capitalises
//! the first letter, Rust, Go and Kotlin write PascalCase, Python writes
//! UPPER_SNAKE, C11 upper-cases under a machine prefix — and a convention
//! that folds two names together declares one enumerator, variant or
//! constant twice. Measured 2026-09-29, before this rule: `idle` beside
//! `Idle` passed `check --lint` and `generate` with exit status 0 in all six
//! backends, and every one of them declared the member twice (C++ wrote
//! `enum class State : uint8_t { Idle, Idle };`, and the Python module did not
//! import). `doorOpen` beside `door_open` did the same in Rust, Go, Python and
//! Kotlin. A document SCE accepts must not generate code that does not build.
//!
//! **The spelling is the templates' own.** Every function a family below
//! calls is the filter the template for that family calls, so what this
//! module compares and what the output contains are the same strings. The
//! members a template declares whatever the document says — the sentinels —
//! are listed with their family, and `sce-build/tests/generated_names.rs`
//! renders documents through every backend and reads the members back, so a
//! template that gains or renames one fails there rather than here.
//!
//! **Refused for every backend at once**, as a reserved code identifier is
//! (`docs/SCE_ACCEPTED_SUBSET.md` §2.14.1): a document can be generated in
//! any of the six, and whether SCE accepts it must not depend on which one a
//! deployment happens to build.

use std::borrow::Cow;
use std::collections::BTreeMap;

use serde_json::Value;

use crate::analyzer::WILDCARD_EVENT;
use crate::filters;
use crate::generator::Language;
use crate::model::SCXMLModel;

/// Which of a document's names a collision is between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Enumeration {
    /// `<state>`, `<parallel>` and `<final>` ids.
    State,
    /// `<history>` ids.
    History,
    /// Event names, as the document's transitions and sends use them.
    Event,
}

impl Enumeration {
    /// The plural a message names the two with.
    pub fn plural(self) -> &'static str {
        match self {
            Enumeration::State => "states",
            Enumeration::History => "history states",
            Enumeration::Event => "event names",
        }
    }

    /// The singular a message names one with.
    pub fn singular(self) -> &'static str {
        match self {
            Enumeration::State => "state",
            Enumeration::History => "history state",
            Enumeration::Event => "event name",
        }
    }
}

/// What the reported name collides with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clash {
    /// Another name of the document.
    Name(String),
    /// A member the generated code declares for its own use.
    Generated,
}

/// One pair of names that spell one member, and every backend that does so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collision {
    pub enumeration: Enumeration,
    /// The name the report is placed on.
    pub name: String,
    pub clash: Clash,
    /// `(backend, spelling)` for every backend that folds the pair, in
    /// [`Language::ALL`] order.
    pub spellings: Vec<(&'static str, String)>,
}

/// Which of the model's names a family declares a symbol for.
#[derive(Clone, Copy)]
enum Source {
    States,
    /// States with at least one `<onentry>` — C11 names a function per block.
    StatesWithEntry,
    /// States with at least one `<onexit>`.
    StatesWithExit,
    Histories,
    Events,
}

impl Source {
    fn enumeration(self) -> Enumeration {
        match self {
            Source::States | Source::StatesWithEntry | Source::StatesWithExit => Enumeration::State,
            Source::Histories => Enumeration::History,
            Source::Events => Enumeration::Event,
        }
    }

    fn names(self, model: &SCXMLModel) -> Vec<&str> {
        match self {
            Source::States => model.states.keys().map(String::as_str).collect(),
            Source::StatesWithEntry => model
                .states
                .values()
                .filter(|s| !s.on_entry_blocks.is_empty())
                .map(|s| s.id.as_str())
                .collect(),
            Source::StatesWithExit => model
                .states
                .values()
                .filter(|s| !s.on_exit_blocks.is_empty())
                .map(|s| s.id.as_str())
                .collect(),
            Source::Histories => model.history_states.keys().map(String::as_str).collect(),
            Source::Events => model.events.iter().map(String::as_str).collect(),
        }
    }
}

/// One set of symbols one backend declares from one kind of name.
struct Family {
    language: Language,
    /// Which set: `State`, `History`, `Event`, or C11's `on_entry` /
    /// `on_exit` block functions.
    label: &'static str,
    source: Source,
    /// The member a name becomes; `None` for a name the template leaves out.
    spell: fn(&str) -> Option<String>,
    /// Members the template declares whatever the document says.
    sentinels: &'static [&'static str],
}

fn cpp_state(name: &str) -> Option<String> {
    Some(filters::capitalize_state(name.to_string()))
}

fn cpp_event(name: &str) -> Option<String> {
    Some(filters::to_cpp_event_variant(Cow::Borrowed(name)))
}

fn pascal_state(name: &str) -> Option<String> {
    Some(filters::to_state_variant(name.to_string()))
}

fn kotlin_state(name: &str) -> Option<String> {
    Some(filters::to_state_class_name(name.to_string()))
}

/// Rust and Go skip the wildcard entry and give eventless dispatch a member
/// of their own.
fn pascal_event(name: &str) -> Option<String> {
    (name != WILDCARD_EVENT).then(|| filters::to_event_variant(name.to_string()))
}

fn python_state(name: &str) -> Option<String> {
    Some(filters::to_python_const(name.to_string()))
}

fn python_event(name: &str) -> Option<String> {
    (name != WILDCARD_EVENT).then(|| filters::to_python_const(name.to_string()))
}

fn c11_upper(name: &str) -> Option<String> {
    Some(filters::to_c11_upper_ident(Cow::Borrowed(name)))
}

fn c11_lower(name: &str) -> Option<String> {
    Some(filters::to_c11_lower_ident(Cow::Borrowed(name)))
}

/// Every family, grouped by backend in [`Language::ALL`] order. Kotlin's
/// events are not here: they are a tree of nested types rather than one
/// enumeration, and [`kotlin_event_collision`] walks it.
const FAMILIES: &[Family] = &[
    // Rust: `<M>State`, `<M>History`, `<M>Event` enums.
    Family {
        language: Language::Rust,
        label: "State",
        source: Source::States,
        spell: pascal_state,
        sentinels: &[],
    },
    Family {
        language: Language::Rust,
        label: "History",
        source: Source::Histories,
        spell: pascal_state,
        sentinels: &[],
    },
    Family {
        language: Language::Rust,
        label: "Event",
        source: Source::Events,
        spell: pascal_event,
        sentinels: &["Null"],
    },
    // C++: `State`, `History`, `Event` enum classes.
    Family {
        language: Language::Cpp,
        label: "State",
        source: Source::States,
        spell: cpp_state,
        sentinels: &[],
    },
    Family {
        language: Language::Cpp,
        label: "History",
        source: Source::Histories,
        spell: cpp_state,
        sentinels: &[],
    },
    Family {
        language: Language::Cpp,
        label: "Event",
        source: Source::Events,
        spell: cpp_event,
        sentinels: &["NONE"],
    },
    // Kotlin: `data object`s of the state interface, `history<X>` ids.
    Family {
        language: Language::Kotlin,
        label: "State",
        source: Source::States,
        spell: kotlin_state,
        sentinels: &[],
    },
    Family {
        language: Language::Kotlin,
        label: "History",
        source: Source::Histories,
        spell: kotlin_state,
        sentinels: &[],
    },
    // Go: `<M>State<X>`, `<M>History<X>`, `<M>Event<X>` constants.
    Family {
        language: Language::Go,
        label: "State",
        source: Source::States,
        spell: pascal_state,
        sentinels: &[],
    },
    Family {
        language: Language::Go,
        label: "History",
        source: Source::Histories,
        spell: pascal_state,
        sentinels: &[],
    },
    Family {
        language: Language::Go,
        label: "Event",
        source: Source::Events,
        spell: pascal_event,
        sentinels: &["Null"],
    },
    // Python: `IntEnum` members; a history is only ever a string literal.
    Family {
        language: Language::Python,
        label: "State",
        source: Source::States,
        spell: python_state,
        sentinels: &[],
    },
    Family {
        language: Language::Python,
        label: "Event",
        source: Source::Events,
        spell: python_event,
        sentinels: &["NULL"],
    },
    // C11: `_STATE_<X>`, `_HIST_<X>`, `_EVENT_<X>` enumerators, and one
    // function per `<onentry>` / `<onexit>` block, named after its state.
    Family {
        language: Language::C11,
        label: "State",
        source: Source::States,
        spell: c11_upper,
        sentinels: &["COUNT"],
    },
    Family {
        language: Language::C11,
        label: "on_entry",
        source: Source::StatesWithEntry,
        spell: c11_lower,
        sentinels: &[],
    },
    Family {
        language: Language::C11,
        label: "on_exit",
        source: Source::StatesWithExit,
        spell: c11_lower,
        sentinels: &[],
    },
    Family {
        language: Language::C11,
        label: "History",
        source: Source::Histories,
        spell: c11_upper,
        sentinels: &["NONE"],
    },
    Family {
        language: Language::C11,
        label: "Event",
        source: Source::Events,
        spell: c11_upper,
        sentinels: &["NONE"],
    },
];

/// One set of symbols, as this module expects a backend to declare it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub language: Language,
    /// `State`, `History`, `Event`, or C11's `on_entry` / `on_exit`.
    pub label: &'static str,
    /// Each name's spelling, and the members the template adds itself.
    pub members: std::collections::BTreeSet<String>,
    /// The members the template adds itself, alone.
    pub sentinels: std::collections::BTreeSet<String>,
}

/// What every backend's code for `model` declares, as the refusal above
/// reasons about it. Not used to generate anything: it is the claim
/// `sce-build/tests/generated_names.rs` holds against the rendered code, so
/// a template that spells a member some other way than this module does, or
/// adds a sentinel this module does not know, fails there.
pub fn declared(model: &SCXMLModel) -> Vec<Declared> {
    FAMILIES
        .iter()
        .map(|family| {
            let sentinels: std::collections::BTreeSet<String> =
                family.sentinels.iter().map(|s| s.to_string()).collect();
            let mut members = sentinels.clone();
            members.extend(
                family
                    .source
                    .names(model)
                    .into_iter()
                    .filter_map(family.spell),
            );
            Declared {
                language: family.language,
                label: family.label,
                members,
                sentinels,
            }
        })
        .collect()
}

/// The first pair of names, in the order the report walks them, that two
/// members of the generated code would share.
///
/// States come before history states and events, and within an enumeration
/// the backends in [`Language::ALL`] order, so a document with several
/// collisions reports the same one on every run. The pair found first is
/// then asked of every family, so the report names each backend that folds
/// it rather than the first one alone.
pub fn first_collision(model: &SCXMLModel) -> Option<Collision> {
    for enumeration in [Enumeration::State, Enumeration::History, Enumeration::Event] {
        let mut found: Option<(String, Clash)> = None;
        for family in FAMILIES
            .iter()
            .filter(|f| f.source.enumeration() == enumeration)
        {
            if let Some(pair) = family_collision(family, model) {
                found = Some(pair);
                break;
            }
        }
        if found.is_none() && enumeration == Enumeration::Event {
            found = kotlin_event_collisions(model)
                .into_iter()
                .next()
                .map(|(name, clash, _)| (name, clash));
        }
        if let Some((name, clash)) = found {
            let spellings = spellings_of(enumeration, &name, &clash, model);
            return Some(Collision {
                enumeration,
                name,
                clash,
                spellings,
            });
        }
    }
    None
}

/// The first name in `family` that spells a member already taken.
fn family_collision(family: &Family, model: &SCXMLModel) -> Option<(String, Clash)> {
    let mut taken: BTreeMap<String, Option<&str>> = family
        .sentinels
        .iter()
        .map(|s| (s.to_string(), None))
        .collect();
    let mut names = family.source.names(model);
    names.sort_unstable();
    for name in names {
        let Some(member) = (family.spell)(name) else {
            continue;
        };
        match taken.get(&member) {
            Some(Some(other)) => return Some((name.to_string(), Clash::Name(other.to_string()))),
            Some(None) => return Some((name.to_string(), Clash::Generated)),
            None => {
                taken.insert(member, Some(name));
            }
        }
    }
    None
}

/// Every backend that folds this pair, with its spelling.
fn spellings_of(
    enumeration: Enumeration,
    name: &str,
    clash: &Clash,
    model: &SCXMLModel,
) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for language in Language::ALL {
        let mut spelled = None;
        for family in FAMILIES
            .iter()
            .filter(|f| f.language == *language && f.source.enumeration() == enumeration)
        {
            let in_family = |n: &str| family.source.names(model).contains(&n);
            let Some(member) = (family.spell)(name).filter(|_| in_family(name)) else {
                continue;
            };
            let folds = match clash {
                Clash::Name(other) => {
                    in_family(other) && (family.spell)(other).as_ref() == Some(&member)
                }
                Clash::Generated => family.sentinels.contains(&member.as_str()),
            };
            if folds {
                spelled = Some(member);
                break;
            }
        }
        if spelled.is_none() && *language == Language::Kotlin && enumeration == Enumeration::Event {
            spelled = kotlin_event_spelling(model, name, clash);
        }
        if let Some(member) = spelled {
            out.push((language.canonical_name(), member));
        }
    }
    out
}

/// Whether Kotlin folds this pair, and how it spells it.
///
/// Two event names Kotlin refers to by one nested path (`to_event_class_name`)
/// are one type to it. A collision the tree walk found between two tokens
/// under one parent may name prefixes rather than whole events, so that walk
/// is asked too.
fn kotlin_event_spelling(model: &SCXMLModel, name: &str, clash: &Clash) -> Option<String> {
    if let Clash::Name(other) = clash {
        let events = |n: &str| n != WILDCARD_EVENT && model.events.contains(n);
        if events(name) && events(other) {
            let spelled = filters::to_event_class_name(name.to_string());
            if spelled == filters::to_event_class_name(other.clone()) {
                return Some(spelled);
            }
        }
    }
    kotlin_event_collisions(model)
        .into_iter()
        .find(|(found, found_clash, _)| found == name && found_clash == clash)
        .map(|(_, _, member)| member)
}

/// Kotlin declares one nested type per dot-separated token of an event name
/// (`door.open` is `Door.Open`, [`crate::kotlin::render_event_tree`]), so two
/// names collide when two tokens under one parent spell one type, or when a
/// token spells the `Self` member an event that is also a prefix declares.
/// Each entry is the reported path, what it collides with, and the spelling,
/// in the order the tree is walked.
fn kotlin_event_collisions(model: &SCXMLModel) -> Vec<(String, Clash, String)> {
    let events: std::collections::BTreeSet<String> = model
        .events
        .iter()
        .filter(|e| e.as_str() != WILDCARD_EVENT)
        .cloned()
        .collect();
    let tree = crate::kotlin::build_event_tree(&events);
    let mut out = Vec::new();
    kotlin_tree_collisions(&tree, "", &mut out);
    out
}

fn kotlin_tree_collisions(node: &Value, prefix: &str, out: &mut Vec<(String, Clash, String)>) {
    let Some(object) = node.as_object() else {
        return;
    };
    let is_leaf = object
        .get("_leaf")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut keys: Vec<&String> = object.keys().filter(|k| k.as_str() != "_leaf").collect();
    keys.sort_unstable();
    let path = |key: &str| {
        if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        }
    };
    let mut taken: BTreeMap<String, Option<String>> = BTreeMap::new();
    if is_leaf && !keys.is_empty() {
        taken.insert(crate::kotlin::EVENT_TREE_SELF_MEMBER.to_string(), None);
    }
    for key in &keys {
        let member = crate::kotlin::event_tree_class_name(key);
        match taken.get(&member) {
            Some(Some(other)) => out.push((path(key), Clash::Name(other.clone()), member)),
            Some(None) => out.push((path(key), Clash::Generated, member)),
            None => {
                taken.insert(member, Some(path(key)));
            }
        }
    }
    for key in &keys {
        kotlin_tree_collisions(&object[key.as_str()], &path(key), out);
    }
}

/// Refuse the document when two of its names would be one member of the
/// generated code. Called from [`crate::analyzer::can_generate_static`], the
/// gate both the library and every `sce-codegen` subcommand share.
pub fn validate(
    model: &SCXMLModel,
    diag_label: &str,
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    use crate::forge::error::ForgeError;
    use crate::scxml_semantic::ScxmlSemanticError;

    let Some(collision) = first_collision(model) else {
        return Ok(());
    };
    let at = match collision.enumeration {
        Enumeration::State => model
            .states
            .get(&collision.name)
            .and_then(|s| s.source_location.as_ref()),
        Enumeration::History => model
            .history_states
            .get(&collision.name)
            .and_then(|h| h.source_location.as_ref()),
        // An event name has no one declaration; the document owns it.
        Enumeration::Event => None,
    }
    .or(model.source_location.as_ref());
    let err = ForgeError::Scxml(Box::new(ScxmlSemanticError::GeneratedNameCollision {
        enumeration: collision.enumeration,
        name: collision.name,
        clash: collision.clash,
        spellings: collision.spellings,
    }));
    Err(model.locate(err, at, diag_label))
}
