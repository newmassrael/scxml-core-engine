// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A place of a module, in the vocabulary of the witness
//! (docs/SCE_TEST_PROFILE.md §9.4, **Places**): a path from the module's top,
//! each segment of it an object of one member, its kind.
//!
//! - a definition: `type` and `external-function` in the types module;
//!   `port-type`, `template`, `modulepar`, `component-type`, `function`,
//!   `testcase` and `control`, which its module's name names, in the
//!   behaviour module;
//! - a member of a component type: `port`, `timer`, `var`;
//! - in a function: `local`; `label`, which opens the statements after it up
//!   to the next label; `statement`, a position from 0 among the statements
//!   the segment before it opens;
//! - in a statement: `then` and `else`, a position from 0 in an `if`'s two
//!   blocks, and `alternative`, a position from 0 among an `alt`'s
//!   alternatives, which opens its block.
//!
//! The witness reads its locations into [`Segment`]s, and the reader of the
//! modules writes the places it finds as [`Segment`]s, so the two are held to
//! each other by equality.

use std::fmt;

use serde::Deserialize;

/// One step of a path, as the witness writes it: `{"<kind>": <value>}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Segment {
    /// A type of the types module.
    Type(String),
    /// A codec function of the types module.
    ExternalFunction(String),
    PortType(String),
    Template(String),
    Modulepar(String),
    ComponentType(String),
    Function(String),
    Testcase(String),
    /// The control part, named by its module.
    Control(String),
    Port(String),
    Timer(String),
    Var(String),
    Local(String),
    /// The label of an atomic state.
    Label(String),
    Statement(usize),
    Then(usize),
    Else(usize),
    Alternative(usize),
}

impl Segment {
    /// The segment's kind: the name of its one member.
    pub fn kind(&self) -> &'static str {
        match self {
            Segment::Type(_) => "type",
            Segment::ExternalFunction(_) => "external-function",
            Segment::PortType(_) => "port-type",
            Segment::Template(_) => "template",
            Segment::Modulepar(_) => "modulepar",
            Segment::ComponentType(_) => "component-type",
            Segment::Function(_) => "function",
            Segment::Testcase(_) => "testcase",
            Segment::Control(_) => "control",
            Segment::Port(_) => "port",
            Segment::Timer(_) => "timer",
            Segment::Var(_) => "var",
            Segment::Local(_) => "local",
            Segment::Label(_) => "label",
            Segment::Statement(_) => "statement",
            Segment::Then(_) => "then",
            Segment::Else(_) => "else",
            Segment::Alternative(_) => "alternative",
        }
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Segment::Statement(position)
            | Segment::Then(position)
            | Segment::Else(position)
            | Segment::Alternative(position) => {
                write!(f, "{{\"{}\":{position}}}", self.kind())
            }
            Segment::Type(name)
            | Segment::ExternalFunction(name)
            | Segment::PortType(name)
            | Segment::Template(name)
            | Segment::Modulepar(name)
            | Segment::ComponentType(name)
            | Segment::Function(name)
            | Segment::Testcase(name)
            | Segment::Control(name)
            | Segment::Port(name)
            | Segment::Timer(name)
            | Segment::Var(name)
            | Segment::Local(name)
            | Segment::Label(name) => write!(
                f,
                "{{\"{}\":{}}}",
                self.kind(),
                serde_json::Value::String(name.clone())
            ),
        }
    }
}

/// `path` as the witness writes it: a JSON array of its segments.
pub fn path_text(path: &[Segment]) -> String {
    let segments: Vec<String> = path.iter().map(Segment::to_string).collect();
    format!("[{}]", segments.join(","))
}

/// `path` with `segment` after it.
pub fn extended(path: &[Segment], segment: Segment) -> Vec<Segment> {
    let mut path = path.to_vec();
    path.push(segment);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A segment is read from an object of one member and written back as
    /// the same object.
    #[test]
    fn a_segment_is_an_object_of_one_member() {
        let path: Vec<Segment> = serde_json::from_str(
            r#"[{"function":"f_c"},{"label":"a"},{"statement":1},{"alternative":0}]"#,
        )
        .expect("a path");
        assert_eq!(
            path,
            [
                Segment::Function("f_c".to_string()),
                Segment::Label("a".to_string()),
                Segment::Statement(1),
                Segment::Alternative(0),
            ]
        );
        assert_eq!(
            path_text(&path),
            r#"[{"function":"f_c"},{"label":"a"},{"statement":1},{"alternative":0}]"#
        );
        let kebab: Segment =
            serde_json::from_str(r#"{"external-function":"enc_x"}"#).expect("a segment");
        assert_eq!(kebab, Segment::ExternalFunction("enc_x".to_string()));
        assert!(serde_json::from_str::<Segment>(r#"{"label":"a","statement":0}"#).is_err());
        assert!(serde_json::from_str::<Segment>(r#"{"step":0}"#).is_err());
    }
}
