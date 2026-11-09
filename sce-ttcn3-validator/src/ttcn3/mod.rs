// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The TTCN-3 the export writes, read into a tree ([`ast`]) by a tokenizer
//! ([`lexer`]) and a recursive descent ([`parser`]) of this crate's own, and
//! the places of that tree in the witness's vocabulary ([`places`]).
//!
//! The reader takes exactly the constructs the export writes (§9.5 and the
//! templates under `tools/codegen/templates/ttcn3/`). A module that holds
//! anything else is refused at the row it stands on, so a construct the
//! reader does not know is never a place it silently leaves out.

pub mod ast;
pub mod lexer;
pub mod parser;

use std::fmt;

use crate::place::{extended, Segment};
use ast::{Definition, Member, Module, Statement};

/// Why a module is not read: the row, and what stood there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: u32,
    pub message: String,
}

impl ParseError {
    pub fn new(line: u32, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "row {}: {}", self.line, self.message)
    }
}

/// The module `text` is.
pub fn parse(text: &str) -> Result<Module, ParseError> {
    parser::Parser::new(lexer::lex(text)?).module()
}

/// Every place of `module`, in textual order: each definition, each member
/// of a component type, each local, label and statement of a function, each
/// statement inside an `if` and each alternative of an `alt` and statement
/// of its block, and each statement of the control part, which the module's
/// name names.
pub fn places(module: &Module) -> Vec<Vec<Segment>> {
    let mut places = Vec::new();
    for definition in &module.definitions {
        match definition {
            Definition::Type { name, .. } => places.push(vec![Segment::Type(name.clone())]),
            Definition::ExternalFunction { name, .. } => {
                places.push(vec![Segment::ExternalFunction(name.clone())])
            }
            Definition::PortType { name, .. } => places.push(vec![Segment::PortType(name.clone())]),
            Definition::Template { name, .. } => places.push(vec![Segment::Template(name.clone())]),
            Definition::Modulepar { name, .. } => {
                places.push(vec![Segment::Modulepar(name.clone())])
            }
            Definition::ComponentType { name, members } => {
                let at = vec![Segment::ComponentType(name.clone())];
                places.push(at.clone());
                for member in members {
                    let name = member.name().to_string();
                    let segment = match member {
                        Member::Port { .. } => Segment::Port(name),
                        Member::Timer { .. } => Segment::Timer(name),
                        Member::Var { .. } => Segment::Var(name),
                    };
                    places.push(extended(&at, segment));
                }
            }
            Definition::Function {
                name, locals, body, ..
            } => {
                let at = vec![Segment::Function(name.clone())];
                places.push(at.clone());
                for local in locals {
                    places.push(extended(&at, Segment::Local(local.name.clone())));
                }
                // The statements before the first label are the function's
                // own; each label opens those after it.
                let mut opened = at.clone();
                let mut position = 0;
                for statement in body {
                    if let Statement::Label(label) = statement {
                        opened = extended(&at, Segment::Label(label.clone()));
                        places.push(opened.clone());
                        position = 0;
                        continue;
                    }
                    statement_places(
                        extended(&opened, Segment::Statement(position)),
                        statement,
                        &mut places,
                    );
                    position += 1;
                }
            }
            Definition::Testcase { name, .. } => places.push(vec![Segment::Testcase(name.clone())]),
            Definition::Control { body } => {
                let at = vec![Segment::Control(module.name.clone())];
                places.push(at.clone());
                for (position, statement) in body.iter().enumerate() {
                    statement_places(
                        extended(&at, Segment::Statement(position)),
                        statement,
                        &mut places,
                    );
                }
            }
        }
    }
    places
}

