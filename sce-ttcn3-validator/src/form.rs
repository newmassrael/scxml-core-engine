// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The second check of docs/SCE_TEST_PROFILE.md §9.4: each entry's rule
//! applies to its construct's kind (`rule-kind`), and every place the entry
//! names has the TTCN-3 form that rule writes for that construct, read from
//! the documents (`form`).
//!
//! The structure is compared, not the text: the definition or the statement
//! the reader of the modules found at the place, against what the document
//! writes. A type has its kind, its name and its fields, variants or
//! alternatives (E1); a template its name, its type and its formal
//! parameters (E2); a port type its messages and their directions (E3); a
//! component type its ports, timers and variables by name and type (E4); a
//! function runs on its component (E5); an atomic state is a label and a
//! guarded eventless transition an `if` (E6); an atomic state that waits an
//! `alt` (E7); a branch an alternative that waits for its port and template,
//! its timer or its PTC, with a guard where it has a `cond` (E8); a send is
//! sent on its port with its template (E9); a timer is started with the
//! duration the document writes, as the same number of seconds, or stopped
//! (E10); a PTC is created, started and killed by its reference (E11); a
//! top-level `<final>` returns (E12); a verdict is set to its value and a
//! log logs its label (E13); a test case runs on its MTC's component type in
//! its configuration's system (E14); and the control part executes each
//! test case with its guard (E15). An expression is not compared: the export
//! writes it from a typed tree this validator does not build, and its
//! presence is held where the document writes one.
//!
//! A statement, a label or a local stands in a function of its component,
//! and a member of a component type in its component's type.

use crate::documents::{Detail, Document, DocumentKind, Documents, Form, Trigger};
use crate::place::Segment;
use crate::rules::{self, Action, Kind, Rule};
use crate::ttcn3::ast::{
    Alternative, Call, Connection, Definition, Event, Field, Local, Member, Message, Module,
    Parameter, Statement, TypeBody, TypeExpr,
};
use crate::ttcn3::{self, lexer, Found};
use crate::types::{self, Written, INTEGERS};
use crate::violation::Violation;
use crate::witness::{Construct, Witness};
use crate::{stem, unit_name};

type Held = Result<(), String>;

/// The violations of the second check: of each entry of `witness` whose
/// construct the documents hold, against `modules`, the modules that parsed.
pub fn check(documents: &Documents, modules: &[&Module], witness: &Witness) -> Vec<Violation> {
    let mut violations = Vec::new();
    for entry in &witness.entries {
        // A construct the documents do not hold is the first check's
        // (`construct-dangling`).
        let Some(kind) = documents.kind_of(&entry.construct) else {
            continue;
        };
        if !rules::applies(kind, entry.rule) {
            violations.push(Violation::RuleKind {
                construct: entry.construct.clone(),
                rule: entry.rule,
                kind,
            });
            continue;
        }
        let Some(document) = documents.document(&entry.construct.document) else {
            continue;
        };
        let forms = Forms {
            documents,
            document,
            node: documents.node(&entry.construct).map(|(_, index)| index),
            construct: &entry.construct,
            kind,
            rule: entry.rule,
        };
        for location in &entry.produced {
            let Some(module) = modules.iter().find(|module| module.name == location.module) else {
                continue;
            };
            // A place the module does not hold is the first check's
            // (`location-dangling`).
            let Some(found) = ttcn3::find(module, &location.path) else {
                continue;
            };
            if let Err(reason) = forms.holds(&location.path, found) {
                violations.push(Violation::Form {
                    module: location.module.clone(),
                    path: location.path.clone(),
                    construct: entry.construct.clone(),
                    rule: entry.rule,
                    reason,
                });
            }
        }
    }
    violations
}

/// The unit name of `document`: its file's stem in lower snake case, the
/// name of every definition the export writes of a document (§9.5).
pub fn unit(document: &Document) -> String {
    unit_name(stem(&document.name))
}

/// The names of the functions `component` runs (E5): one for each
/// configuration that holds it where it invokes a PTC, and one otherwise.
pub fn function_names(documents: &Documents, component: &Document) -> Vec<String> {
    let name = unit(component);
    if !component.invokes() {
        return vec![format!("f_{name}")];
    }
    documents
        .configurations_holding(component)
        .into_iter()
        .map(|configuration| format!("f_{name}_in_{}", unit(configuration)))
        .collect()
}

/// The whole number of microseconds `number` units of `scale` microseconds
/// each are, where it is one: `number` decimal digits with at most one
/// point.
fn microseconds(number: &str, scale: u128) -> Option<u128> {
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !digits(whole) || !digits(fraction) {
        return None;
    }
    let mut us: u128 = 0;
    for digit in whole.bytes() {
        us = us.checked_mul(10)?.checked_add(u128::from(digit - b'0'))?;
    }
    us = us.checked_mul(scale)?;
    let mut place = scale;
    for digit in fraction.bytes() {
        let digit = u128::from(digit - b'0');
        if place % 10 != 0 {
            // Below a microsecond: only a zero is a whole number of them.
            if digit != 0 {
                return None;
            }
            continue;
        }
        place /= 10;
        us = us.checked_add(digit * place)?;
    }
    Some(us)
}

/// The microseconds of a delay the document writes, as the profile takes a
/// delay: a number of `ms` or `s` (docs/SCE_TEST_PROFILE.md §4.7).
fn delay(text: &str) -> Option<u128> {
    let text = text.trim().to_ascii_lowercase();
    if let Some(number) = text.strip_suffix("ms") {
        return microseconds(number, 1_000);
    }
    microseconds(text.strip_suffix('s')?, 1_000_000)
}

