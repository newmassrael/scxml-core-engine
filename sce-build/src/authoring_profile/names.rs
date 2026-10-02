// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `names` setting of an authoring profile: how the names a document
//! DEFINES are spelled.
//!
//! # Why a spelling is a setting
//!
//! Which term in a specification a name stands for is a reading of prose, and
//! nothing here judges prose. How the name is SPELLED is a fact about the text:
//! measured 2026-09-29, the only difference between two drafts of a forge case
//! was the document's `name`, and five drafts of one connection specification
//! spelled the machine's own event names three ways. A spelling is the part of
//! a naming preference a machine can hold every later draft to.
//!
//! # Which names
//!
//! Four classes, the ones a document introduces: its `name`, the ids of its
//! states, the events it raises, sends or takes by a literal name, and the ids
//! of its `<data>`. Nothing a document does not choose is judged:
//!
//! - an event an imported event-schema declares is a fact about the platform
//!   the schema describes, and is taken verbatim wherever the style would have
//!   spelled it otherwise;
//! - the platform's own events (`error.*`, `done.state.*`, `done.invoke.*`) and
//!   any name that begins with an underscore belong to the platform;
//! - a descriptor that is a pattern (`door.*`) matches events and declares
//!   none, so it is not a name (§scxml-3.12.1, [`crate::event_descriptor`]).
//!
//! # What a style is
//!
//! ASCII only, and applied to one token: a state id, a data id, the document
//! name, or each dot-separated token of an event name. A capital letter in
//! camel and Pascal case starts a word and is followed by a lower-case letter
//! or a digit, so an acronym is spelled as a word. That is stricter than a
//! reader might expect and it is what makes the rule decidable: `doorHTTP`
//! could be read as one word or two, `doorHttp` cannot.
//!
//! # Prefix-free events
//!
//! §scxml-3.12.1 matches an event descriptor by token prefix, so a
//! transition on `door` also takes `door.open`. The `prefix_free` rule is a
//! correctness rule in the form of a naming rule: no literal event name of the
//! document, and no name its event-schemas declare, is a token prefix of
//! another.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::event_descriptor::{literal_event, EventDescriptor};
use crate::forge::error::SourceLocation;
use crate::model::SCXMLModel;

/// A spelling convention for one token of a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// `door_open`.
    Snake,
    /// `DOOR_OPEN`.
    UpperSnake,
    /// `doorOpen`.
    Camel,
    /// `DoorOpen`.
    Pascal,
    /// `door-open`.
    Kebab,
    /// `dooropen`.
    Lower,
}

impl Style {
    /// How the style is written in a sentence.
    pub const fn spelled(self) -> &'static str {
        match self {
            Style::Snake => "snake_case",
            Style::UpperSnake => "UPPER_SNAKE_CASE",
            Style::Camel => "camelCase",
            Style::Pascal => "PascalCase",
            Style::Kebab => "kebab-case",
            Style::Lower => "lowercase",
        }
    }

    /// The wire spelling, as the schema names it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Style::Snake => "snake",
            Style::UpperSnake => "upper_snake",
            Style::Camel => "camel",
            Style::Pascal => "pascal",
            Style::Kebab => "kebab",
            Style::Lower => "lower",
        }
    }

    /// Whether `token` is spelled in this style.
    pub fn accepts(self, token: &str) -> bool {
        match self {
            Style::Lower => {
                starts_with(token, |c| c.is_ascii_lowercase()) && token.chars().all(lower_digit)
            }
            Style::Snake => {
                starts_with(token, |c| c.is_ascii_lowercase()) && separated(token, '_', lower_digit)
            }
            Style::Kebab => {
                starts_with(token, |c| c.is_ascii_lowercase()) && separated(token, '-', lower_digit)
            }
            Style::UpperSnake => {
                starts_with(token, |c| c.is_ascii_uppercase()) && separated(token, '_', upper_digit)
            }
            Style::Camel => starts_with(token, |c| c.is_ascii_lowercase()) && capitalised(token),
            Style::Pascal => starts_with(token, |c| c.is_ascii_uppercase()) && capitalised(token),
        }
    }

    /// `words` spelled in this style.
    pub fn spell(self, words: &[String]) -> String {
        let capital = |word: &str| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        };
        match self {
            Style::Snake => words.join("_"),
            Style::UpperSnake => words
                .iter()
                .map(|w| w.to_ascii_uppercase())
                .collect::<Vec<_>>()
                .join("_"),
            Style::Kebab => words.join("-"),
            Style::Lower => words.concat(),
            Style::Pascal => words.iter().map(|w| capital(w)).collect(),
            Style::Camel => words
                .iter()
                .enumerate()
                .map(|(at, w)| if at == 0 { w.clone() } else { capital(w) })
                .collect(),
        }
    }

    /// `token` respelled in this style, when the respelling is one the style
    /// accepts. `None` for a token with no ASCII word in it, or one whose
    /// first word begins with a digit, which no style here accepts.
    pub fn respell(self, token: &str) -> Option<String> {
        let spelled = self.spell(&words_of(token));
        (!spelled.is_empty() && self.accepts(&spelled)).then_some(spelled)
    }
}

