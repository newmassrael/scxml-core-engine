// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bounds a deployment chooses: `sce:limit`.
//!
//! A `sce-static` machine keeps every list and string in a fixed capacity, and
//! an imported algorithm keeps its result buffer in one: the number is part of
//! what the document means, since a write past it is refused the same way on
//! every backend. A document writes the number it is built with. When the same
//! document has to be built larger for a deployment that holds more (a team
//! calendar on a server rather than a room on a microcontroller), it cannot be
//! copied and edited: the machine and the algorithms it calls name their
//! bounds in several files, which have to move together or the build is
//! refused (a list variable may not be smaller than what an algorithm returns).
//!
//! `sce:limit="name"` writes that connection. An element that carries a
//! `capacity` (a list variable or buffer) or a `returns-max-size` (an
//! algorithm's return) and `sce:limit` takes the number the deployment gave the
//! name when it gave one, and the number the element wrote when it did not. So
//! one name moves every bound that has to move with it, wherever it is written,
//! and a document built without a deployment is the document as written.
//!
//! The deployment's numbers are given to the generator (`sce-codegen --limit
//! name=N`, repeatable, or [`scope`] for a build script), and they are part of
//! what the `source-hash` of the files it writes says (`forge::drift`), so a
//! tree built larger cannot pass for the tree built as written.
//!
//! The numbers a generator is given are kept for the thread that runs it, not
//! for the process: the parsers that read a bound are many and deep, and a
//! parameter to each would be a second way to say what one scope says; and a
//! test that builds a document larger runs beside tests that build it as
//! written.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

/// The numbers a deployment gave its named bounds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    values: BTreeMap<String, u32>,
}

/// Why a `name=N` was not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitError {
    /// There was no `=`: the text is not a `name=N`.
    NoEquals(String),
    /// A name is a lower-case word of letters, digits and underscores that starts
    /// with a letter, which is what an XML attribute can spell without quoting.
    BadName(String),
    /// The number is a positive whole number that fits 32 bits, as every bound is.
    BadNumber { name: String, value: String },
    /// The name was given twice, with two numbers.
    Twice(String),
}

impl std::fmt::Display for LimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEquals(text) => write!(f, "`{text}` is not a limit: write name=N"),
            Self::BadName(name) => write!(
                f,
                "`{name}` is not a limit name: a lower-case letter, then lower-case letters, \
                 digits and underscores"
            ),
            Self::BadNumber { name, value } => write!(
                f,
                "limit `{name}` is `{value}`, which is not a positive whole number that fits \
                 32 bits"
            ),
            Self::Twice(name) => write!(f, "limit `{name}` is given twice"),
        }
    }
}

impl std::error::Error for LimitError {}

fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl Limits {
    /// No limits: every document is built as written.
    pub fn none() -> Self {
        Self::default()
    }

    /// One `name=N` more.
    pub fn with(mut self, name: &str, value: u32) -> Result<Self, LimitError> {
        if !is_name(name) {
            return Err(LimitError::BadName(name.to_string()));
        }
        if value == 0 {
            return Err(LimitError::BadNumber {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
        if self.values.insert(name.to_string(), value).is_some() {
            return Err(LimitError::Twice(name.to_string()));
        }
        Ok(self)
    }

    /// The limits written as `name=N` words, as the command line gives them.
    pub fn parse<I, S>(words: I) -> Result<Self, LimitError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut limits = Self::none();
        for word in words {
            let word = word.as_ref();
            let (name, value) = word
                .split_once('=')
                .ok_or_else(|| LimitError::NoEquals(word.to_string()))?;
            let number = value
                .parse::<u32>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(|| LimitError::BadNumber {
                    name: name.to_string(),
                    value: value.to_string(),
                })?;
            limits = limits.with(name, number)?;
        }
        Ok(limits)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The names given, in order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    /// The limits as one text, the same for the same limits whatever order they
    /// were given in: what the `source-hash` of a tree built with them folds in.
    pub fn canonical(&self) -> String {
        self.values
            .iter()
            .map(|(name, value)| format!("{name}={value}\n"))
            .collect()
    }
}

/// What this thread's generator has been given and has read.
#[derive(Default)]
struct Active {
    limits: Limits,
    /// Names an element asked about, whether or not the deployment gave a number.
    declared: BTreeSet<String>,
    /// Names the deployment gave a number that an element took.
    applied: BTreeSet<String>,
}

thread_local! {
    static ACTIVE: RefCell<Active> = RefCell::new(Active::default());
}

/// The limits a thread's generator runs under, until the guard is dropped.
pub struct Scope {
    before: Option<Active>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(before) = self.before.take() {
            ACTIVE.with(|active| *active.borrow_mut() = before);
        }
    }
}

/// Run this thread's generator under `limits` until the returned guard is dropped.
pub fn scope(limits: &Limits) -> Scope {
    let before = ACTIVE.with(|active| {
        active.replace(Active {
            limits: limits.clone(),
            ..Active::default()
        })
    });
    Scope {
        before: Some(before),
    }
}

/// The number a deployment gave `name`, or `written` when it gave none.
///
/// Called by the readers of a bound, which have the number the element wrote and
/// the name it carries. Records that the name was asked about, and that it was
/// taken when it was.
pub fn resolve(name: &str, written: u32) -> u32 {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        active.declared.insert(name.to_string());
        match active.limits.values.get(name).copied() {
            Some(value) => {
                active.applied.insert(name.to_string());
                value
            }
            None => written,
        }
    })
}