/// Whether the TTCN-3 `written`, a decimal of seconds, is the duration the
/// document writes as `document` (E10): the same number of microseconds.
fn same_duration(document: &str, written: &str) -> Held {
    match (delay(document), microseconds(written.trim(), 1_000_000)) {
        (Some(owed), Some(seconds)) if owed == seconds => Ok(()),
        _ => Err(format!(
            "its duration is {written} seconds, and the document writes '{document}'"
        )),
    }
}

/// The text a character string of the export is, its quadruples read as
/// their characters, where `item` is one.
fn charstring(item: &str) -> Option<String> {
    let tokens: Vec<lexer::Token> = lexer::lex(item)
        .ok()?
        .into_iter()
        .map(|lexed| lexed.token)
        .collect();
    let mut text = String::new();
    let mut at = 0;
    loop {
        match tokens.get(at) {
            Some(lexer::Token::Text(part)) => {
                text.push_str(part);
                at += 1;
            }
            Some(lexer::Token::Word(word)) if word == "char" => {
                let quadruple = tokens.get(at + 1..at + 10)?;
                let mut code: u32 = 0;
                for (position, token) in quadruple.iter().enumerate() {
                    let punct = match position {
                        0 => "(",
                        8 => ")",
                        _ if position % 2 == 0 => ",",
                        _ => "",
                    };
                    match token {
                        lexer::Token::Punct(written) if !punct.is_empty() && *written == punct => {}
                        lexer::Token::Integer(digits) if punct.is_empty() => {
                            code = code.checked_mul(256)?.checked_add(digits.parse().ok()?)?;
                        }
                        _ => return None,
                    }
                }
                text.push(char::from_u32(code)?);
                at += 10;
            }
            _ => return None,
        }
        match tokens.get(at) {
            None => return Some(text),
            Some(lexer::Token::Punct("&")) => at += 1,
            Some(_) => return None,
        }
    }
}

/// What a place holds, as a finding names it.
fn what(found: Found<'_>) -> String {
    fn called(kind: &str, name: &str) -> String {
        format!("{kind} '{name}'")
    }
    match found {
        Found::Definition(definition) => match definition {
            Definition::Type { name, .. } => called("the type", name),
            Definition::ExternalFunction { name, .. } => called("the external function", name),
            Definition::PortType { name, .. } => called("the port type", name),
            Definition::Template { name, .. } => called("the template", name),
            Definition::Modulepar { name, .. } => called("the module parameter", name),
            Definition::ComponentType { name, .. } => called("the component type", name),
            Definition::Function { name, .. } => called("the function", name),
            Definition::Testcase { name, .. } => called("the test case", name),
            Definition::Control { .. } => "the control part".to_string(),
        },
        Found::Member(member) => match member {
            Member::Port { name, .. } => called("the port", name),
            Member::Timer { name, .. } => called("the timer", name),
            Member::Var { name, .. } => called("the variable", name),
        },
        Found::Local(local) => called("the local", &local.name),
        Found::Label(label) => called("the label", label),
        Found::Alternative(_) => "an alternative".to_string(),
        Found::Statement(statement) => match statement {
            Statement::Label(label) => called("the label", label),
            Statement::Goto(label) => format!("goto {label}"),
            Statement::Return => "a return".to_string(),
            Statement::Send { port, .. } => format!("a send on {port}"),
            Statement::StartTimer { timer, .. } => format!("{timer}.start"),
            Statement::StopTimer { timer } => format!("{timer}.stop"),
            Statement::Setverdict { verdict, .. } => format!("setverdict({verdict})"),
            Statement::Log(_) => "a log".to_string(),
            Statement::Assign { location, .. } => format!("an assignment of {location}"),
            Statement::If { .. } => "an if".to_string(),
            Statement::Alt(_) => "an alt".to_string(),
            Statement::Create { reference, .. } => format!("the creation of {reference}"),
            Statement::Kill { reference } => format!("the kill of {reference}"),
            Statement::CreateAssign { reference, .. } => format!("{reference} := create"),
            Statement::Connect(connection) => format!("a {}", connection.operation),
            Statement::StartComponent { reference, .. } => format!("{reference}.start"),
            Statement::KillComponent { reference } => format!("{reference}.kill"),
            Statement::Call(function) => format!("a call of {function}"),
            Statement::Execute { test_case, .. } | Statement::SelectedExecute { test_case, .. } => {
                format!("an execute of {test_case}")
            }
        },
    }
}

/// `found` is not `owed`.
fn wrong(found: Found<'_>, owed: &str) -> Held {
    Err(format!("it is {}, where {owed} is owed", what(found)))
}

/// `written` is the name `owed`.
fn named(what: &str, written: &str, owed: &str) -> Held {
    if written == owed {
        Ok(())
    } else {
        Err(format!(
            "{what} is '{written}', and the document names it '{owed}'"
        ))
    }
}

/// Whether two numbers the document and the export write are one.
fn same_number(a: &str, b: &str) -> bool {
    match (a.trim().parse::<i128>(), b.trim().parse::<i128>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a.trim() == b.trim(),
    }
}

/// The construct of one entry, and the documents its forms are read from.
struct Forms<'d> {
    documents: &'d Documents,
    /// The construct's document.
    document: &'d Document,
    /// The construct's node, where it is a node of the document: every
    /// construct but the local of a received message.
    node: Option<usize>,
    construct: &'d Construct,
    kind: Kind,
    rule: Rule,
}