/// The place `path` of `statement`, and the places inside it.
fn statement_places(path: Vec<Segment>, statement: &Statement, places: &mut Vec<Vec<Segment>>) {
    places.push(path.clone());
    match statement {
        Statement::If {
            then, otherwise, ..
        } => {
            for (position, inner) in then.iter().enumerate() {
                statement_places(extended(&path, Segment::Then(position)), inner, places);
            }
            for (position, inner) in otherwise.iter().enumerate() {
                statement_places(extended(&path, Segment::Else(position)), inner, places);
            }
        }
        Statement::Alt(alternatives) => {
            for (position, alternative) in alternatives.iter().enumerate() {
                let opened = extended(&path, Segment::Alternative(position));
                places.push(opened.clone());
                for (index, inner) in alternative.block.iter().enumerate() {
                    statement_places(extended(&opened, Segment::Statement(index)), inner, places);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::path_text;

    fn texts(module: &Module) -> Vec<String> {
        places(module).iter().map(|path| path_text(path)).collect()
    }

    /// A function's places: the definition, its locals, the statements
    /// before its first label as its own, each label and the statements it
    /// opens, an `if`'s two blocks, an `alt`'s alternatives and their
    /// blocks; the start and the kill of a PTC are one statement each.
    #[test]
    fn a_functions_places_follow_its_labels_and_blocks() {
        let module = parse(
            "// source-hash: 0123
module s {
import from s_Types all;
function f_c() runs on c {
  var helper v_helper := null;
  var job v_job;
  p.send(t(x));
  goto a;
  label a;
  if (v_helper == null) {
    v_helper := helper.create(\"helper\") alive;
    connect(mtc:ctl, v_helper:ctl);
    if (v_other != null) {
      connect(v_other:p, v_helper:q);
    }
    v_helper.start(f_helper());
  }
  if (x == 1) {
    setverdict(pass);
    return;
  } else {
    log(\"no\", x);
  }
  alt {
    [] p.receive(t(x)) -> value v_job {
      if (v_helper != null) {
        v_helper.kill;
        v_helper := null;
      }
      x := v_job.n;
      goto b;
    }
    [x > 0] T.timeout {
      return;
    }
  }
  label b;
  alt {
  }
}
control {
  execute(t1(), 10.0);
  if (PICS_A and (PICS_B or PICS_C)) {
    execute(t2());
  }
}
}",
        )
        .expect("a module");
        assert_eq!(module.imports, ["s_Types"]);
        assert_eq!(
            texts(&module),
            [
                r#"[{"function":"f_c"}]"#,
                r#"[{"function":"f_c"},{"local":"v_helper"}]"#,
                r#"[{"function":"f_c"},{"local":"v_job"}]"#,
                r#"[{"function":"f_c"},{"statement":0}]"#,
                r#"[{"function":"f_c"},{"statement":1}]"#,
                r#"[{"function":"f_c"},{"label":"a"}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":0}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":1}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":1},{"then":0}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":1},{"then":1}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":1},{"else":0}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":2}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":2},{"alternative":0}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":2},{"alternative":0},{"statement":0}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":2},{"alternative":0},{"statement":1}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":2},{"alternative":0},{"statement":2}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":2},{"alternative":1}]"#,
                r#"[{"function":"f_c"},{"label":"a"},{"statement":2},{"alternative":1},{"statement":0}]"#,
                r#"[{"function":"f_c"},{"label":"b"}]"#,
                r#"[{"function":"f_c"},{"label":"b"},{"statement":0}]"#,
                r#"[{"control":"s"}]"#,
                r#"[{"control":"s"},{"statement":0}]"#,
                r#"[{"control":"s"},{"statement":1}]"#,
            ]
        );
        let Definition::Function { body, .. } = &module.definitions[0] else {
            panic!("the function: {:?}", module.definitions);
        };
        let Statement::Create { connections, .. } = &body[3] else {
            panic!("the start of the PTC: {:?}", body[3]);
        };
        assert_eq!(connections.len(), 2);
        assert_eq!(connections[1].when.as_deref(), Some("v_other"));
    }

    /// The types module's places are its types and its codec functions.
    #[test]
    fn a_types_modules_places_are_its_definitions() {
        let module = parse(
            "module s_Types {
type integer int8 (-128 .. 127);
type record status {
  uint32 elapsed_ms,
  universal charstring note optional, // at most 32 bytes of UTF-8
  record length(0 .. 4) of uint8 codes
}
type union payment {
  card card,
  record {} cash
}
type enumerated color { red(0), green(1) }
type record nothing {}
external function enc_status(in status msg) return octetstring;
external function dec_status(inout octetstring octets, out status msg) return integer;
}",
        )
        .expect("a module");
        assert_eq!(
            texts(&module),
            [
                r#"[{"type":"int8"}]"#,
                r#"[{"type":"status"}]"#,
                r#"[{"type":"payment"}]"#,
                r#"[{"type":"color"}]"#,
                r#"[{"type":"nothing"}]"#,
                r#"[{"external-function":"enc_status"}]"#,
                r#"[{"external-function":"dec_status"}]"#,
            ]
        );
    }

    /// Every matcher the export writes is read.
    #[test]
    fn every_matcher_is_read() {
        let module = parse(
            "module s {
template uint32 at_most(uint32 limit) := (-infinity .. limit);
template job every(uint32 limit, template (omit) uint8 tag) := {
  elapsed_ms := (1, (10 .. limit)),
  note := ? length(0 .. 8) ifpresent,
  codes := { 3, ?, complement(0) } length(1 .. 4),
  tag := *,
  retry := omit,
  count := at_most(limit),
  text := \"a\" & char(0, 0, 0, 233),
  key := '0AFF'O,
  lines := { { sku := \"A-1\", vip := true }, exact } length(0 .. infinity),
  payment := ({ card := { last4 := ? length(4 .. infinity) } }, { cash := {} }),
  empty := {},
  low := -5
};
modulepar octetstring key := '0AFF'O; // at most 2 octets
modulepar boolean PICS_A;
type port p message {
  inout job;
  in other;
}
type component c {
  port p sut;
  timer t := 0.5;
  timer u;
  var uint8 n := 0;
}
testcase t1() runs on c system sys {
  map(mtc:sut, system:sut);
  f_c();
}
}",
        )
        .expect("a module");
        assert_eq!(
            texts(&module),
            [
                r#"[{"template":"at_most"}]"#,
                r#"[{"template":"every"}]"#,
                r#"[{"modulepar":"key"}]"#,
                r#"[{"modulepar":"PICS_A"}]"#,
                r#"[{"port-type":"p"}]"#,
                r#"[{"component-type":"c"}]"#,
                r#"[{"component-type":"c"},{"port":"sut"}]"#,
                r#"[{"component-type":"c"},{"timer":"t"}]"#,
                r#"[{"component-type":"c"},{"timer":"u"}]"#,
                r#"[{"component-type":"c"},{"var":"n"}]"#,
                r#"[{"testcase":"t1"}]"#,
            ]
        );
        let Definition::Template { matcher, .. } = &module.definitions[0] else {
            panic!("the template");
        };
        assert_eq!(matcher, "(-infinity .. limit)");
    }

    /// What the export does not write is refused at its row, never skipped.
    #[test]
    fn a_construct_outside_the_subset_is_refused_at_its_row() {
        let error = parse(
            "module s {
function f() runs on c {
  label a;
  repeat;
}
}",
        )
        .expect_err("refused");
        assert_eq!(error.line, 4);
        let error = parse("module s {\nsignature x();\n}").expect_err("refused");
        assert_eq!(error.line, 2);
        assert!(parse("module s {\n").is_err());
        assert!(parse("module s {}\nmodule t {}").is_err());
        assert!(parse("module s {\nfunction f() runs on c {\n  p.call(x);\n}\n}").is_err());
    }
}
