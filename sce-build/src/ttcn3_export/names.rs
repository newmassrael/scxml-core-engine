// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The name contract of the TTCN-3 export (docs/SCE_TEST_PROFILE.md §9.5).
//!
//! Every name the export writes is a name of the documents, written as it is
//! written: a document by the name every unit of it is named by on every
//! backend ([`unit_name`]), a field, a variant and a formal parameter by its
//! own. A consumer that writes a module of its own against the manifest
//! (ADR 0091 C2) therefore needs no mapping to know what a definition is
//! called. A name the export cannot write so is refused
//! (`test-profile/export-name`), never renamed: a renaming would be a second
//! name for one thing, which the consumer's module, the test log and the
//! correspondence witness would each have to be told.
//!
//! A name is refused when it is no TTCN-3 identifier ([`is_identifier`]),
//! when it is a word TTCN-3 reserves ([`RESERVED`]), or when it is one
//! identifier with another name the export writes in a scope that holds both
//! ([`Namespace`]).

use std::collections::BTreeMap;

use crate::forge::error::{ForgeError, Located, TestProfileRule, ValidationError};
use crate::forge::model::ForgeKind;

/// The name the units of the document of file stem `stem` are named by on
/// every backend, and so the name its TTCN-3 definition takes: the stem in
/// lower snake case (`event-schema` is `event_schema`).
pub fn unit_name(stem: &str) -> String {
    crate::filters::to_snake_case(stem.to_string())
}

/// Whether `name` is a TTCN-3 identifier: an ASCII letter, then ASCII
/// letters, digits and underscores (ES 201 873-1 Annex A).
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The words the export does not let a name take: the keywords of TTCN-3
/// (ES 201 873-1 Annex A), the keywords its extension packages add (the
/// real-time package's and the object-oriented package's), and the names of
/// its predefined functions (-1 Annex C), which a definition of the same name
/// would make ambiguous. Wider than a tool needs is safe: a refused name is
/// renamed in its document once, and a name a tool reserves and the export
/// wrote would be a module that does not compile.
pub const RESERVED: &[&str] = &[
    // Keywords.
    "action",
    "activate",
    "address",
    "alive",
    "all",
    "alt",
    "altstep",
    "and",
    "and4b",
    "any",
    "anytype",
    "bitstring",
    "boolean",
    "break",
    "call",
    "case",
    "catch",
    "char",
    "charstring",
    "check",
    "clear",
    "complement",
    "component",
    "connect",
    "const",
    "continue",
    "control",
    "create",
    "deactivate",
    "decmatch",
    "default",
    "disconnect",
    "display",
    "do",
    "done",
    "else",
    "encode",
    "enumerated",
    "error",
    "except",
    "exception",
    "execute",
    "extends",
    "extension",
    "external",
    "fail",
    "false",
    "float",
    "for",
    "friend",
    "from",
    "function",
    "getcall",
    "getreply",
    "getverdict",
    "goto",
    "group",
    "halt",
    "hexstring",
    "if",
    "ifpresent",
    "import",
    "in",
    "inconc",
    "infinity",
    "inout",
    "integer",
    "interleave",
    "kill",
    "killed",
    "label",
    "language",
    "length",
    "log",
    "map",
    "match",
    "message",
    "mixed",
    "mod",
    "modifies",
    "module",
    "modulepar",
    "mtc",
    "noblock",
    "none",
    "not",
    "not4b",
    "not_a_number",
    "nowait",
    "null",
    "objid",
    "octetstring",
    "of",
    "omit",
    "on",
    "optional",
    "or",
    "or4b",
    "out",
    "override",
    "param",
    "pass",
    "pattern",
    "permutation",
    "port",
    "present",
    "private",
    "procedure",
    "public",
    "raise",
    "read",
    "receive",
    "record",
    "recursive",
    "rem",
    "repeat",
    "reply",
    "return",
    "running",
    "runs",
    "select",
    "self",
    "send",
    "sender",
    "set",
    "setencode",
    "setverdict",
    "signature",
    "start",
    "stop",
    "subset",
    "superset",
    "system",
    "template",
    "testcase",
    "timeout",
    "timer",
    "to",
    "trigger",
    "true",
    "type",
    "union",
    "universal",
    "unmap",
    "value",
    "valueof",
    "var",
    "variant",
    "verdicttype",
    "while",
    "with",
    "xor",
    "xor4b",
    // The real-time package's.
    "now",
    "realtime",
    "timestamp",
    "wait",
    // The object-oriented package's.
    "class",
    "finally",
    "object",
    "super",
    "this",
    // The predefined functions.
    "any2unistr",
    "bit2hex",
    "bit2int",
    "bit2oct",
    "bit2str",
    "char2int",
    "char2oct",
    "decvalue",
    "decvalue_o",
    "decvalue_unichar",
    "encvalue",
    "encvalue_o",
    "encvalue_unichar",
    "enum2int",
    "float2int",
    "get_stringencoding",
    "hex2bit",
    "hex2int",
    "hex2oct",
    "hex2str",
    "hostid",
    "int2bit",
    "int2char",
    "int2enum",
    "int2float",
    "int2hex",
    "int2oct",
    "int2str",
    "int2unichar",
    "isbound",
    "ischosen",
    "ispresent",
    "istemplatekind",
    "isvalue",
    "lengthof",
    "oct2bit",
    "oct2char",
    "oct2hex",
    "oct2int",
    "oct2str",
    "oct2unichar",
    "regexp",
    "remove_bom",
    "replace",
    "rnd",
    "sizeof",
    "sizeoftype",
    "str2float",
    "str2hex",
    "str2int",
    "str2oct",
    "substr",
    "testcasename",
    "unichar2int",
    "unichar2oct",
];

