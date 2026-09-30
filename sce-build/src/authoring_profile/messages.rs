// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The sentences a departure from an authoring profile is reported in.
//!
//! Each names the profile, says what the profile asks for and what the
//! document did, and gives the two ways out: change the document, or use a
//! profile that asks something else. The second is not a courtesy. Which
//! spelling a design is held to is the owner's decision and never the
//! author's, so a sentence that offered only the first would read as though
//! the profile were the language.

use super::names::{EventProblem, NameClass, NameLimit, Style};

/// How a sentence names the profile.
pub(super) fn subject(profile: Option<&str>) -> String {
    match profile {
        Some(name) => format!("the authoring profile '{name}'"),
        None => "the authoring profile it was given".to_string(),
    }
}

/// A name that is not spelled in the style its class asks for.
pub(super) fn name_style(
    profile: Option<&str>,
    class: NameClass,
    name: &str,
    part: &str,
    style: Style,
    respelled: Option<&str>,
) -> String {
    let whole = if part == name {
        format!("the {} '{name}'", class.singular())
    } else {
        format!("the token '{part}' of the {} '{name}'", class.singular())
    };
    let spelling = match respelled {
        Some(better) => format!(" ({} would spell it '{better}')", style.spelled()),
        None => String::new(),
    };
    format!(
        "{} spells {} in {}, and {whole} is not{spelling} — rename it, and everything that \
         refers to it, or use a profile that spells {} another way",
        subject(profile),
        class.plural(),
        style.spelled(),
        class.plural()
    )
}

/// A name that breaks a limit of its class.
pub(super) fn name_limit(
    profile: Option<&str>,
    class: NameClass,
    name: &str,
    limit: &NameLimit,
) -> String {
    let (asks, did) = match limit {
        NameLimit::ForbiddenWords(words) => {
            let quoted: Vec<String> = words.iter().map(|w| format!("'{w}'")).collect();
            (
                format!(
                    "keeps the {} {} out of {}",
                    if words.len() == 1 { "word" } else { "words" },
                    quoted.join(", "),
                    class.plural()
                ),
                format!(
                    "the {} '{name}' is made of {}",
                    class.singular(),
                    if words.len() == 1 { "it" } else { "them" }
                ),
            )
        }
        NameLimit::TooLong { max, actual } => (
            format!("allows {} at most {max} characters", class.plural()),
            format!("the {} '{name}' has {actual}", class.singular()),
        ),
        NameLimit::MissingPrefix(prefix) => (
            format!(
                "requires every {} to begin with '{prefix}'",
                class.singular()
            ),
            format!("the {} '{name}' does not", class.singular()),
        ),
    };
    format!(
        "{} {asks}, and {did} — choose another name for it, or use a profile that asks \
         something else of {}",
        subject(profile),
        class.plural()
    )
}

/// An event name whose structure is not what the profile asks for.
pub(super) fn event_structure(profile: Option<&str>, name: &str, problem: &EventProblem) -> String {
    let (asks, did) = match problem {
        EventProblem::TooFewTokens { min, actual } => (
            format!("asks an event name for at least {min} dot-separated token(s)"),
            format!("'{name}' has {actual}"),
        ),
        EventProblem::TooManyTokens { max, actual } => (
            format!("allows an event name at most {max} dot-separated token(s)"),
            format!("'{name}' has {actual}"),
        ),
        EventProblem::FirstToken { allowed, actual } => (
            format!("begins an event name with one of {}", allowed.join(", ")),
            format!("'{name}' begins with '{actual}'"),
        ),
    };
    format!(
        "{} {asks}, and {did} — rename the event, and everything that raises, sends or takes \
         it, or use a profile that asks something else of event names",
        subject(profile)
    )
}

/// An event name that is a token prefix of another.
pub(super) fn event_prefix(profile: Option<&str>, prefix: &str, longer: &str) -> String {
    format!(
        "{} keeps an event name from being a token prefix of another, and '{prefix}' is one of \
         '{longer}' — W3C SCXML 3.12.1 matches an event descriptor by token prefix, so a \
         transition on '{prefix}' also takes '{longer}'; rename one of them, or use a profile \
         that does not ask for it",
        subject(profile)
    )
}

/// An `<sce:evidence>` that names no anchor in the specification.
pub(super) fn evidence_unanchored(profile: Option<&str>, evidence: &str) -> String {
    let shown: String = evidence.chars().take(60).collect();
    let ellipsis = if evidence.chars().count() > 60 {
        "..."
    } else {
        ""
    };
    format!(
        "{} requires every <sce:evidence> to name where the specification states what it cites, \
         and this one does not: '{shown}{ellipsis}' — add the anchor in the sce:provenance form, \
         or use a profile that does not ask for one",
        subject(profile)
    )
}

/// A state or transition that claims no requirement.
pub(super) fn element_untraced(profile: Option<&str>, element: &str) -> String {
    format!(
        "{} requires every state and transition to claim a requirement of the specification, \
         and {element} claims none: put the id of the requirement it is there for on it \
         (sce:req=\"R3\"; scxml_requirement_set makes the ids from the specification's own \
         words), or use a profile that does not ask for traceability",
        subject(profile)
    )
}