impl<'d> Forms<'d> {
    fn detail(&self) -> &'d Detail {
        static BARE: Detail = Detail::Bare;
        self.node
            .map_or(&BARE, |index| &self.document.nodes[index].detail)
    }

    /// The unit name of the document `from` imports as `alias`, of `kind`.
    fn imported(&self, from: &Document, alias: &str, kind: DocumentKind) -> Result<String, String> {
        self.documents
            .imported(from, alias, kind)
            .map(unit)
            .ok_or_else(|| {
                format!(
                    "'{alias}' names no {} {} imports",
                    kind.as_attr(),
                    from.name
                )
            })
    }

    /// The written type `text` of `document`, its aliases its imports.
    fn written(&self, document: &Document, text: &str) -> Result<Written, String> {
        let named = |kind: DocumentKind, alias: &str| -> Option<String> {
            self.documents.imported(document, alias, kind).map(unit)
        };
        types::read(text, &named)
    }

    /// The place `path` stands in a function of the construct's component.
    fn in_function(&self, path: &[Segment]) -> Held {
        let names = function_names(self.documents, self.document);
        match path.first() {
            Some(Segment::Function(name)) if names.contains(name) => Ok(()),
            _ => Err(format!(
                "it stands outside the functions of {} ({})",
                self.document.name,
                names.join(", ")
            )),
        }
    }

    /// The place `path` stands in the component type of the construct's
    /// document.
    fn in_component_type(&self, path: &[Segment]) -> Held {
        let owed = unit(self.document);
        match path.first() {
            Some(Segment::ComponentType(name)) if *name == owed => Ok(()),
            _ => Err(format!("it stands outside the component type {owed}")),
        }
    }

    /// Whether `found`, at `path`, has the form the entry's rule writes for
    /// its construct.
    fn holds(&self, path: &[Segment], found: Found<'_>) -> Held {
        match (self.kind, self.rule) {
            (Kind::Suite, Rule::E1) => integer_type(found),
            (Kind::Types, Rule::E1) => self.types(found),
            (Kind::Template, Rule::E2) => self.template(found),
            (Kind::PortType, Rule::E3) => self.port_type(found),
            (Kind::Component, Rule::E4) | (Kind::Configuration, Rule::E14) => match found {
                Found::Definition(Definition::ComponentType { name, .. }) => {
                    named("the component type", name, &unit(self.document))
                }
                _ => wrong(found, "a component type"),
            },
            (Kind::Component, Rule::E5) => self.function(found),
            (Kind::Component, Rule::E8) => {
                self.in_function(path)?;
                match found {
                    Found::Statement(Statement::Goto(_)) => Ok(()),
                    _ => wrong(found, "the goto to the initial atomic state"),
                }
            }
            (Kind::Port, Rule::E4)
            | (Kind::Timer, Rule::E4)
            | (Kind::Variable, Rule::E4)
            | (Kind::SystemPort, Rule::E14) => {
                self.in_component_type(path)?;
                self.member(found)
            }
            (Kind::AtomicState, Rule::E6) => {
                self.in_function(path)?;
                self.label(found)
            }
            (Kind::AtomicState, Rule::E7) => {
                self.in_function(path)?;
                match found {
                    Found::Statement(Statement::Alt(_)) => Ok(()),
                    _ => wrong(found, "an alt"),
                }
            }
            (Kind::TopLevelFinal, Rule::E12) => {
                self.in_function(path)?;
                match found {
                    Found::Statement(Statement::Return) => Ok(()),
                    _ => wrong(found, "a return"),
                }
            }
            (Kind::Invoke, Rule::E11) => {
                self.in_function(path)?;
                self.invoke(found)
            }
            (Kind::Transition, Rule::E6) => {
                self.in_function(path)?;
                match (found, self.detail()) {
                    (
                        Found::Statement(Statement::If { otherwise, .. }),
                        Detail::Transition { guarded: true, .. },
                    ) if otherwise.is_empty() => Ok(()),
                    _ => wrong(found, "an if of the transition's guard, with no else"),
                }
            }
            (Kind::Transition, Rule::E8) => {
                self.in_function(path)?;
                match found {
                    Found::Alternative(alternative) => self.alternative(alternative),
                    Found::Statement(Statement::Goto(_)) => Ok(()),
                    _ => wrong(found, "an alternative or a goto"),
                }
            }
            (Kind::Action(action), _) => {
                self.in_function(path)?;
                self.action(action, found)
            }
            (Kind::Elseif, Rule::E8) => {
                self.in_function(path)?;
                match found {
                    Found::Statement(Statement::If { .. }) => Ok(()),
                    _ => wrong(found, "an if"),
                }
            }
            (Kind::Received, Rule::E8) => {
                self.in_function(path)?;
                self.received(found)
            }
            (Kind::TestCase, Rule::E14) => self.test_case(found),
            (Kind::Suite, Rule::E15) => match found {
                Found::Definition(Definition::Control { .. }) => Ok(()),
                _ => wrong(found, "the control part"),
            },
            (Kind::ModuleParameter, Rule::E15) | (Kind::Feature, Rule::E15) => {
                self.module_parameter(found)
            }
            (Kind::Execute, Rule::E15) => self.execute(found),
            // `rules::applies` admits no other pair.
            _ => Ok(()),
        }
    }

    /// A type of an event schema, an enum or a union, or a codec function of
    /// an event schema (E1).
    fn types(&self, found: Found<'_>) -> Held {
        let owed = unit(self.document);
        let (name, body) = match found {
            Found::Definition(Definition::Type { name, body }) => (name, body),
            Found::Definition(Definition::ExternalFunction {
                name,
                parameters,
                returns,
            }) if matches!(self.document.form, Form::Record(_)) => {
                return codec(name, parameters, returns, &owed);
            }
            _ => return wrong(found, &format!("the type {owed}")),
        };
        named("the type", name, &owed)?;
        match (&self.document.form, body) {
            (Form::Record(fields), TypeBody::Record(written)) => {
                let mut declared = Vec::new();
                for field in fields {
                    let ty = self.written(self.document, &field.ty)?;
                    declared.push(Field {
                        ty: ty.ty,
                        name: field.name.clone(),
                        optional: ty.optional,
                    });
                }
                if *written == declared {
                    Ok(())
                } else {
                    Err(format!(
                        "its fields are {}, and the event schema declares {}",
                        fields_text(written),
                        fields_text(&declared)
                    ))
                }
            }
            (Form::Enumerated(variants), TypeBody::Enumerated(written)) => {
                let same = variants.len() == written.len()
                    && variants.iter().zip(written).all(|(declared, written)| {
                        declared.0 == written.0 && same_number(&declared.1, &written.1)
                    });
                if same {
                    Ok(())
                } else {
                    Err(format!(
                        "its variants are {}, and the enum declares {}",
                        variants_text(written),
                        variants_text(variants)
                    ))
                }
            }
            (Form::Union { tag, cases }, TypeBody::Union(written)) => {
                let declared = self.alternatives(tag, cases)?;
                if *written == declared {
                    Ok(())
                } else {
                    Err(format!(
                        "its alternatives are {}, and the union declares {}",
                        fields_text(written),
                        fields_text(&declared)
                    ))
                }
            }
            (Form::Record(_), _) => wrong(found, "a record type"),
            (Form::Enumerated(_), _) => wrong(found, "an enumerated type"),
            (Form::Union { .. }, _) => wrong(found, "a union type"),
            _ => wrong(found, "no type"),
        }
    }

    /// The alternatives of a union whose tag is `tag`: one per variant of
    /// the tag's enum, in its order, each the record its case holds or an
    /// empty record.
    fn alternatives(
        &self,
        tag: &str,
        cases: &[(String, Option<String>)],
    ) -> Result<Vec<Field>, String> {
        let alias = tag
            .trim()
            .strip_prefix("enum:")
            .map(str::trim)
            .unwrap_or_default();
        let variants = match self
            .documents
            .imported(self.document, alias, DocumentKind::Enum)
            .map(|document| &document.form)
        {
            Some(Form::Enumerated(variants)) => variants,
            _ => return Err(format!("its tag's type '{tag}' names no enum it imports")),
        };
        let mut alternatives = Vec::new();
        for (variant, _) in variants {
            let record = cases
                .iter()
                .find(|(case, _)| case == variant)
                .and_then(|(_, record)| record.as_deref());
            let ty = match record {
                Some(record) => TypeExpr::Named(self.imported(
                    self.document,
                    record,
                    DocumentKind::EventSchema,
                )?),
                None => TypeExpr::Record(Vec::new()),
            };
            alternatives.push(Field {
                ty,
                name: variant.clone(),
                optional: false,
            });
        }
        Ok(alternatives)
    }

    /// A template, its type and its formal parameters (E2).
    fn template(&self, found: Found<'_>) -> Held {
        let Found::Definition(Definition::Template {
            name,
            ty,
            parameters,
            ..
        }) = found
        else {
            return wrong(found, "a template");
        };
        named("the template", name, &unit(self.document))?;
        let Form::Template {
            ty: declared,
            parameters: formal,
        } = &self.document.form
        else {
            return Ok(());
        };
        let declared = self.written(self.document, declared)?;
        if *ty != declared.ty {
            return Err(format!(
                "its type is {ty:?}, and the template is of {:?}",
                declared.ty
            ));
        }
        let mut owed = Vec::new();
        for parameter in formal {
            let written = self.written(self.document, &parameter.ty)?;
            owed.push(Parameter {
                direction: None,
                omit: written.optional,
                ty: written.ty,
                name: parameter.name.clone(),
            });
        }
        if *parameters == owed {
            Ok(())
        } else {
            Err(format!(
                "its formal parameters are {}, and the template declares {}",
                parameters_text(parameters),
                parameters_text(&owed)
            ))
        }
    }

    /// A message port type and its messages (E3).
    fn port_type(&self, found: Found<'_>) -> Held {
        let Found::Definition(Definition::PortType { name, messages }) = found else {
            return wrong(found, "a port type");
        };
        named("the port type", name, &unit(self.document))?;
        let Form::PortType(declared) = &self.document.form else {
            return Ok(());
        };
        let mut owed = Vec::new();
        for (direction, alias) in declared {
            owed.push(Message {
                direction: direction.clone(),
                ty: TypeExpr::Named(self.imported(
                    self.document,
                    alias,
                    DocumentKind::EventSchema,
                )?),
            });
        }
        if *messages == owed {
            Ok(())
        } else {
            Err(format!(
                "its messages are {}, and the port type declares {}",
                messages_text(messages),
                messages_text(&owed)
            ))
        }
    }

    /// A function that runs on the component's type (E5).
    fn function(&self, found: Found<'_>) -> Held {
        let Found::Definition(Definition::Function { name, runs_on, .. }) = found else {
            return wrong(found, "a function");
        };
        let names = function_names(self.documents, self.document);
        if !names.contains(name) {
            return Err(format!(
                "the function is '{name}', and the component's functions are {}",
                names.join(", ")
            ));
        }
        named(
            "the component type it runs on",
            runs_on,
            &unit(self.document),
        )
    }

    /// A port, a timer or a variable of a component type, or a port of a
    /// configuration's system component type (E4, E14).
    fn member(&self, found: Found<'_>) -> Held {
        let Found::Member(member) = found else {
            return wrong(found, "a member of a component type");
        };
        match (self.detail(), member) {
            (
                Detail::Port { id, port_type },
                Member::Port {
                    port_type: written,
                    name,
                },
            ) => {
                named("the port", name, id)?;
                named(
                    "its port type",
                    written,
                    &self.imported(self.document, port_type, DocumentKind::TestPortType)?,
                )
            }
            (
                Detail::Timer { id, duration },
                Member::Timer {
                    name,
                    duration: written,
                },
            ) => {
                named("the timer", name, id)?;
                match (duration, written) {
                    (None, None) => Ok(()),
                    (Some(duration), Some(written)) => same_duration(duration, written),
                    (None, Some(written)) => Err(format!(
                        "its default duration is {written}, and the timer declares none"
                    )),
                    (Some(duration), None) => Err(format!(
                        "it has no default duration, and the timer declares '{duration}'"
                    )),
                }
            }
            (
                Detail::Variable { id, ty },
                Member::Var {
                    ty: written, name, ..
                },
            ) => {
                named("the variable", name, id)?;
                let declared = self.written(self.document, ty.as_deref().unwrap_or_default())?;
                if declared.optional || *written != declared.ty {
                    return Err(format!(
                        "its type is {written:?}, and the variable is of {:?}",
                        declared.ty
                    ));
                }
                Ok(())
            }
            _ => wrong(found, &format!("the member {}", self.construct.node)),
        }
    }

    /// The label of an atomic state (E6).
    fn label(&self, found: Found<'_>) -> Held {
        let Found::Label(label) = found else {
            return wrong(found, "a label");
        };
        let state = self.document.statechart.as_ref().and_then(|chart| {
            chart
                .states
                .iter()
                .find(|state| Some(state.node) == self.node)
        });
        match state {
            Some(state) => named("the label", label, &state.id),
            None => Err("the construct is no state of the statechart".to_string()),
        }
    }

    /// The reference, the start or the kill of a PTC (E11).
    fn invoke(&self, found: Found<'_>) -> Held {
        let Detail::Invoke { id, src } = self.detail() else {
            return Ok(());
        };
        let reference = format!("v_{id}");
        let component = unit_name(stem(src));
        match found {
            Found::Local(Local {
                ty,
                name,
                starts_null,
            }) => {
                named("the reference", name, &reference)?;
                if !*starts_null || *ty != TypeExpr::Named(component.clone()) {
                    return Err(format!(
                        "the reference is not `var {component} {reference} := null`"
                    ));
                }
                Ok(())
            }
            Found::Statement(Statement::Create {
                reference: written,
                component_type,
                ptc,
                function,
                ..
            }) => {
                named("the reference", written, &reference)?;
                named("the component type created", component_type, &component)?;
                named("the PTC", ptc, id)?;
                let owed = format!("f_{component}");
                if *function == owed || function.starts_with(&format!("{owed}_in_")) {
                    Ok(())
                } else {
                    Err(format!(
                        "it starts {function}, which is no function of {component}"
                    ))
                }
            }
            Found::Statement(Statement::Kill { reference: written }) => {
                named("the reference", written, &reference)
            }
            _ => wrong(found, "the reference, the creation or the kill of the PTC"),
        }
    }

    /// The alternative a branch is (E8).
    fn alternative(&self, alternative: &Alternative) -> Held {
        let Detail::Transition { trigger, guarded } = self.detail() else {
            return Ok(());
        };
        if alternative.guard.is_some() != *guarded {
            return Err(if *guarded {
                "the alternative has no guard, and the transition has a cond".to_string()
            } else {
                "the alternative has a guard, and the transition has no cond".to_string()
            });
        }
        match (trigger, &alternative.event) {
            (
                Some(Trigger::Receive {
                    port,
                    template,
                    args,
                }),
                Event::Receive {
                    port: written,
                    template: matched,
                    redirect,
                },
            ) => {
                named("the port it receives on", written, port)?;
                match (template, matched) {
                    (None, None) if redirect.is_none() => Ok(()),
                    (None, _) => Err(
                        "it receives with a template or a redirect, and the branch receives any \
                         message"
                            .to_string(),
                    ),
                    (Some(alias), Some(Call { name, args: bound })) => {
                        named(
                            "the template it receives",
                            name,
                            &self.imported(self.document, alias, DocumentKind::TestTemplate)?,
                        )?;
                        if bound == args {
                            Ok(())
                        } else {
                            Err(format!(
                                "the template is given ({}), and the branch binds ({})",
                                bound.join(", "),
                                args.join(", ")
                            ))
                        }
                    }
                    (Some(alias), None) => Err(format!(
                        "it receives any message, and the branch receives with '{alias}'"
                    )),
                }
            }
            (Some(Trigger::Timeout { timer }), Event::Timeout { timer: written }) => {
                named("the timer it waits for", written, timer)
            }
            (Some(Trigger::Done { invoke }), Event::Done { reference }) => {
                named("the PTC it waits for", reference, &format!("v_{invoke}"))
            }
            (Some(Trigger::Receive { port, .. }), _) => Err(format!(
                "it waits for no receive, and the branch receives on {port}"
            )),
            (Some(Trigger::Timeout { timer }), _) => Err(format!(
                "it waits for no timeout, and the branch waits for {timer}"
            )),
            (Some(Trigger::Done { invoke }), _) => Err(format!(
                "it waits for no done, and the branch waits for the PTC {invoke}"
            )),
            (None, _) => Err("the transition is no branch".to_string()),
        }
    }

    /// An element of executable content (E8, E9, E10, E13).
    fn action(&self, action: Action, found: Found<'_>) -> Held {
        let Found::Statement(statement) = found else {
            return wrong(found, "a statement");
        };
        match (action, self.detail(), statement) {
            (Action::If, _, Statement::If { .. }) => Ok(()),
            (
                Action::Assign,
                Detail::Assign { location },
                Statement::Assign {
                    location: written, ..
                },
            ) => named("the location assigned", written, location),
            (
                Action::Send,
                Detail::Send {
                    port,
                    template,
                    args,
                },
                Statement::Send {
                    port: written,
                    template: Call { name, args: bound },
                },
            ) => {
                named("the port it sends on", written, port)?;
                named(
                    "the template it sends",
                    name,
                    &self.imported(self.document, template, DocumentKind::TestTemplate)?,
                )?;
                if bound == args {
                    Ok(())
                } else {
                    Err(format!(
                        "the template is given ({}), and the send binds ({})",
                        bound.join(", "),
                        args.join(", ")
                    ))
                }
            }
            (Action::Log, Detail::Log { label, expr }, Statement::Log(items)) => {
                log(label, *expr, items)
            }
            (
                Action::Setverdict,
                Detail::Setverdict { value, reason },
                Statement::Setverdict {
                    verdict,
                    reason: written,
                },
            ) => {
                let owed = match value.as_str() {
                    "inconclusive" => "inconc",
                    other => other,
                };
                named("the verdict", verdict, owed)?;
                if written.is_some() == *reason {
                    Ok(())
                } else {
                    Err(
                        "it gives a reason where the document gives none, or none where it does"
                            .to_string(),
                    )
                }
            }
            (
                Action::Timer,
                Detail::StartTimer { timer, duration },
                Statement::StartTimer {
                    timer: written,
                    duration: started,
                },
            ) => {
                named("the timer it starts", written, timer)?;
                match (duration, started) {
                    (None, None) => Ok(()),
                    (Some(duration), Some(started)) => same_duration(duration, started),
                    (None, Some(started)) => Err(format!(
                        "it starts the timer for {started}, and the document starts it for its \
                         default"
                    )),
                    (Some(duration), None) => Err(format!(
                        "it starts the timer for its default, and the document writes \
                         '{duration}'"
                    )),
                }
            }
            (
                Action::Timer,
                Detail::StopTimer { timer },
                Statement::StopTimer { timer: written },
            ) => named("the timer it stops", written, timer),
            _ => wrong(found, &format!("the statement of {}", self.construct.node)),
        }
    }

    /// The local a receive branch stores its message in (E8): `v_<record>`,
    /// of the record's type.
    fn received(&self, found: Found<'_>) -> Held {
        let record = self
            .construct
            .node
            .strip_prefix("received ")
            .unwrap_or_default();
        match found {
            Found::Local(Local {
                ty,
                name,
                starts_null: false,
            }) if *ty == TypeExpr::Named(record.to_string()) => {
                named("the local", name, &format!("v_{record}"))
            }
            _ => wrong(found, &format!("`var {record} v_{record};`")),
        }
    }

    /// A test case (E14): on its MTC's component type, in its
    /// configuration's system, mapping the MTC's ports and calling the MTC's
    /// function.
    fn test_case(&self, found: Found<'_>) -> Held {
        let Found::Definition(Definition::Testcase {
            name,
            runs_on,
            system,
            body,
        }) = found
        else {
            return wrong(found, "a test case");
        };
        named("the test case", name, &unit(self.document))?;
        let configuration = self
            .documents
            .configuration_of(self.document)
            .ok_or("the test case runs in no configuration the suite reaches")?;
        let Form::Configuration { mtc, maps } = &configuration.form else {
            return Ok(());
        };
        let component = self
            .documents
            .imported(configuration, mtc, DocumentKind::TestComponent)
            .ok_or_else(|| format!("'{mtc}' names no component {} imports", configuration.name))?;
        named("the component type it runs on", runs_on, &unit(component))?;
        named("its system", system, &unit(configuration))?;
        let function = if component.invokes() {
            format!("f_{}_in_{}", unit(component), unit(configuration))
        } else {
            format!("f_{}", unit(component))
        };
        let mut owed: Vec<Statement> = maps
            .iter()
            .filter(|map| map.component == "mtc")
            .map(|map| {
                Statement::Connect(Connection {
                    operation: "map".to_string(),
                    left: "mtc".to_string(),
                    left_port: map.port.clone(),
                    right: "system".to_string(),
                    right_port: map.system_port.clone(),
                    when: None,
                })
            })
            .collect();
        owed.push(Statement::Call(function));
        if *body == owed {
            Ok(())
        } else {
            Err(
                "its body is not the MTC's maps, in document order, and the call of its \
                 function"
                    .to_string(),
            )
        }
    }

    /// A module parameter of the suite, or of a feature (E15).
    fn module_parameter(&self, found: Found<'_>) -> Held {
        let Found::Definition(Definition::Modulepar { name, ty, value }) = found else {
            return wrong(found, "a module parameter");
        };
        match self.detail() {
            Detail::ModuleParameter { id, ty: declared } => {
                named("the module parameter", name, id)?;
                let declared = self.written(self.document, declared)?;
                if declared.optional || *ty != declared.ty || value.is_none() {
                    return Err(format!("it is not `modulepar {:?} {id} := …`", declared.ty));
                }
                Ok(())
            }
            Detail::Feature { name: feature } => {
                let owed = format!("PICS_{}", feature.to_ascii_uppercase().replace('-', "_"));
                named("the module parameter", name, &owed)?;
                if *ty != TypeExpr::Boolean || value.is_some() {
                    return Err(format!("it is not `modulepar boolean {owed};`"));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// An execute of the control part, with its guard, under its selection
    /// where the suite writes one (E15).
    fn execute(&self, found: Found<'_>) -> Held {
        let Detail::Execute {
            test_case,
            guard,
            requires,
        } = self.detail()
        else {
            return Ok(());
        };
        let (written, guarded, selected) = match found {
            Found::Statement(Statement::Execute { test_case, guard }) => (test_case, guard, false),
            Found::Statement(Statement::SelectedExecute {
                test_case, guard, ..
            }) => (test_case, guard, true),
            _ => return wrong(found, "an execute"),
        };
        named(
            "the test case it executes",
            written,
            &self.imported(self.document, test_case, DocumentKind::TestCase)?,
        )?;
        if selected != *requires {
            return Err(if *requires {
                "it stands under no selection, and the suite selects the test case".to_string()
            } else {
                "it stands under a selection, and the suite selects nothing".to_string()
            });
        }
        match (guard, guarded) {
            (None, None) => Ok(()),
            (Some(guard), Some(written)) => same_duration(guard, written),
            (None, Some(written)) => {
                Err(format!("its guard is {written}, and the suite gives none"))
            }
            (Some(guard), None) => Err(format!("it has no guard, and the suite gives '{guard}'")),
        }
    }
}

/// One of the integer types of the types module (E1 of the suite).
fn integer_type(found: Found<'_>) -> Held {
    match found {
        Found::Definition(Definition::Type {
            name,
            body: TypeBody::Integer { min, max },
        }) => {
            let held = INTEGERS.iter().any(|(integer, low, high)| {
                name.as_str() == *integer && min.as_str() == *low && max.as_str() == *high
            });
            if held {
                Ok(())
            } else {
                Err(format!(
                    "`type integer {name} ({min} .. {max})` is no integer type of the type \
                     grammar with its width's range"
                ))
            }
        }
        _ => wrong(found, "an integer type"),
    }
}

/// A codec function of the record `record` (E1): `enc_R(in R msg) return
/// octetstring` or `dec_R(inout octetstring octets, out R msg) return
/// integer`.
fn codec(name: &str, parameters: &[Parameter], returns: &TypeExpr, record: &str) -> Held {
    let parameter = |direction: &str, ty: TypeExpr, name: &str| Parameter {
        direction: Some(direction.to_string()),
        omit: false,
        ty,
        name: name.to_string(),
    };
    let message = TypeExpr::Named(record.to_string());
    let (owed, result) = if name == format!("enc_{record}") {
        (vec![parameter("in", message, "msg")], TypeExpr::Octetstring)
    } else if name == format!("dec_{record}") {
        (
            vec![
                parameter("inout", TypeExpr::Octetstring, "octets"),
                parameter("out", message, "msg"),
            ],
            TypeExpr::Named("integer".to_string()),
        )
    } else {
        return Err(format!(
            "the codec functions of {record} are enc_{record} and dec_{record}, and it is {name}"
        ));
    };
    if parameters == owed.as_slice() && *returns == result {
        Ok(())
    } else {
        Err(format!("it is not the signature §9.5 gives {name}"))
    }
}

/// A log of `label`, and of the value where the document writes one (E13):
/// `log("label: ", v)`, `log(v)` with no label, `log("label")` with no
/// value.
fn log(label: &str, expr: bool, items: &[String]) -> Held {
    let (owed, text) = match (expr, label.is_empty()) {
        (true, false) => (2, Some(format!("{label}: "))),
        (true, true) => (1, None),
        (false, _) => (1, Some(label.to_string())),
    };
    if items.len() != owed {
        return Err(format!(
            "it logs {} items, and the document logs {owed}",
            items.len()
        ));
    }
    match text {
        Some(text) if charstring(&items[0]).as_deref() != Some(text.as_str()) => Err(format!(
            "it logs {}, and the document's label is '{label}'",
            items[0]
        )),
        _ => Ok(()),
    }
}

fn fields_text(fields: &[Field]) -> String {
    let fields: Vec<String> = fields
        .iter()
        .map(|field| {
            format!(
                "{} {:?}{}",
                field.name,
                field.ty,
                if field.optional { " optional" } else { "" }
            )
        })
        .collect();
    format!("[{}]", fields.join(", "))
}

fn variants_text(variants: &[(String, String)]) -> String {
    let variants: Vec<String> = variants
        .iter()
        .map(|(name, number)| format!("{name}({number})"))
        .collect();
    format!("[{}]", variants.join(", "))
}

fn parameters_text(parameters: &[Parameter]) -> String {
    let parameters: Vec<String> = parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}{:?} {}",
                if parameter.omit {
                    "template (omit) "
                } else {
                    ""
                },
                parameter.ty,
                parameter.name
            )
        })
        .collect();
    format!("({})", parameters.join(", "))
}