/// Whether `name` is a word the export does not let a name take
/// ([`RESERVED`]).
pub fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name)
}

/// What every refusal of a name says the rule is.
const NAME_RULE: &str = "the export writes every name as it is written \
                         (docs/SCE_TEST_PROFILE.md §9.5), so a name it cannot write is \
                         refused rather than renamed";

/// The document a name the export writes is written in, as a refusal of the
/// name places it.
#[derive(Debug, Clone)]
pub(super) struct Origin {
    pub(super) kind: ForgeKind,
    /// The document's name, as its own refusals name it.
    pub(super) document: String,
    /// The label its refusals carry: its path.
    pub(super) label: String,
    /// The row the name is written on, where the model keeps one.
    pub(super) line: Option<u32>,
}

impl Origin {
    /// This document, on `line`.
    pub(super) fn at(&self, line: Option<u32>) -> Self {
        Self {
            line: line.or(self.line),
            ..self.clone()
        }
    }
}

/// One name the export writes: what it names, as a refusal says it ("the
/// record", "the formal parameter"), where it is written, and whether the
/// export made it rather than read it from a document.
#[derive(Debug, Clone)]
pub(super) struct Declared {
    pub(super) what: &'static str,
    pub(super) origin: Origin,
    pub(super) made_by_export: bool,
}

impl Declared {
    /// A name read from the document at `origin`.
    pub(super) fn read(what: &'static str, origin: Origin) -> Self {
        Self {
            what,
            origin,
            made_by_export: false,
        }
    }

    /// A name the export makes for the document at `origin`: an integer type
    /// of the type grammar, a module, a codec function or its parameter.
    pub(super) fn made(what: &'static str, origin: Origin) -> Self {
        Self {
            what,
            origin,
            made_by_export: true,
        }
    }

    fn spelled(&self, name: &str) -> String {
        format!("{} '{name}'", self.what)
    }
}

/// The refusal of `name`, declared as `at`, for `detail`.
fn refusal(name: &str, at: &Declared, detail: String) -> Located<ForgeError> {
    Located::new(
        ValidationError::TestProfile {
            rule: TestProfileRule::ExportName,
            kind: at.origin.kind,
            document: at.origin.document.clone(),
            actual: name.to_string(),
            detail,
        }
        .into(),
        at.origin.label.clone(),
        at.origin.line,
        None,
    )
}