fn starts_with(token: &str, first: impl Fn(char) -> bool) -> bool {
    token.chars().next().is_some_and(first)
}

fn lower_digit(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

fn upper_digit(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit()
}

/// `token` is words made of `word` characters, joined by single `separator`s.
fn separated(token: &str, separator: char, word: impl Fn(char) -> bool + Copy) -> bool {
    token
        .split(separator)
        .all(|part| !part.is_empty() && part.chars().all(word))
}

/// Camel and Pascal case: a run of lower-case letters and digits, then any
/// number of words each a capital followed by at least one lower-case letter
/// or digit. The leading run of a Pascal token is empty, and its first capital
/// begins the first word.
fn capitalised(token: &str) -> bool {
    let mut chars = token.chars().peekable();
    while chars.peek().is_some_and(|c| lower_digit(*c)) {
        chars.next();
    }
    while let Some(capital) = chars.next() {
        if !capital.is_ascii_uppercase() || !chars.peek().is_some_and(|c| lower_digit(*c)) {
            return false;
        }
        while chars.peek().is_some_and(|c| lower_digit(*c)) {
            chars.next();
        }
    }
    true
}

/// The words `token` is made of, lower-cased: split at every character that is
/// not an ASCII letter or digit, where a lower-case letter or a digit meets a
/// capital, and inside a run of capitals before the last one that begins a
/// lower-case word (`HTTPServer` is `http` and `server`).
pub fn words_of(token: &str) -> Vec<String> {
    let chars: Vec<char> = token.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (at, &c) in chars.iter().enumerate() {
        if !c.is_ascii_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let boundary = at > 0
            && !current.is_empty()
            && c.is_ascii_uppercase()
            && (lower_digit(chars[at - 1])
                || (chars[at - 1].is_ascii_uppercase()
                    && chars.get(at + 1).is_some_and(|n| n.is_ascii_lowercase())));
        if boundary {
            words.push(std::mem::take(&mut current));
        }
        current.push(c.to_ascii_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// What a rule about a name holds it to, besides its style.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameRule {
    pub style: Option<Style>,
    pub max_length: Option<usize>,
    pub forbidden_words: Option<Vec<String>>,
    pub required_prefix: Option<String>,
}

/// How many dot-separated tokens an event name has, both ends inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenRange {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

/// A [`NameRule`] for event names, with the structure of one.
///
/// ⚠ The fields of [`NameRule`] are written out again, not flattened in:
/// serde refuses `deny_unknown_fields` together with `flatten`, and the
/// refusal of a setting this build does not know is the point of the former.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventRule {
    pub style: Option<Style>,
    pub max_length: Option<usize>,
    pub forbidden_words: Option<Vec<String>>,
    pub required_prefix: Option<String>,
    pub tokens: Option<TokenRange>,
    pub first_tokens: Option<Vec<String>>,
    pub prefix_free: Option<bool>,
}

impl EventRule {
    fn name_rule(&self) -> NameRule {
        NameRule {
            style: self.style,
            max_length: self.max_length,
            forbidden_words: self.forbidden_words.clone(),
            required_prefix: self.required_prefix.clone(),
        }
    }
}

/// The `names` setting: one rule per class of name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamesRule {
    pub document: Option<NameRule>,
    pub state: Option<NameRule>,
    pub event: Option<EventRule>,
    pub data: Option<NameRule>,
}

impl NamesRule {
    /// Why this setting is not one the schema accepts, when it is not: the
    /// rules `serde` cannot state. The schema says the same with
    /// `minProperties`, `minItems` and `pattern`, and a test holds the two to
    /// one answer.
    pub fn refusal(&self) -> Option<String> {
        let rules: [(&str, Option<&NameRule>); 3] = [
            ("document", self.document.as_ref()),
            ("state", self.state.as_ref()),
            ("data", self.data.as_ref()),
        ];
        if self.document.is_none()
            && self.state.is_none()
            && self.event.is_none()
            && self.data.is_none()
        {
            return Some("`names` names no class of name".to_string());
        }
        for (class, rule) in rules {
            if let Some(rule) = rule {
                if *rule == NameRule::default() {
                    return Some(format!("`names.{class}` states no rule"));
                }
                if let Some(fault) = name_rule_refusal(class, rule) {
                    return Some(fault);
                }
            }
        }
        let event = self.event.as_ref()?;
        if *event == EventRule::default() {
            return Some("`names.event` states no rule".to_string());
        }
        if let Some(fault) = name_rule_refusal("event", &event.name_rule()) {
            return Some(fault);
        }
        if let Some(range) = event.tokens {
            if range.min.is_none() && range.max.is_none() {
                return Some("`names.event.tokens` gives neither `min` nor `max`".to_string());
            }
            if range.min == Some(0) || range.max == Some(0) {
                return Some("`names.event.tokens` counts from 1".to_string());
            }
            if let (Some(min), Some(max)) = (range.min, range.max) {
                if min > max {
                    return Some(format!(
                        "`names.event.tokens` asks for at least {min} and at most {max}"
                    ));
                }
            }
        }
        if let Some(first) = &event.first_tokens {
            if first.is_empty() || first.iter().any(String::is_empty) {
                return Some("`names.event.first_tokens` lists a token or none".to_string());
            }
            let distinct: BTreeSet<&String> = first.iter().collect();
            if distinct.len() != first.len() {
                return Some("`names.event.first_tokens` lists a token twice".to_string());
            }
        }
        None
    }

    /// Whether the setting constrains the document's events as a set.
    fn wants_prefix_free(&self) -> bool {
        self.event.as_ref().and_then(|e| e.prefix_free) == Some(true)
    }
}

fn name_rule_refusal(class: &str, rule: &NameRule) -> Option<String> {
    if rule.max_length == Some(0) {
        return Some(format!("`names.{class}.max_length` counts from 1"));
    }
    if let Some(words) = &rule.forbidden_words {
        if words.is_empty() {
            return Some(format!("`names.{class}.forbidden_words` lists no word"));
        }
        if let Some(bad) = words
            .iter()
            .find(|w| w.is_empty() || !w.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            return Some(format!(
                "`names.{class}.forbidden_words` holds {bad:?}, and a word is letters and digits"
            ));
        }
        let distinct: BTreeSet<String> = words.iter().map(|w| w.to_ascii_lowercase()).collect();
        if distinct.len() != words.len() {
            return Some(format!(
                "`names.{class}.forbidden_words` lists a word twice"
            ));
        }
    }
    if rule.required_prefix.as_deref() == Some("") {
        return Some(format!("`names.{class}.required_prefix` is empty"));
    }
    None
}

/// A class of name a document defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameClass {
    /// The `name` of the `<scxml>` root.
    Document,
    /// The id of a state, parallel, final or history.
    State,
    /// An event the document raises, sends or takes by a literal name.
    Event,
    /// A `<data>` id.
    Data,
}

