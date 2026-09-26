// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `datamodel="sce-static"` machine's whole state, saved at a macrostep
//! boundary and restored into a new process (SCE Accepted Subset §2.15, E17).
//!
//! A host whose process can be killed — a phone app, an ECU that reboots —
//! saves the machine and gives it back later. What it saves is everything a
//! macrostep boundary holds that the document cannot recompute: where the
//! machine is (its configuration and current leaf) and every variable, the
//! machine's own included — not only the ones a snapshot publishes.
//!
//! The format is one JSON document (`sce-saved-state`, version [`FORMAT`]),
//! the same on every backend, so what one backend saved another can read. A
//! variable is keyed by its document id and written as its `sce:type` says:
//! a number, except a 64-bit integer, written as a text so no reader that
//! holds numbers as doubles loses its low bits; a record as an object of its
//! fields; a list as an array.
//!
//! A saved state is bound to the document it was saved from, by that
//! document's source hash: restoring it into a machine generated from another
//! document is refused, since a variable of the same name need not mean the
//! same thing there.
//!
//! ⚠ `std` only: a `no_std` machine has no JSON and nothing to save it to.

#![cfg(not(feature = "no_std"))]

use core::fmt;

use crate::json::{self, Value};

/// The format version this runtime writes and reads.
pub const FORMAT: u32 = 1;

/// Why a saved state cannot be restored, or a machine cannot be saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateRefusal(String);

impl StateRefusal {
    /// Refuse, saying why.
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    /// Why.
    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StateRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A machine's saved state.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedState {
    /// The source hash of the document the machine was generated from.
    pub document: String,
    /// The active configuration, as state ids.
    pub configuration: Vec<String>,
    /// The current leaf, as a state id — which of a `<parallel>`'s regions
    /// the machine last stood in, which the configuration alone cannot say.
    pub current: String,
    /// Every variable, by document id, in declaration order.
    pub variables: Vec<(String, Value)>,
}

impl SavedState {
    /// The variable `id`, or a refusal naming it.
    pub fn variable(&self, id: &str) -> Result<&Value, StateRefusal> {
        self.variables
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, v)| v)
            .ok_or_else(|| StateRefusal::new(format!("the saved state has no variable '{id}'")))
    }

    /// The state as `sce-saved-state` JSON.
    pub fn to_json(&self) -> String {
        json::write(&Value::Object(vec![
            ("format".to_string(), Value::Number(FORMAT.to_string())),
            ("document".to_string(), Value::Text(self.document.clone())),
            (
                "configuration".to_string(),
                Value::Array(
                    self.configuration
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ),
            ("current".to_string(), Value::Text(self.current.clone())),
            (
                "variables".to_string(),
                Value::Object(self.variables.clone()),
            ),
        ]))
    }

    /// Read `sce-saved-state` JSON. The document hash is not judged here —
    /// the machine that restores it knows its own.
    pub fn from_json(text: &str) -> Result<Self, StateRefusal> {
        let value =
            json::parse(text).map_err(|e| StateRefusal::new(format!("the saved state is {e}")))?;
        let field = |name: &str| {
            value
                .member(name)
                .ok_or_else(|| StateRefusal::new(format!("the saved state has no '{name}'")))
        };
        match field("format")? {
            Value::Number(n) if n == &FORMAT.to_string() => {}
            other => {
                return Err(StateRefusal::new(format!(
                    "the saved state is format {}, and this runtime reads format {FORMAT}",
                    json::write(other)
                )))
            }
        }
        let text_of = |v: &Value, what: &str| match v {
            Value::Text(s) => Ok(s.clone()),
            _ => Err(StateRefusal::new(format!("'{what}' is not a text"))),
        };
        let configuration = match field("configuration")? {
            Value::Array(items) => items
                .iter()
                .map(|v| text_of(v, "configuration"))
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(StateRefusal::new("'configuration' is not an array")),
        };
        let variables = match field("variables")? {
            Value::Object(members) => members.clone(),
            _ => return Err(StateRefusal::new("'variables' is not an object")),
        };
        Ok(Self {
            document: text_of(field("document")?, "document")?,
            configuration,
            current: text_of(field("current")?, "current")?,
            variables,
        })
    }
}

/// A value a saved state holds: written as its `sce:type` says, and read back
/// only if it is one — a value of another kind, or one its width cannot hold,
/// is refused rather than converted.
pub trait SavedValue: Sized {
    /// The value as saved-state JSON.
    fn to_saved(&self) -> Value;
    /// The value read back from saved-state JSON; `what` names it in a
    /// refusal.
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal>;
}