/// Refuse `name` when the export cannot write it as it is written at all: no
/// identifier, or a reserved word. Every name the export writes passes here,
/// a field's and a union alternative's among them, which live in a scope of
/// their own type and are asked nothing more.
pub(super) fn written(name: &str, at: &Declared) -> Result<(), Located<ForgeError>> {
    if !is_identifier(name) {
        return Err(refusal(
            name,
            at,
            format!(
                "{} is not a TTCN-3 identifier, a letter followed by letters, digits and \
                 underscores (ES 201 873-1 Annex A): {NAME_RULE}",
                at.spelled(name)
            ),
        ));
    }
    if is_reserved(name) {
        return Err(refusal(
            name,
            at,
            format!(
                "{} is a reserved word of TTCN-3 (ES 201 873-1 Annex A): {NAME_RULE}",
                at.spelled(name)
            ),
        ));
    }
    Ok(())
}

/// The names the export writes in the scopes of its modules, which TTCN-3
/// holds to one rule: an identifier is not declared again in a scope that
/// sees it, nested or not (ES 201 873-1 §5.2.2).
///
/// - A **definition** of either module (a type, a codec function, a port
///   type, a template, a module itself) is unique among every name below.
///   The two modules are one scope here: the behaviour module imports the
///   types module whole.
/// - An **enumerated value** may be the value of another enumerated type
///   too, and is no other name.
/// - A **nested** name (a formal parameter) may be repeated in another
///   definition's scope, and is no definition's name or enumerated value.
#[derive(Debug, Default)]
pub(super) struct Namespace {
    definitions: BTreeMap<String, Declared>,
    values: BTreeMap<String, Declared>,
    nested: BTreeMap<String, Declared>,
}

impl Namespace {
    /// A definition of one of the modules.
    pub(super) fn definition(
        &mut self,
        name: &str,
        declared: Declared,
    ) -> Result<(), Located<ForgeError>> {
        written(name, &declared)?;
        let earlier = self
            .definitions
            .get(name)
            .or_else(|| self.values.get(name))
            .or_else(|| self.nested.get(name));
        if let Some(earlier) = earlier {
            return Err(collision(name, earlier, &declared));
        }
        self.definitions.insert(name.to_string(), declared);
        Ok(())
    }

    /// A value of an enumerated type.
    pub(super) fn value(
        &mut self,
        name: &str,
        declared: Declared,
    ) -> Result<(), Located<ForgeError>> {
        written(name, &declared)?;
        let earlier = self.definitions.get(name).or_else(|| self.nested.get(name));
        if let Some(earlier) = earlier {
            return Err(collision(name, earlier, &declared));
        }
        self.values.entry(name.to_string()).or_insert(declared);
        Ok(())
    }

    /// A name of a definition's own scope: a formal parameter.
    pub(super) fn nested(
        &mut self,
        name: &str,
        declared: Declared,
    ) -> Result<(), Located<ForgeError>> {
        written(name, &declared)?;
        let earlier = self.definitions.get(name).or_else(|| self.values.get(name));
        if let Some(earlier) = earlier {
            return Err(collision(name, earlier, &declared));
        }
        self.nested.entry(name.to_string()).or_insert(declared);
        Ok(())
    }
}