impl NameClass {
    /// The class as the schema names it, and as a record keys on it.
    pub const fn as_str(self) -> &'static str {
        match self {
            NameClass::Document => "document",
            NameClass::State => "state",
            NameClass::Event => "event",
            NameClass::Data => "data",
        }
    }

    /// One of the class, in a sentence.
    pub const fn singular(self) -> &'static str {
        match self {
            NameClass::Document => "document name",
            NameClass::State => "state id",
            NameClass::Event => "event name",
            NameClass::Data => "data id",
        }
    }

    /// The class in the plural, in a sentence.
    pub const fn plural(self) -> &'static str {
        match self {
            NameClass::Document => "document names",
            NameClass::State => "state ids",
            NameClass::Event => "event names",
            NameClass::Data => "data ids",
        }
    }
}

/// What a name of a class is held to besides its style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameLimit {
    /// It is made of a word the profile forbids; every such word is listed.
    ForbiddenWords(Vec<String>),
    /// It is longer than the profile allows.
    TooLong { max: usize, actual: usize },
    /// It does not begin with what the profile requires.
    MissingPrefix(String),
}

impl NameLimit {
    /// Which limit, for a record's key.
    pub const fn kind(&self) -> &'static str {
        match self {
            NameLimit::ForbiddenWords(_) => "forbidden-word",
            NameLimit::TooLong { .. } => "length",
            NameLimit::MissingPrefix(_) => "prefix",
        }
    }
}

/// What is wrong with the structure of an event name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventProblem {
    /// Fewer dot-separated tokens than the profile asks for.
    TooFewTokens { min: usize, actual: usize },
    /// More dot-separated tokens than the profile allows.
    TooManyTokens { max: usize, actual: usize },
    /// It begins with a token the profile does not list.
    FirstToken {
        allowed: Vec<String>,
        actual: String,
    },
}

impl EventProblem {
    /// Which problem, for a record's key.
    pub const fn kind(&self) -> &'static str {
        match self {
            EventProblem::TooFewTokens { .. } => "tokens-min",
            EventProblem::TooManyTokens { .. } => "tokens-max",
            EventProblem::FirstToken { .. } => "first-token",
        }
    }
}

/// One departure from the `names` setting, before it is placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Departure {
    /// A name is not spelled in the style its class asks for. `part` is the
    /// token that is not — the whole name unless the class is `event`.
    Style {
        class: NameClass,
        name: String,
        part: String,
        style: Style,
        respelled: Option<String>,
    },
    /// A name breaks a limit of its class.
    Limit {
        class: NameClass,
        name: String,
        limit: NameLimit,
    },
    /// The structure of an event name is not what the profile asks for.
    Event { name: String, problem: EventProblem },
    /// One event name is a token prefix of another.
    PrefixOfAnother { prefix: String, longer: String },
}

/// A departure and where the document writes the name it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub departure: Departure,
    pub at: Option<SourceLocation>,
}

/// A name a document defines, with where it first writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Occurrence {
    name: String,
    at: Option<SourceLocation>,
}