/// What this thread's limits are now: for the `source-hash`, which folds them in.
pub fn current() -> Limits {
    ACTIVE.with(|active| active.borrow().limits.clone())
}

/// The names the deployment gave that no element read, in order: a name that is
/// spelled wrong, or that the documents built so far do not use. A build that
/// builds every document of a deployment reads them all, so a name still here
/// afterwards names nothing.
pub fn unused() -> Vec<String> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        active
            .limits
            .names()
            .filter(|name| !active.applied.contains(*name))
            .map(str::to_string)
            .collect()
    })
}

/// The names an element declared, whether or not the deployment gave them a number.
pub fn declared() -> Vec<String> {
    ACTIVE.with(|active| active.borrow().declared.iter().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_without_a_deployment_is_the_number_written() {
        let _scope = scope(&Limits::none());
        assert_eq!(resolve("room", 512), 512);
        assert_eq!(declared(), ["room"]);
    }

    #[test]
    fn a_deployment_replaces_the_number_for_its_name_only() {
        let limits = Limits::parse(["room=4096"]).unwrap();
        let _scope = scope(&limits);
        assert_eq!(resolve("room", 512), 4096);
        assert_eq!(resolve("other", 64), 64);
        assert!(unused().is_empty());
    }

    #[test]
    fn a_scope_ends_and_the_thread_is_as_written_again() {
        let _outer = scope(&Limits::none());
        {
            let limits = Limits::parse(["room=9"]).unwrap();
            let _scope = scope(&limits);
            assert_eq!(resolve("room", 1), 9);
        }
        assert_eq!(resolve("room", 1), 1);
        assert!(current().is_empty());
    }

    #[test]
    fn a_name_that_no_element_read_is_reported() {
        let limits = Limits::parse(["room=9", "roomm=9"]).unwrap();
        let _scope = scope(&limits);
        assert_eq!(resolve("room", 1), 9);
        assert_eq!(unused(), ["roomm"]);
    }

    #[test]
    fn what_is_not_a_limit_is_refused_naming_why() {
        assert_eq!(
            Limits::parse(["room"]),
            Err(LimitError::NoEquals("room".into()))
        );
        assert_eq!(
            Limits::parse(["Room=1"]),
            Err(LimitError::BadName("Room".into()))
        );
        assert_eq!(
            Limits::parse(["room=0"]),
            Err(LimitError::BadNumber {
                name: "room".into(),
                value: "0".into()
            })
        );
        assert_eq!(
            Limits::parse(["room=4294967296"]),
            Err(LimitError::BadNumber {
                name: "room".into(),
                value: "4294967296".into()
            })
        );
        assert_eq!(
            Limits::parse(["room=1", "room=2"]),
            Err(LimitError::Twice("room".into()))
        );
    }

    #[test]
    fn the_canonical_text_does_not_depend_on_the_order_given() {
        let one = Limits::parse(["b=2", "a=1"]).unwrap();
        let two = Limits::parse(["a=1", "b=2"]).unwrap();
        assert_eq!(one.canonical(), two.canonical());
        assert_eq!(one.canonical(), "a=1\nb=2\n");
    }
}