fn messages_text(messages: &[Message]) -> String {
    let messages: Vec<String> = messages
        .iter()
        .map(|message| format!("{} {:?}", message.direction, message.ty))
        .collect();
    format!("[{}]", messages.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{
        documents, module, position, witness_of, witness_value, BEHAVIOUR, TYPES,
    };
    use serde_json::{json, Value};

    fn checked(types: &str, behaviour: &str, witness: &Value) -> Vec<Violation> {
        let documents = documents();
        let types = module(types);
        let behaviour = module(behaviour);
        check(&documents, &[&types, &behaviour], &witness_of(witness))
    }

    fn codes(violations: &[Violation]) -> Vec<&'static str> {
        violations.iter().map(Violation::code).collect()
    }

    #[test]
    fn an_export_in_the_forms_of_its_documents_has_no_violation() {
        assert_eq!(
            checked(TYPES, BEHAVIOUR, &witness_value()),
            Vec::<Violation>::new()
        );
    }

    /// A template named otherwise than its document's unit name, its witness
    /// naming the place where it stands.
    #[test]
    fn a_template_named_otherwise_is_found() {
        let renamed = BEHAVIOUR.replace("template m ask(", "template m asks(");
        assert_ne!(renamed, BEHAVIOUR);
        let mut witness = witness_value();
        let at = witness["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .position(|entry| entry["construct"]["document"] == "ask.scxml")
            .expect("the template's entry");
        witness["entries"][at]["produced"][0]["path"] = json!([{"template": "asks"}]);
        let found = checked(TYPES, &renamed, &witness);
        assert_eq!(codes(&found), ["form"], "{found:?}");
        assert!(found[0].to_string().contains("'asks'"), "{}", found[0]);
    }

    /// A field, a formal parameter or a timer's duration other than the
    /// document's is found; a duration written with more zeros is the same
    /// number of seconds.
    #[test]
    fn a_type_or_a_duration_other_than_the_documents_is_found() {
        let field = TYPES.replace("uint8 n", "uint16 n");
        assert_eq!(
            codes(&checked(&field, BEHAVIOUR, &witness_value())),
            ["form"]
        );
        let parameter = BEHAVIOUR.replace("(uint8 limit)", "(uint16 limit)");
        assert_eq!(
            codes(&checked(TYPES, &parameter, &witness_value())),
            ["form"]
        );
        let duration = BEHAVIOUR.replace("timer t := 1.0;", "timer t := 1.000001;");
        assert_eq!(
            codes(&checked(TYPES, &duration, &witness_value())),
            ["form"]
        );
        let same = BEHAVIOUR.replace("timer u := 2.0;", "timer u := 2.000;");
        assert_eq!(
            checked(TYPES, &same, &witness_value()),
            Vec::<Violation>::new()
        );
    }

    /// An alternative that waits for another timer, or has a guard the
    /// transition does not, and a log of another label, are found.
    #[test]
    fn a_statement_of_another_form_is_found() {
        for changed in [
            BEHAVIOUR.replacen("[] t.timeout {", "[] u.timeout {", 1),
            BEHAVIOUR.replacen("[] t.timeout {", "[x > 0] t.timeout {", 1),
            BEHAVIOUR.replacen("log(\"bye\");", "log(\"by\");", 1),
        ] {
            assert_ne!(changed, BEHAVIOUR);
            let found = checked(TYPES, &changed, &witness_value());
            assert_eq!(codes(&found), ["form"], "{found:?}");
        }
    }

    /// An entry of a rule that translates no construct of its construct's
    /// kind is found, and its places are held to nothing else.
    #[test]
    fn an_entry_of_a_rule_that_does_not_translate_its_construct_is_found() {
        let mut witness = witness_value();
        let at = position(&witness, "state 'b'", "E6");
        witness["entries"][at]["rule"] = json!("E9");
        let found = checked(TYPES, BEHAVIOUR, &witness);
        assert_eq!(codes(&found), ["rule-kind"], "{found:?}");
        assert!(
            found[0].to_string().contains("atomic state"),
            "{}",
            found[0]
        );
    }

    /// A duration is read as a whole number of microseconds, exactly, and a
    /// log's character string with its quadruples.
    #[test]
    fn a_duration_and_a_character_string_are_read_exactly() {
        assert_eq!(delay("500ms"), Some(500_000));
        assert_eq!(delay("0.1ms"), Some(100));
        assert_eq!(delay("2s"), Some(2_000_000));
        assert_eq!(delay("2"), None);
        assert_eq!(microseconds("0.000001", 1_000_000), Some(1));
        assert_eq!(microseconds("0.0000010", 1_000_000), Some(1));
        assert_eq!(microseconds("0.0000001", 1_000_000), None);
        assert_eq!(microseconds(".", 1_000_000), None);
        assert_eq!(
            charstring("\"x\" & char(0, 0, 0, 65)"),
            Some("xA".to_string())
        );
        assert_eq!(
            charstring("\"say \"\"hi\"\"\""),
            Some("say \"hi\"".to_string())
        );
        assert_eq!(charstring("v_job.n"), None);
    }
}