/// Every departure of `model` from `rule`, in the order a reader finds them:
/// by class, then by the row the name is first written on.
pub fn judge(rule: &NamesRule, model: &SCXMLModel) -> Vec<Found> {
    let mut found = Vec::new();
    if let Some(document) = &rule.document {
        // A root with no `name` defines no name.
        if !model.scxml_name.is_empty() {
            let occurrence = Occurrence {
                name: model.scxml_name.clone(),
                at: model.source_location.clone(),
            };
            apply(document, NameClass::Document, &occurrence, &mut found);
        }
    }
    if let Some(state) = &rule.state {
        for occurrence in state_names(model) {
            apply(state, NameClass::State, &occurrence, &mut found);
        }
    }
    let events = event_names(model);
    if let Some(event) = &rule.event {
        for occurrence in &events {
            apply(&event.name_rule(), NameClass::Event, occurrence, &mut found);
            structure(event, occurrence, &mut found);
        }
    }
    if rule.wants_prefix_free() {
        prefix_free(model, &events, &mut found);
    }
    if let Some(data) = &rule.data {
        for occurrence in data_names(model) {
            apply(data, NameClass::Data, &occurrence, &mut found);
        }
    }
    found
}

/// Every departure of an event-schema document from `rule`: the event it
/// declares, and the ids of the fields its payload carries.
///
/// The event a schema declares is a name the OWNER's document defines, and it
/// is here that a boundary event is spelled once: a statechart that imports
/// the schema takes the name from it and is not judged for it
/// ([`event_names`]), so a rule that reached only the statechart would leave
/// the names an interface is made of, and where they part between drafts,
/// outside the profile.
///
/// `event_name` is what the document writes in `sce:event-name`, and `fields`
/// the ids of its `<data>`. Neither carries a row: the model keeps none for
/// them, so a finding is placed on the document.
pub fn judge_event_schema(rule: &NamesRule, event_name: &str, fields: &[String]) -> Vec<Found> {
    let mut found = Vec::new();
    if let Some(event) = &rule.event {
        let occurrence = Occurrence {
            name: event_name.to_string(),
            at: None,
        };
        apply(
            &event.name_rule(),
            NameClass::Event,
            &occurrence,
            &mut found,
        );
        structure(event, &occurrence, &mut found);
    }
    if let Some(data) = &rule.data {
        for id in fields.iter().filter(|id| !id.starts_with('_')) {
            let occurrence = Occurrence {
                name: id.clone(),
                at: None,
            };
            apply(data, NameClass::Data, &occurrence, &mut found);
        }
    }
    found
}

impl NamesRule {
    /// Whether the setting reaches an event-schema document: it asks something
    /// of the events or of the data ids.
    pub fn reaches_event_schemas(&self) -> bool {
        self.event.is_some() || self.data.is_some()
    }
}

/// The style and the limits of `rule` applied to one name.
fn apply(rule: &NameRule, class: NameClass, occurrence: &Occurrence, found: &mut Vec<Found>) {
    let name = occurrence.name.as_str();
    let mut push = |departure| {
        found.push(Found {
            departure,
            at: occurrence.at.clone(),
        })
    };
    if let Some(style) = rule.style {
        // An event is spelled token by token; every other class is one token.
        let parts: Vec<&str> = if class == NameClass::Event {
            name.split('.').collect()
        } else {
            vec![name]
        };
        if let Some(part) = parts.iter().find(|part| !style.accepts(part)) {
            let respelled = parts
                .iter()
                .map(|part| style.respell(part))
                .collect::<Option<Vec<_>>>()
                .map(|tokens| tokens.join("."));
            push(Departure::Style {
                class,
                name: name.to_string(),
                part: (*part).to_string(),
                style,
                respelled,
            });
        }
    }
    if let Some(forbidden) = &rule.forbidden_words {
        let made_of = words_of(name);
        let hits: Vec<String> = forbidden
            .iter()
            .filter(|word| made_of.contains(&word.to_ascii_lowercase()))
            .cloned()
            .collect();
        if !hits.is_empty() {
            push(Departure::Limit {
                class,
                name: name.to_string(),
                limit: NameLimit::ForbiddenWords(hits),
            });
        }
    }
    if let Some(max) = rule.max_length {
        let actual = name.chars().count();
        if actual > max {
            push(Departure::Limit {
                class,
                name: name.to_string(),
                limit: NameLimit::TooLong { max, actual },
            });
        }
    }
    if let Some(prefix) = &rule.required_prefix {
        if !name.starts_with(prefix.as_str()) {
            push(Departure::Limit {
                class,
                name: name.to_string(),
                limit: NameLimit::MissingPrefix(prefix.clone()),
            });
        }
    }
}

/// The structure of one event name.
fn structure(rule: &EventRule, occurrence: &Occurrence, found: &mut Vec<Found>) {
    let tokens: Vec<&str> = occurrence.name.split('.').collect();
    let mut push = |problem| {
        found.push(Found {
            departure: Departure::Event {
                name: occurrence.name.clone(),
                problem,
            },
            at: occurrence.at.clone(),
        })
    };
    if let Some(range) = rule.tokens {
        if let Some(min) = range.min.filter(|min| tokens.len() < *min) {
            push(EventProblem::TooFewTokens {
                min,
                actual: tokens.len(),
            });
        }
        if let Some(max) = range.max.filter(|max| tokens.len() > *max) {
            push(EventProblem::TooManyTokens {
                max,
                actual: tokens.len(),
            });
        }
    }
    if let Some(allowed) = &rule.first_tokens {
        let first = tokens.first().copied().unwrap_or_default();
        if !allowed.iter().any(|token| token == first) {
            push(EventProblem::FirstToken {
                allowed: allowed.clone(),
                actual: first.to_string(),
            });
        }
    }
}