/// The refusal of `name`, declared as `earlier` and again as `later`: placed
/// at the name an author wrote, the later one where both are.
fn collision(name: &str, earlier: &Declared, later: &Declared) -> Located<ForgeError> {
    let (refused, other) = if later.made_by_export && !earlier.made_by_export {
        (earlier, later)
    } else {
        (later, earlier)
    };
    let elsewhere = if other.origin.label == refused.origin.label {
        String::new()
    } else {
        format!(" of {}", other.origin.label)
    };
    refusal(
        name,
        refused,
        format!(
            "{} is one TTCN-3 identifier with {}{elsewhere}, in a scope that sees both \
             (ES 201 873-1 §5.2.2): {NAME_RULE}",
            refused.spelled(name),
            other.spelled(name)
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identifier_is_a_letter_then_letters_digits_and_underscores() {
        for name in ["a", "job_status", "Job2", "x_1_"] {
            assert!(is_identifier(name), "{name}");
        }
        for name in ["", "_job", "2nd", "job-status", "job.status", "caf\u{e9}"] {
            assert!(!is_identifier(name), "{name}");
        }
    }

    /// Every reserved word is an identifier, so the two refusals never
    /// overlap, and none is listed twice.
    #[test]
    fn every_reserved_word_is_an_identifier_listed_once() {
        let mut seen = std::collections::BTreeSet::new();
        for word in RESERVED {
            assert!(is_identifier(word), "{word}");
            assert!(seen.insert(*word), "{word} is listed twice");
        }
        for word in [
            "error", "pass", "timeout", "value", "lengthof", "now", "class",
        ] {
            assert!(is_reserved(word), "{word}");
        }
        for word in ["job_status", "limit", "count", "tag", "note"] {
            assert!(!is_reserved(word), "{word}");
        }
    }

    #[test]
    fn a_unit_name_is_the_stem_in_snake_case() {
        assert_eq!(unit_name("event-schema"), "event_schema");
        assert_eq!(unit_name("job_status"), "job_status");
    }

    fn origin(document: &str) -> Origin {
        Origin {
            kind: ForgeKind::EventSchema,
            document: document.to_string(),
            label: format!("{document}.scxml"),
            line: Some(3),
        }
    }

    /// A definition and an enumerated value, or a definition and a formal
    /// parameter, are one name; two values of two enumerated types, or two
    /// parameters of two templates, are not.
    #[test]
    fn a_name_is_refused_where_a_scope_sees_it_twice() {
        let mut names = Namespace::default();
        names
            .definition(
                "job_status",
                Declared::read("the record", origin("job_status")),
            )
            .expect("a first definition");
        names
            .value("ok", Declared::read("the variant", origin("result")))
            .expect("a value");
        names
            .value("ok", Declared::read("the variant", origin("outcome")))
            .expect("a value of another enumerated type");
        names
            .nested("limit", Declared::read("the formal parameter", origin("a")))
            .expect("a parameter");
        names
            .nested("limit", Declared::read("the formal parameter", origin("b")))
            .expect("a parameter of another template");
        for refused in [
            names.definition("ok", Declared::read("the record", origin("ok"))),
            names.definition("limit", Declared::read("the template", origin("limit"))),
            names.value("job_status", Declared::read("the variant", origin("x"))),
            names.nested(
                "job_status",
                Declared::read("the formal parameter", origin("y")),
            ),
            names.definition("job_status", Declared::read("the template", origin("t"))),
        ] {
            let refused = refused.expect_err("one name twice in one scope");
            assert!(
                matches!(
                    refused.error,
                    ForgeError::Validation(ref v) if matches!(
                        **v,
                        ValidationError::TestProfile { rule: TestProfileRule::ExportName, .. }
                    )
                ),
                "{refused:?}"
            );
        }
    }

    /// A name the export made and one an author wrote, one identifier: the
    /// refusal stands at the author's.
    #[test]
    fn a_collision_with_a_made_name_stands_at_the_written_one() {
        let mut names = Namespace::default();
        names
            .definition("uint8", Declared::made("the integer type", origin("suite")))
            .expect("the export's own");
        let refused = names
            .definition("uint8", Declared::read("the record", origin("uint8")))
            .expect_err("a record named as an integer type");
        assert_eq!(refused.location.file, "uint8.scxml");
        let mut names = Namespace::default();
        names
            .definition("msg", Declared::read("the record", origin("msg")))
            .expect("a record");
        let refused = names
            .nested(
                "msg",
                Declared::made("the codec parameter", origin("suite")),
            )
            .expect_err("a codec parameter named as a record");
        assert_eq!(refused.location.file, "msg.scxml");
    }
}
