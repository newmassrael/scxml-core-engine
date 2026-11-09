// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tree a module of the export is read into: each definition and each
//! statement the export writes, with what names it. An expression and a
//! template's matcher are kept as their text, spelled as the export spells
//! one ([`super::lexer::spelled`]).

/// A module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    /// The modules it imports whole.
    pub imports: Vec<String>,
    pub definitions: Vec<Definition>,
}

/// A definition of a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Definition {
    /// `type integer`, `type enumerated`, `type record`, `type union`.
    Type { name: String, body: TypeBody },
    /// A codec function, declared and not implemented.
    ExternalFunction {
        name: String,
        parameters: Vec<Parameter>,
        returns: TypeExpr,
    },
    /// `type port … message`.
    PortType {
        name: String,
        messages: Vec<Message>,
    },
    Template {
        name: String,
        ty: TypeExpr,
        parameters: Vec<Parameter>,
        matcher: String,
    },
    Modulepar {
        name: String,
        ty: TypeExpr,
        value: Option<String>,
    },
    /// `type component`.
    ComponentType { name: String, members: Vec<Member> },
    Function {
        name: String,
        runs_on: String,
        locals: Vec<Local>,
        body: Vec<Statement>,
    },
    Testcase {
        name: String,
        runs_on: String,
        system: String,
        body: Vec<Statement>,
    },
    /// The control part.
    Control { body: Vec<Statement> },
}

/// What a `type` definition defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeBody {
    Integer {
        min: String,
        max: String,
    },
    /// Each variant and its number.
    Enumerated(Vec<(String, String)>),
    Record(Vec<Field>),
    Union(Vec<Field>),
}

/// A field of a record, or an alternative of a union.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub ty: TypeExpr,
    pub name: String,
    pub optional: bool,
}

/// A type as a definition names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    /// A type by its name: a definition of a module, or `integer`.
    Named(String),
    Boolean,
    UniversalCharstring,
    Octetstring,
    RecordOf {
        min: String,
        max: String,
        element: Box<TypeExpr>,
    },
    /// A record written in place.
    Record(Vec<Field>),
}

/// A formal parameter: of an external function with its direction, or of a
/// template, `template (omit)` where it may be absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub direction: Option<String>,
    pub omit: bool,
    pub ty: TypeExpr,
    pub name: String,
}

/// A message of a port type, with its direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub direction: String,
    pub ty: TypeExpr,
}

/// A member of a component type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member {
    Port {
        port_type: String,
        name: String,
    },
    Timer {
        name: String,
        duration: Option<String>,
    },
    Var {
        ty: TypeExpr,
        name: String,
        value: String,
    },
}

impl Member {
    /// The member's name.
    pub fn name(&self) -> &str {
        match self {
            Member::Port { name, .. } | Member::Timer { name, .. } | Member::Var { name, .. } => {
                name
            }
        }
    }
}

/// A variable local to a function, `null` at its start where it is written
/// so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub ty: TypeExpr,
    pub name: String,
    pub starts_null: bool,
}

/// A template as a send or a receive names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub name: String,
    pub args: Vec<String>,
}

/// A `connect` or a `map`, and the reference its `if` asks to be created,
/// where it stands in one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub operation: String,
    pub left: String,
    pub left_port: String,
    pub right: String,
    pub right_port: String,
    pub when: Option<String>,
}

/// What an alternative waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Receive {
        port: String,
        template: Option<Call>,
        redirect: Option<String>,
    },
    Timeout {
        timer: String,
    },
    Done {
        reference: String,
    },
}

/// An alternative of an `alt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alternative {
    pub guard: Option<String>,
    pub event: Event,
    pub block: Vec<Statement>,
}

/// A statement.
///
/// The start of a PTC (`if (v == null) { v := T.create(…) alive; …
/// v.start(f()); }`), its kill (`if (v != null) { v.kill; v := null; }`) and
/// an execute the suite selects (`if (c) { execute(…); }`) are each one
/// statement, as the export writes one, and hold no place inside them. The
/// statements they are written with are read on their own first
/// ([`Statement::CreateAssign`], [`Statement::Connect`],
/// [`Statement::StartComponent`], [`Statement::KillComponent`],
/// [`Statement::Execute`]), so one that stands anywhere else is a place of
/// its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Label(String),
    Goto(String),
    Return,
    Send {
        port: String,
        template: Call,
    },
    StartTimer {
        timer: String,
        duration: Option<String>,
    },
    StopTimer {
        timer: String,
    },
    Setverdict {
        verdict: String,
        reason: Option<String>,
    },
    Log(Vec<String>),
    Assign {
        location: String,
        value: String,
    },
    If {
        condition: String,
        then: Vec<Statement>,
        otherwise: Vec<Statement>,
    },
    Alt(Vec<Alternative>),
    /// The start of a pending PTC.
    Create {
        reference: String,
        component_type: String,
        ptc: String,
        connections: Vec<Connection>,
        function: String,
    },
    /// The kill of a PTC.
    Kill {
        reference: String,
    },
    /// `v := T.create("ptc") alive;`
    CreateAssign {
        reference: String,
        component_type: String,
        ptc: String,
    },
    /// `connect(…);` or `map(…);`
    Connect(Connection),
    /// `v.start(f());`
    StartComponent {
        reference: String,
        function: String,
    },
    /// `v.kill;`
    KillComponent {
        reference: String,
    },
    /// `f();`, as a test case calls its MTC's function.
    Call(String),
    /// `execute(t()[, guard]);`
    Execute {
        test_case: String,
        guard: Option<String>,
    },
    /// An execute under the selection the suite writes for it.
    SelectedExecute {
        condition: String,
        test_case: String,
        guard: Option<String>,
    },
}