/// Every pair of event names in which the first is a token prefix of the
/// second, where the document wrote at least one of the two.
///
/// The names an imported event-schema declares are in the set: a transition on
/// a schema's `door` takes a `door.open` the document defines all the same.
fn prefix_free(model: &SCXMLModel, defined: &[Occurrence], found: &mut Vec<Found>) {
    let supplied: Vec<&str> = model
        .imported_event_schemas
        .keys()
        .map(String::as_str)
        .collect();
    let written: BTreeSet<&str> = defined.iter().map(|o| o.name.as_str()).collect();
    let mut all: Vec<&str> = written.iter().copied().chain(supplied).collect();
    all.sort_unstable();
    all.dedup();
    for shorter in &all {
        for longer in &all {
            let is_prefix = shorter != longer && EventDescriptor::Prefix(shorter).matches(longer);
            if !is_prefix || !(written.contains(shorter) || written.contains(longer)) {
                continue;
            }
            let at = defined
                .iter()
                .find(|o| o.name == *shorter || o.name == *longer)
                .and_then(|o| o.at.clone());
            found.push(Found {
                departure: Departure::PrefixOfAnother {
                    prefix: (*shorter).to_string(),
                    longer: (*longer).to_string(),
                },
                at,
            });
        }
    }
}

/// The ids of the document's states, in document order, and of its history
/// pseudostates. A name the platform reserves — a leading underscore — is not
/// one the author chose.
fn state_names(model: &SCXMLModel) -> Vec<Occurrence> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    let mut names: Vec<Occurrence> = states
        .iter()
        .map(|state| Occurrence {
            name: state.id.clone(),
            at: state.source_location.clone(),
        })
        .collect();
    for (id, history) in &model.history_states {
        names.push(Occurrence {
            name: id.clone(),
            at: model
                .states
                .get(&history.parent)
                .and_then(|parent| parent.source_location.clone()),
        });
    }
    names.retain(|o| !o.name.starts_with('_'));
    names
}

/// The ids of the document's `<data>`, top level and per state, once each.
fn data_names(model: &SCXMLModel) -> Vec<Occurrence> {
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    let declared = model
        .variables
        .iter()
        .chain(states.iter().flat_map(|state| state.datamodel.iter()));
    for variable in declared {
        if !variable.id.starts_with('_') && seen.insert(variable.id.clone()) {
            names.push(Occurrence {
                name: variable.id.clone(),
                at: variable.source_location.clone(),
            });
        }
    }
    names
}