fn number<'v>(value: &'v Value, what: &str) -> Result<&'v str, StateRefusal> {
    match value {
        Value::Number(n) => Ok(n),
        _ => Err(StateRefusal::new(format!("'{what}' is not a number"))),
    }
}

macro_rules! saved_narrow_int {
    ($($t:ty),*) => {$(
        impl SavedValue for $t {
            fn to_saved(&self) -> Value {
                Value::Number(self.to_string())
            }
            fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
                let n = number(value, what)?;
                n.parse().map_err(|_| {
                    StateRefusal::new(format!(
                        "'{what}' ({n}) is not a whole number its type can hold"
                    ))
                })
            }
        }
    )*};
}
saved_narrow_int!(u8, u16, u32, i8, i16, i32);

// A 64-bit integer is written as a text: a reader that holds JSON numbers as
// doubles would lose its low bits, and a saved state outlives the backend
// that wrote it.
macro_rules! saved_wide_int {
    ($($t:ty),*) => {$(
        impl SavedValue for $t {
            fn to_saved(&self) -> Value {
                Value::Text(self.to_string())
            }
            fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
                match value {
                    Value::Text(n) => n.parse().map_err(|_| {
                        StateRefusal::new(format!(
                            "'{what}' ({n}) is not a whole number its type can hold"
                        ))
                    }),
                    _ => Err(StateRefusal::new(format!(
                        "'{what}' is not a 64-bit whole number written as a text"
                    ))),
                }
            }
        }
    )*};
}
saved_wide_int!(u64, i64);

macro_rules! saved_real {
    ($($t:ty),*) => {$(
        impl SavedValue for $t {
            fn to_saved(&self) -> Value {
                Value::Number(format!("{:?}", self))
            }
            fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
                let n = number(value, what)?;
                n.parse()
                    .map_err(|_| StateRefusal::new(format!("'{what}' ({n}) is not a number")))
            }
        }
    )*};
}
saved_real!(f32, f64);

impl SavedValue for bool {
    fn to_saved(&self) -> Value {
        Value::Bool(*self)
    }
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        match value {
            Value::Bool(b) => Ok(*b),
            _ => Err(StateRefusal::new(format!("'{what}' is not a truth value"))),
        }
    }
}

impl SavedValue for String {
    fn to_saved(&self) -> Value {
        Value::Text(self.clone())
    }
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        match value {
            Value::Text(s) => Ok(s.clone()),
            _ => Err(StateRefusal::new(format!("'{what}' is not a text"))),
        }
    }
}

impl<T: SavedValue> SavedValue for Vec<T> {
    fn to_saved(&self) -> Value {
        Value::Array(self.iter().map(SavedValue::to_saved).collect())
    }
    fn from_saved(value: &Value, what: &str) -> Result<Self, StateRefusal> {
        match value {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| T::from_saved(item, &format!("{what}[{i}]")))
                .collect(),
            _ => Err(StateRefusal::new(format!("'{what}' is not an array"))),
        }
    }
}

/// The field `name` of a saved record `value`, read as `T` — what a
/// generated record's [`SavedValue`] reads each field with.
pub fn record_field<T: SavedValue>(
    value: &Value,
    record: &str,
    name: &str,
) -> Result<T, StateRefusal> {
    let field = value
        .member(name)
        .ok_or_else(|| StateRefusal::new(format!("'{record}' has no field '{name}'")))?;
    T::from_saved(field, &format!("{record}.{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_state_is_read_back_as_it_was_written() {
        let state = SavedState {
            document: "abc".to_string(),
            configuration: vec!["counting".to_string()],
            current: "counting".to_string(),
            variables: vec![
                ("count".to_string(), 5u32.to_saved()),
                ("big".to_string(), u64::MAX.to_saved()),
                ("picked".to_string(), vec![3u8, 1].to_saved()),
            ],
        };
        let back = SavedState::from_json(&state.to_json()).expect("reads");
        assert_eq!(back, state);
        assert_eq!(
            u64::from_saved(back.variable("big").unwrap(), "big"),
            Ok(u64::MAX)
        );
    }

    #[test]
    fn a_value_its_type_cannot_hold_is_refused() {
        assert!(u8::from_saved(&Value::Number("256".to_string()), "level").is_err());
        assert!(u32::from_saved(&Value::Text("5".to_string()), "count").is_err());
        assert!(u64::from_saved(&Value::Number("5".to_string()), "big").is_err());
    }

    #[test]
    fn another_format_is_refused() {
        let text = r#"{"format":2,"document":"d","configuration":[],"current":"s","variables":{}}"#;
        assert!(SavedState::from_json(text).is_err());
    }
}