/// The events the document defines by a literal name, once each, in the order
/// they are first written: the transitions' literal descriptors, every
/// `<raise>` and every `<send>` that names its event.
///
/// Not what the interface supplies (an event an imported event-schema
/// declares) and not what the platform does.
fn event_names(model: &SCXMLModel) -> Vec<Occurrence> {
    let mut seen = BTreeSet::new();
    let mut names: Vec<Occurrence> = Vec::new();
    let mut add = |name: &str, at: Option<SourceLocation>| {
        if !crate::analyzer::is_reserved_ingress_event(name)
            && !model.imported_event_schemas.contains_key(name)
            && seen.insert(name.to_string())
        {
            names.push(Occurrence {
                name: name.to_string(),
                at,
            });
        }
    };
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    for state in &states {
        for transition in &state.transitions {
            for token in transition.event.split_whitespace() {
                if let Some(name) = literal_event(token) {
                    add(name, transition.source_location.clone());
                }
            }
        }
    }
    crate::host_processor_analyzer::walk_model_actions(model, &mut |_state, action| {
        let names_an_event = matches!(action.action_type.as_str(), "raise" | "send")
            && action.eventexpr.is_empty()
            && !action.event.is_empty();
        if names_an_event {
            add(&action.event, action.source_location.clone());
        }
    });
    // Captured at parse time, `<finalize>` included, so it reaches an event put
    // on the internal queue that the walk above cannot see; such a name has no
    // row of its own.
    for queued in &model.internal_queue_events {
        add(queued, None);
    }
    names.sort_by_key(|o| {
        (
            o.at.is_none(),
            o.at.as_ref().and_then(|at| at.line),
            o.at.as_ref().and_then(|at| at.col),
            o.name.clone(),
        )
    });
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(spelled: &[&str]) -> Vec<String> {
        spelled.iter().map(|w| w.to_string()).collect()
    }

    /// Every style accepts what it spells and refuses what another style
    /// spells, so a rule that judged only the ones that happen to agree would
    /// fail here.
    #[test]
    fn a_style_accepts_exactly_its_own_spelling() {
        let door_open = words(&["door", "open"]);
        let spelled = [
            (Style::Snake, "door_open"),
            (Style::UpperSnake, "DOOR_OPEN"),
            (Style::Camel, "doorOpen"),
            (Style::Pascal, "DoorOpen"),
            (Style::Kebab, "door-open"),
            (Style::Lower, "dooropen"),
        ];
        for (style, text) in spelled {
            assert_eq!(style.spell(&door_open), text, "{style:?}");
            assert!(style.accepts(text), "{style:?} refused its own {text}");
        }
        // Two words tell the styles apart, and each is refused by every other.
        // `dooropen` is the exception that proves the rule: it has no second
        // word to mark, so it is a single word in four styles at once.
        for (style, _) in spelled {
            for (other, other_text) in spelled {
                if other == Style::Lower || style == Style::Lower {
                    continue;
                }
                assert_eq!(
                    style.accepts(other_text),
                    other == style,
                    "{style:?} judging {other_text} ({other:?})"
                );
            }
        }
        for text in [
            "door_open",
            "DOOR_OPEN",
            "doorOpen",
            "DoorOpen",
            "door-open",
        ] {
            assert!(!Style::Lower.accepts(text), "lowercase accepted {text}");
        }
        for style in [Style::Snake, Style::Kebab, Style::Camel] {
            assert!(style.accepts("dooropen"), "{style:?}");
        }
        assert!(!Style::Pascal.accepts("dooropen"));
        assert!(!Style::UpperSnake.accepts("dooropen"));
    }

    #[test]
    fn a_word_that_is_one_token_is_every_styles_to_accept_when_it_fits() {
        // `door` has no second word to mark, so all the lower-case styles and
        // camel case spell it the same.
        for style in [Style::Snake, Style::Kebab, Style::Lower, Style::Camel] {
            assert!(style.accepts("door"), "{style:?}");
        }
        assert!(Style::Pascal.accepts("Door"));
        assert!(Style::UpperSnake.accepts("DOOR"));
        assert!(!Style::Pascal.accepts("door"));
        assert!(!Style::Snake.accepts("Door"));
    }

    #[test]
    fn what_no_style_accepts_is_refused() {
        for style in [
            Style::Snake,
            Style::UpperSnake,
            Style::Camel,
            Style::Pascal,
            Style::Kebab,
            Style::Lower,
        ] {
            for token in [
                "",
                "_",
                "-",
                "door__open",
                "_door",
                "door_",
                "1door",
                "d\u{f8}r",
            ] {
                assert!(!style.accepts(token), "{style:?} accepted {token:?}");
            }
        }
        // A capital starts a word and is followed by a lower-case letter or a
        // digit, so an acronym is a word and not a run of capitals.
        assert!(!Style::Camel.accepts("doorHTTP"));
        assert!(!Style::Pascal.accepts("DoorHTTP"));
        assert!(Style::Camel.accepts("doorHttp"));
        assert!(Style::Camel.accepts("door2Open"));
    }

    #[test]
    fn the_words_of_a_name_are_found_under_every_spelling() {
        let expected = words(&["door", "open", "request"]);
        for spelled in [
            "door_open_request",
            "DOOR_OPEN_REQUEST",
            "doorOpenRequest",
            "DoorOpenRequest",
            "door-open-request",
            "door open request",
        ] {
            assert_eq!(words_of(spelled), expected, "{spelled}");
        }
        assert_eq!(words_of("HTTPServer"), words(&["http", "server"]));
        assert_eq!(words_of("door2Open"), words(&["door2", "open"]));
        assert!(words_of("\u{b3d9}").is_empty());
    }

    #[test]
    fn a_respelling_is_offered_only_when_the_style_accepts_it() {
        assert_eq!(
            Style::Snake.respell("DoorOpen").as_deref(),
            Some("door_open")
        );
        assert_eq!(
            Style::Pascal.respell("door_open").as_deref(),
            Some("DoorOpen")
        );
        assert_eq!(
            Style::Camel.respell("DOOR_OPEN").as_deref(),
            Some("doorOpen")
        );
        // A name that begins with a digit has no respelling in any style here.
        assert_eq!(Style::Snake.respell("2nd_door"), None);
        // And one with no ASCII word in it has none either.
        assert_eq!(Style::Snake.respell("\u{b3d9}\u{c791}"), None);
    }

    /// Whatever the words, what a style spells it accepts — the property that
    /// makes a suggested respelling one the profile will not refuse again.
    #[test]
    fn a_style_accepts_what_it_spells() {
        let samples = [
            words(&["door"]),
            words(&["door", "open"]),
            words(&["door2", "open"]),
            words(&["http", "server", "2"]),
        ];
        for style in [
            Style::Snake,
            Style::UpperSnake,
            Style::Camel,
            Style::Pascal,
            Style::Kebab,
            Style::Lower,
        ] {
            for sample in &samples {
                let spelled = style.spell(sample);
                assert!(style.accepts(&spelled), "{style:?} refused {spelled:?}");
            }
        }
    }

    /// A capital letter starts a word and is followed by a lower-case letter
    /// or a digit, so a single-letter word after the first has no camel or
    /// Pascal spelling: `aBC` reads as a word and a run of capitals. The
    /// respelling is then not offered, rather than offered and refused again.
    #[test]
    fn a_single_letter_word_has_no_camel_or_pascal_spelling() {
        let letters = words(&["a", "b", "c"]);
        for style in [Style::Camel, Style::Pascal] {
            assert!(!style.accepts(&style.spell(&letters)), "{style:?}");
            assert_eq!(style.respell("a_b_c"), None, "{style:?}");
        }
        for style in [Style::Snake, Style::UpperSnake, Style::Kebab, Style::Lower] {
            assert!(style.accepts(&style.spell(&letters)), "{style:?}");
        }
    }

    fn rule(json: &str) -> NamesRule {
        serde_json::from_str(json).expect("a names setting")
    }

    fn statechart(body: &str) -> SCXMLModel {
        crate::parser::SCXMLParser::new()
            .parse_string(
                &format!(
                    r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                             xmlns:sce="http://sce.dev/ext" version="1.0"
                             name="DoorControl" initial="idle" datamodel="ecmascript">
                         {body}
                       </scxml>"#
                ),
                "names",
            )
            .expect("parses")
    }

    const DOOR: &str = r#"
        <datamodel><data id="openCount" expr="0"/><data id="_system"/></datamodel>
        <state id="idle">
          <transition event="door.open" target="OpenState"/>
          <transition event="error.execution" target="idle"/>
          <transition event="door.*" target="idle"/>
        </state>
        <state id="OpenState">
          <onentry><raise event="door.Timer"/><send event="door.close.request"/></onentry>
          <transition event="door.close" target="idle"/>
        </state>"#;

    fn spelled(found: &[Found]) -> Vec<String> {
        found
            .iter()
            .map(|f| match &f.departure {
                Departure::Style { class, name, .. } => {
                    format!("style {} {name}", class.as_str())
                }
                Departure::Limit {
                    class, name, limit, ..
                } => format!("limit {} {name} {}", class.as_str(), limit.kind()),
                Departure::Event { name, problem } => {
                    format!("event {name} {}", problem.kind())
                }
                Departure::PrefixOfAnother { prefix, longer } => {
                    format!("prefix {prefix} of {longer}")
                }
            })
            .collect()
    }

    #[test]
    fn a_rule_finds_the_names_the_document_defines_and_none_the_platform_does() {
        let model = statechart(DOOR);
        let found = judge(
            &rule(
                r#"{"state":{"style":"snake"},"document":{"style":"snake"},"event":{"style":"snake"},"data":{"style":"snake"}}"#,
            ),
            &model,
        );
        assert_eq!(
            spelled(&found),
            [
                "style document DoorControl",
                "style state OpenState",
                "style event door.Timer",
                "style data openCount",
            ],
            "the platform's error.execution, the pattern door.* and _system are nobody's choice"
        );
    }

    #[test]
    fn a_respelling_of_an_event_is_made_token_by_token() {
        let model = statechart(DOOR);
        let found = judge(&rule(r#"{"event":{"style":"snake"}}"#), &model);
        let Departure::Style {
            part, respelled, ..
        } = &found[0].departure
        else {
            panic!("{found:?}");
        };
        assert_eq!(part, "Timer");
        assert_eq!(respelled.as_deref(), Some("door.timer"));
    }

    #[test]
    fn every_limit_a_rule_states_is_held() {
        let model = statechart(DOOR);
        let found = judge(
            &rule(
                r#"{"state":{"forbidden_words":["open"],"max_length":4,"required_prefix":"st_"}}"#,
            ),
            &model,
        );
        assert_eq!(
            spelled(&found),
            [
                "limit state idle prefix",
                "limit state OpenState forbidden-word",
                "limit state OpenState length",
                "limit state OpenState prefix",
            ],
            "idle is short and lacks the prefix; OpenState is made of `open`, is too long and lacks it"
        );
    }

    /// A limit is held AT its edge, not only far past it: a name exactly as
    /// long as the limit is fine and one character more is refused; a range's
    /// ends are inclusive; a prefix the name has is not a prefix it lacks.
    /// Every other limit test here uses a name well outside the limit, which
    /// a limit that was one too generous would still refuse.
    #[test]
    fn a_limit_is_held_at_its_edge() {
        let state = |json: &str, id: &str| {
            let model = statechart(&format!(
                r#"<state id="{id}"><transition event="a.b" target="{id}"/></state>"#
            ));
            spelled(&judge(&rule(json), &model))
        };
        // `idle` is 4 characters and `idles` is 5.
        assert!(state(r#"{"state":{"max_length":4}}"#, "idle").is_empty());
        assert_eq!(
            state(r#"{"state":{"max_length":4}}"#, "idles"),
            ["limit state idles length"]
        );
        // A prefix the name has, and one it lacks, by a single character.
        assert!(state(r#"{"state":{"required_prefix":"s_"}}"#, "s_idle").is_empty());
        assert_eq!(
            state(r#"{"state":{"required_prefix":"s_"}}"#, "s-idle"),
            ["limit state s-idle prefix"]
        );
        // A forbidden word is a whole word of the name: `idle` is not made of
        // `id`, and `door_id` is.
        assert!(state(r#"{"state":{"forbidden_words":["id"]}}"#, "idle").is_empty());
        assert_eq!(
            state(r#"{"state":{"forbidden_words":["id"]}}"#, "door_id"),
            ["limit state door_id forbidden-word"]
        );
        // The ends of a token range are inclusive.
        let event = |json: &str, name: &str| {
            let model = statechart(&format!(
                r#"<state id="idle"><transition event="{name}" target="idle"/></state>"#
            ));
            spelled(&judge(&rule(json), &model))
        };
        let two_or_three = r#"{"event":{"tokens":{"min":2,"max":3}}}"#;
        assert!(event(two_or_three, "a.b").is_empty());
        assert!(event(two_or_three, "a.b.c").is_empty());
        assert_eq!(event(two_or_three, "a"), ["event a tokens-min"]);
        assert_eq!(event(two_or_three, "a.b.c.d"), ["event a.b.c.d tokens-max"]);
    }

    #[test]
    fn the_structure_of_an_event_name_is_held() {
        let model = statechart(DOOR);
        let found = judge(
            &rule(r#"{"event":{"tokens":{"min":2,"max":2},"first_tokens":["door"]}}"#),
            &model,
        );
        assert_eq!(
            spelled(&found),
            ["event door.close.request tokens-max"],
            "door.open, door.Timer and door.close are two tokens each and begin with door"
        );
        let found = judge(&rule(r#"{"event":{"first_tokens":["lock"]}}"#), &model);
        assert_eq!(found.len(), 4, "{found:?}");
    }

    #[test]
    fn an_event_that_is_a_token_prefix_of_another_is_found_once() {
        let model = statechart(DOOR);
        let found = judge(&rule(r#"{"event":{"prefix_free":true}}"#), &model);
        assert_eq!(
            spelled(&found),
            ["prefix door.close of door.close.request"],
            "door.open and door.Timer share a first token and are prefixes of nothing"
        );
        // The control: the same rule over names that are not prefixes of each
        // other finds nothing, and `false` is the rule not being asked.
        let quiet = statechart(
            r#"<state id="idle"><transition event="door.open" target="b"/></state>
               <state id="b"><transition event="door.opened" target="idle"/></state>"#,
        );
        assert!(judge(&rule(r#"{"event":{"prefix_free":true}}"#), &quiet).is_empty());
        assert!(judge(&rule(r#"{"event":{"prefix_free":false}}"#), &model).is_empty());
    }

    #[test]
    fn a_descriptor_that_is_a_pattern_declares_no_name() {
        let model =
            statechart(r#"<state id="idle"><transition event="door.* *" target="idle"/></state>"#);
        let found = judge(
            &rule(r#"{"event":{"style":"upper_snake","prefix_free":true}}"#),
            &model,
        );
        assert!(found.is_empty(), "{found:?}");
    }

    /// The event a schema declares and its fields' ids are names the owner's
    /// document defines, and are held to the rule; what the rule says of a
    /// state or of the document name does not reach a schema.
    #[test]
    fn an_event_schema_is_judged_for_its_event_and_its_fields() {
        let rule = rule(
            r#"{"state":{"style":"pascal"},"document":{"style":"pascal"},
                "event":{"style":"snake","tokens":{"min":2}},
                "data":{"style":"snake"}}"#,
        );
        let found = judge_event_schema(
            &rule,
            "Door.open",
            &[
                "retryCount".to_string(),
                "_reserved".to_string(),
                "delay".to_string(),
            ],
        );
        assert_eq!(
            spelled(&found),
            ["style event Door.open", "style data retryCount"],
            "the event's first token is in the wrong case, `delay` is snake_case, `_reserved` is \
             the platform's, and the state and document rules are not about a schema"
        );
        let short = judge_event_schema(&rule, "door", &[]);
        assert_eq!(spelled(&short), ["event door tokens-min"]);
        assert!(judge_event_schema(&rule, "door.open", &["delay".to_string()]).is_empty());
        assert!(rule.reaches_event_schemas());
        assert!(!self::rule(r#"{"state":{"style":"snake"}}"#).reaches_event_schemas());
    }

    #[test]
    fn a_rule_that_is_not_a_rule_is_refused() {
        for (setting, said) in [
            (r#"{}"#, "no class"),
            (r#"{"state":{}}"#, "states no rule"),
            (r#"{"event":{}}"#, "states no rule"),
            (r#"{"state":{"max_length":0}}"#, "counts from 1"),
            (r#"{"state":{"forbidden_words":[]}}"#, "no word"),
            (
                r#"{"state":{"forbidden_words":["a b"]}}"#,
                "letters and digits",
            ),
            (r#"{"state":{"forbidden_words":["Open","open"]}}"#, "twice"),
            (r#"{"state":{"required_prefix":""}}"#, "is empty"),
            (r#"{"event":{"tokens":{}}}"#, "neither"),
            (r#"{"event":{"tokens":{"min":3,"max":2}}}"#, "at most 2"),
            (r#"{"event":{"tokens":{"min":0}}}"#, "from 1"),
            (r#"{"event":{"first_tokens":[]}}"#, "lists a token or none"),
            (r#"{"event":{"first_tokens":["a","a"]}}"#, "twice"),
        ] {
            let refusal = rule(setting).refusal();
            assert!(
                refusal.as_deref().is_some_and(|r| r.contains(said)),
                "{setting}: {refusal:?}"
            );
        }
        assert_eq!(rule(r#"{"state":{"style":"snake"}}"#).refusal(), None);
    }
}
