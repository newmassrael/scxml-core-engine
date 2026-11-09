// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A recursive descent over the tokens of a module, for exactly the
//! constructs the export writes (docs/SCE_TEST_PROFILE.md §9.5; the templates
//! under `tools/codegen/templates/ttcn3/`): a module, its imports, the types
//! of the types module and its external functions, port types, templates,
//! module parameters, component types, functions with their locals, labels
//! and statements, test cases and the control part. What is outside them is a
//! [`ParseError`] that names the row and what was found.

use super::ast::{
    Alternative, Call, Connection, Definition, Event, Field, Local, Member, Message, Module,
    Parameter, Statement, TypeBody, TypeExpr,
};
use super::lexer::{spelled, Lexed, Token};
use super::ParseError;

type Parsed<T> = Result<T, ParseError>;

/// The tokens of one module and how far they are read.
pub struct Parser {
    tokens: Vec<Lexed>,
    at: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Lexed>) -> Self {
        Self { tokens, at: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.peek_ahead(0)
    }

    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.at + offset).map(|lexed| &lexed.token)
    }

    /// The row of the next token, or of the last where none is left.
    fn line(&self) -> u32 {
        self.tokens
            .get(self.at)
            .or_else(|| self.tokens.last())
            .map_or(1, |lexed| lexed.line)
    }

    fn fail<T>(&self, message: impl Into<String>) -> Parsed<T> {
        Err(ParseError::new(self.line(), message))
    }

    /// What stands next, for a message.
    fn found(&self) -> String {
        match self.peek() {
            Some(token) => format!("'{}'", token.spelling()),
            None => "the end of the module".to_string(),
        }
    }

    fn is_punct(&self, punct: &str) -> bool {
        matches!(self.peek(), Some(Token::Punct(next)) if *next == punct)
    }

    fn is_word(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(next)) if next == word)
    }

    fn eat_punct(&mut self, punct: &str) -> bool {
        let next = self.is_punct(punct);
        if next {
            self.at += 1;
        }
        next
    }

    fn eat_word(&mut self, word: &str) -> bool {
        let next = self.is_word(word);
        if next {
            self.at += 1;
        }
        next
    }

    fn punct(&mut self, punct: &str) -> Parsed<()> {
        if self.eat_punct(punct) {
            return Ok(());
        }
        self.fail(format!("'{punct}' expected, {} found", self.found()))
    }

    fn word(&mut self, word: &str) -> Parsed<()> {
        if self.eat_word(word) {
            return Ok(());
        }
        self.fail(format!("'{word}' expected, {} found", self.found()))
    }

    /// The next token, a word, without reading it.
    fn next_word(&self) -> Parsed<String> {
        match self.peek() {
            Some(Token::Word(word)) => Ok(word.clone()),
            _ => self.fail(format!("a name expected, {} found", self.found())),
        }
    }

    fn name(&mut self) -> Parsed<String> {
        let name = self.next_word()?;
        self.at += 1;
        Ok(name)
    }

    fn integer(&mut self) -> Parsed<String> {
        match self.peek() {
            Some(Token::Integer(digits)) => {
                let digits = digits.clone();
                self.at += 1;
                Ok(digits)
            }
            _ => self.fail(format!("an integer expected, {} found", self.found())),
        }
    }

    /// An integer with its sign, where it has one.
    fn signed(&mut self) -> Parsed<String> {
        let negative = self.eat_punct("-");
        let digits = self.integer()?;
        Ok(if negative {
            format!("-{digits}")
        } else {
            digits
        })
    }

    fn text(&mut self) -> Parsed<String> {
        match self.peek() {
            Some(Token::Text(text)) => {
                let text = text.clone();
                self.at += 1;
                Ok(text)
            }
            _ => self.fail(format!(
                "a character string expected, {} found",
                self.found()
            )),
        }
    }

    /// The text of the tokens read since `start`.
    fn since(&self, start: usize) -> String {
        spelled(&self.tokens[start..self.at])
    }

    // ── The module ─────────────────────────────────────────────

    /// The module the tokens are, and nothing after it.
    pub fn module(&mut self) -> Parsed<Module> {
        self.word("module")?;
        let name = self.name()?;
        self.punct("{")?;
        let mut imports = Vec::new();
        let mut definitions = Vec::new();
        while !self.eat_punct("}") {
            if self.peek().is_none() {
                return self.fail("the module is never closed");
            }
            if self.eat_word("import") {
                self.word("from")?;
                imports.push(self.name()?);
                self.word("all")?;
                self.punct(";")?;
                continue;
            }
            definitions.push(self.definition()?);
        }
        if self.peek().is_some() {
            return self.fail(format!("{} after the end of the module", self.found()));
        }
        Ok(Module {
            name,
            imports,
            definitions,
        })
    }

    fn definition(&mut self) -> Parsed<Definition> {
        let keyword = self.next_word()?;
        match keyword.as_str() {
            "type" => self.type_definition(),
            "external" => self.external_function(),
            "template" => self.template(),
            "modulepar" => self.modulepar(),
            "function" => self.function(),
            "testcase" => self.testcase(),
            "control" => self.control(),
            _ => self.fail(format!(
                "a definition expected, {} found: the export writes types, external functions, \
                 port and component types, templates, module parameters, functions, test cases \
                 and the control part",
                self.found()
            )),
        }
    }

    fn type_definition(&mut self) -> Parsed<Definition> {
        self.word("type")?;
        let form = self.name()?;
        match form.as_str() {
            "port" => {
                let name = self.name()?;
                self.word("message")?;
                self.punct("{")?;
                let mut messages = Vec::new();
                while !self.eat_punct("}") {
                    let direction = self.name()?;
                    if !matches!(direction.as_str(), "in" | "out" | "inout") {
                        return self.fail(format!(
                            "'{direction}' is no direction of a message: in, out or inout"
                        ));
                    }
                    let ty = self.type_expr()?;
                    self.punct(";")?;
                    messages.push(Message { direction, ty });
                }
                Ok(Definition::PortType { name, messages })
            }
            "component" => {
                let name = self.name()?;
                self.punct("{")?;
                let mut members = Vec::new();
                while !self.eat_punct("}") {
                    members.push(self.member()?);
                }
                Ok(Definition::ComponentType { name, members })
            }
            "integer" => {
                let name = self.name()?;
                self.punct("(")?;
                let min = self.signed()?;
                self.punct("..")?;
                let max = self.signed()?;
                self.punct(")")?;
                self.punct(";")?;
                Ok(Definition::Type {
                    name,
                    body: TypeBody::Integer { min, max },
                })
            }
            "enumerated" => {
                let name = self.name()?;
                self.punct("{")?;
                let mut variants = Vec::new();
                loop {
                    let variant = self.name()?;
                    self.punct("(")?;
                    let number = self.signed()?;
                    self.punct(")")?;
                    variants.push((variant, number));
                    if !self.eat_punct(",") {
                        break;
                    }
                }
                self.punct("}")?;
                Ok(Definition::Type {
                    name,
                    body: TypeBody::Enumerated(variants),
                })
            }
            "record" => {
                let name = self.name()?;
                let fields = self.fields()?;
                Ok(Definition::Type {
                    name,
                    body: TypeBody::Record(fields),
                })
            }
            "union" => {
                let name = self.name()?;
                let fields = self.fields()?;
                Ok(Definition::Type {
                    name,
                    body: TypeBody::Union(fields),
                })
            }
            other => self.fail(format!(
                "'type {other}' is no type the export writes: port, component, integer, \
                 enumerated, record or union"
            )),
        }
    }

    /// `{ T name [optional], … }`, or `{}`.
    fn fields(&mut self) -> Parsed<Vec<Field>> {
        self.punct("{")?;
        let mut fields = Vec::new();
        if self.eat_punct("}") {
            return Ok(fields);
        }
        loop {
            let ty = self.type_expr()?;
            let name = self.name()?;
            let optional = self.eat_word("optional");
            fields.push(Field { ty, name, optional });
            if !self.eat_punct(",") {
                break;
            }
        }
        self.punct("}")?;
        Ok(fields)
    }

    fn type_expr(&mut self) -> Parsed<TypeExpr> {
        let word = self.name()?;
        Ok(match word.as_str() {
            "universal" => {
                self.word("charstring")?;
                TypeExpr::UniversalCharstring
            }
            "boolean" => TypeExpr::Boolean,
            "octetstring" => TypeExpr::Octetstring,
            "record" if self.eat_word("length") => {
                self.punct("(")?;
                let min = self.integer()?;
                self.punct("..")?;
                let max = self.integer()?;
                self.punct(")")?;
                self.word("of")?;
                TypeExpr::RecordOf {
                    min,
                    max,
                    element: Box::new(self.type_expr()?),
                }
            }
            "record" => TypeExpr::Record(self.fields()?),
            _ => TypeExpr::Named(word.clone()),
        })
    }

    /// `external function f(in T msg) return T;`
    fn external_function(&mut self) -> Parsed<Definition> {
        self.word("external")?;
        self.word("function")?;
        let name = self.name()?;
        self.punct("(")?;
        let mut parameters = Vec::new();
        if !self.eat_punct(")") {
            loop {
                let direction = self.name()?;
                if !matches!(direction.as_str(), "in" | "out" | "inout") {
                    return self.fail(format!(
                        "'{direction}' is no direction of a parameter: in, out or inout"
                    ));
                }
                let ty = self.type_expr()?;
                let name = self.name()?;
                parameters.push(Parameter {
                    direction: Some(direction),
                    omit: false,
                    ty,
                    name,
                });
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct(")")?;
        }
        self.word("return")?;
        let returns = self.type_expr()?;
        self.punct(";")?;
        Ok(Definition::ExternalFunction {
            name,
            parameters,
            returns,
        })
    }

    /// `template T name[(params)] := matcher;`
    fn template(&mut self) -> Parsed<Definition> {
        self.word("template")?;
        let ty = self.type_expr()?;
        let name = self.name()?;
        let mut parameters = Vec::new();
        if self.eat_punct("(") {
            loop {
                let omit = self.eat_word("template");
                if omit {
                    self.punct("(")?;
                    self.word("omit")?;
                    self.punct(")")?;
                }
                let ty = self.type_expr()?;
                let name = self.name()?;
                parameters.push(Parameter {
                    direction: None,
                    omit,
                    ty,
                    name,
                });
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct(")")?;
        }
        self.punct(":=")?;
        let start = self.at;
        self.matcher()?;
        let matcher = self.since(start);
        self.punct(";")?;
        Ok(Definition::Template {
            name,
            ty,
            parameters,
            matcher,
        })
    }

    /// A matcher of §6.1 as -1 Annex B.1 writes it, with its length and
    /// `ifpresent` where it has them.
    fn matcher(&mut self) -> Parsed<()> {
        self.matcher_body()?;
        if self.eat_word("length") {
            self.punct("(")?;
            self.bound()?;
            self.punct("..")?;
            self.bound()?;
            self.punct(")")?;
        }
        self.eat_word("ifpresent");
        Ok(())
    }

    fn matcher_body(&mut self) -> Parsed<()> {
        if self.eat_punct("?") || self.eat_punct("*") {
            return Ok(());
        }
        if self.eat_punct("(") {
            let start = self.at;
            if self.bound().is_ok() && self.eat_punct("..") {
                self.bound()?;
                return self.punct(")");
            }
            self.at = start;
            self.matchers_until(")")?;
            return Ok(());
        }
        if self.eat_punct("{") {
            let record = matches!(self.peek(), Some(Token::Word(_)))
                && matches!(self.peek_ahead(1), Some(Token::Punct(":=")));
            if record {
                loop {
                    self.name()?;
                    self.punct(":=")?;
                    self.matcher()?;
                    if !self.eat_punct(",") {
                        break;
                    }
                }
                return self.punct("}");
            }
            if self.eat_punct("}") {
                return Ok(());
            }
            return self.matchers_until("}");
        }
        match self.peek() {
            Some(Token::Integer(_) | Token::Octets(_)) => {
                self.at += 1;
                Ok(())
            }
            Some(Token::Punct("-")) => self.signed().map(|_| ()),
            Some(Token::Text(_)) => self.charstring(),
            Some(Token::Word(word)) => {
                let word = word.clone();
                match word.as_str() {
                    "omit" | "true" | "false" => {
                        self.at += 1;
                        Ok(())
                    }
                    "char" => self.charstring(),
                    "complement" => {
                        self.at += 1;
                        self.punct("(")?;
                        self.matchers_until(")")
                    }
                    _ => {
                        self.at += 1;
                        if self.eat_punct("(") {
                            loop {
                                self.name()?;
                                if !self.eat_punct(",") {
                                    break;
                                }
                            }
                            self.punct(")")?;
                        }
                        Ok(())
                    }
                }
            }
            _ => self.fail(format!("a matcher expected, {} found", self.found())),
        }
    }

    /// Matchers separated by `,`, then `close`.
    fn matchers_until(&mut self, close: &str) -> Parsed<()> {
        loop {
            self.matcher()?;
            if !self.eat_punct(",") {
                break;
            }
        }
        self.punct(close)
    }

    /// A bound of a range or a length: an integer, a name, or an infinity.
    fn bound(&mut self) -> Parsed<()> {
        if self.eat_punct("-") {
            if self.eat_word("infinity") {
                return Ok(());
            }
            return self.integer().map(|_| ());
        }
        match self.peek() {
            Some(Token::Integer(_) | Token::Word(_)) => {
                self.at += 1;
                Ok(())
            }
            _ => self.fail(format!("a bound expected, {} found", self.found())),
        }
    }

    /// A character string, its quadruples concatenated with `&`.
    fn charstring(&mut self) -> Parsed<()> {
        loop {
            if self.eat_word("char") {
                self.quadruple()?;
            } else {
                self.text()?;
            }
            if !self.eat_punct("&") {
                return Ok(());
            }
        }
    }

    /// `(g, p, r, c)`, the quadruple of a character (-1 §6.1.1).
    fn quadruple(&mut self) -> Parsed<()> {
        self.punct("(")?;
        for position in 0..4 {
            if position > 0 {
                self.punct(",")?;
            }
            self.integer()?;
        }
        self.punct(")")
    }

    /// `modulepar T name [:= value];`
    fn modulepar(&mut self) -> Parsed<Definition> {
        self.word("modulepar")?;
        let ty = self.type_expr()?;
        let name = self.name()?;
        let value = if self.eat_punct(":=") {
            Some(self.expression()?)
        } else {
            None
        };
        self.punct(";")?;
        Ok(Definition::Modulepar { name, ty, value })
    }

    /// `port P p;`, `timer t [:= d];`, `var T v := e;`
    fn member(&mut self) -> Parsed<Member> {
        let keyword = self.name()?;
        let member = match keyword.as_str() {
            "port" => {
                let port_type = self.name()?;
                let name = self.name()?;
                Member::Port { port_type, name }
            }
            "timer" => {
                let name = self.name()?;
                let duration = if self.eat_punct(":=") {
                    Some(self.expression()?)
                } else {
                    None
                };
                Member::Timer { name, duration }
            }
            "var" => {
                let ty = self.type_expr()?;
                let name = self.name()?;
                self.punct(":=")?;
                let value = self.expression()?;
                Member::Var { ty, name, value }
            }
            other => {
                return self.fail(format!(
                    "'{other}' is no member of a component type: port, timer or var"
                ))
            }
        };
        self.punct(";")?;
        Ok(member)
    }

    /// `function f() runs on C { locals statements }`
    fn function(&mut self) -> Parsed<Definition> {
        self.word("function")?;
        let name = self.name()?;
        self.punct("(")?;
        self.punct(")")?;
        self.word("runs")?;
        self.word("on")?;
        let runs_on = self.name()?;
        self.punct("{")?;
        let mut locals = Vec::new();
        while self.eat_word("var") {
            let ty = self.type_expr()?;
            let name = self.name()?;
            let starts_null = self.eat_punct(":=");
            if starts_null {
                self.word("null")?;
            }
            self.punct(";")?;
            locals.push(Local {
                ty,
                name,
                starts_null,
            });
        }
        let body = self.statements_until_closed()?;
        Ok(Definition::Function {
            name,
            runs_on,
            locals,
            body,
        })
    }

    /// `testcase t() runs on C system S { statements }`
    fn testcase(&mut self) -> Parsed<Definition> {
        self.word("testcase")?;
        let name = self.name()?;
        self.punct("(")?;
        self.punct(")")?;
        self.word("runs")?;
        self.word("on")?;
        let runs_on = self.name()?;
        self.word("system")?;
        let system = self.name()?;
        let body = self.block()?;
        Ok(Definition::Testcase {
            name,
            runs_on,
            system,
            body,
        })
    }

    /// `control { statements }`
    fn control(&mut self) -> Parsed<Definition> {
        self.word("control")?;
        let body = self.block()?;
        Ok(Definition::Control { body })
    }

    // ── Statements ─────────────────────────────────────────────

    fn block(&mut self) -> Parsed<Vec<Statement>> {
        self.punct("{")?;
        self.statements_until_closed()
    }

    /// The statements up to and with the `}` that closes them.
    fn statements_until_closed(&mut self) -> Parsed<Vec<Statement>> {
        let mut statements = Vec::new();
        while !self.eat_punct("}") {
            if self.peek().is_none() {
                return self.fail("a block is never closed");
            }
            statements.push(self.statement()?);
        }
        Ok(statements)
    }

    fn statement(&mut self) -> Parsed<Statement> {
        let keyword = self.next_word()?;
        match keyword.as_str() {
            "label" | "goto" => {
                self.at += 1;
                let label = self.name()?;
                self.punct(";")?;
                Ok(if keyword == "label" {
                    Statement::Label(label)
                } else {
                    Statement::Goto(label)
                })
            }
            "return" => {
                self.at += 1;
                self.punct(";")?;
                Ok(Statement::Return)
            }
            "setverdict" => {
                self.at += 1;
                self.punct("(")?;
                let verdict = self.name()?;
                if !matches!(verdict.as_str(), "pass" | "inconc" | "fail" | "none") {
                    return self.fail(format!("'{verdict}' is no verdict a document sets"));
                }
                let reason = if self.eat_punct(",") {
                    Some(self.expression()?)
                } else {
                    None
                };
                self.punct(")")?;
                self.punct(";")?;
                Ok(Statement::Setverdict { verdict, reason })
            }
            "log" => {
                self.at += 1;
                self.punct("(")?;
                let mut items = Vec::new();
                loop {
                    items.push(self.expression()?);
                    if !self.eat_punct(",") {
                        break;
                    }
                }
                self.punct(")")?;
                self.punct(";")?;
                Ok(Statement::Log(items))
            }
            "if" => self.conditional(),
            "alt" => self.alt(),
            "connect" | "map" => {
                self.at += 1;
                let connection = self.connection(&keyword)?;
                self.punct(";")?;
                Ok(Statement::Connect(connection))
            }
            "execute" => {
                self.at += 1;
                self.punct("(")?;
                let test_case = self.name()?;
                self.punct("(")?;
                self.punct(")")?;
                let guard = if self.eat_punct(",") {
                    Some(self.expression()?)
                } else {
                    None
                };
                self.punct(")")?;
                self.punct(";")?;
                Ok(Statement::Execute { test_case, guard })
            }
            _ => self.operation(),
        }
    }

    /// A statement that begins with a name: an assignment, a call, or an
    /// operation on a port, a timer or a component.
    fn operation(&mut self) -> Parsed<Statement> {
        let subject = self.name()?;
        if self.eat_punct(":=") {
            let creates = matches!(self.peek_ahead(1), Some(Token::Punct(".")))
                && matches!(self.peek_ahead(2), Some(Token::Word(word)) if word == "create");
            if creates {
                let component_type = self.name()?;
                self.punct(".")?;
                self.word("create")?;
                self.punct("(")?;
                let ptc = self.text()?;
                self.punct(")")?;
                self.word("alive")?;
                self.punct(";")?;
                return Ok(Statement::CreateAssign {
                    reference: subject,
                    component_type,
                    ptc,
                });
            }
            let value = self.expression()?;
            self.punct(";")?;
            return Ok(Statement::Assign {
                location: subject,
                value,
            });
        }
        if self.eat_punct("(") {
            self.punct(")")?;
            self.punct(";")?;
            return Ok(Statement::Call(subject));
        }
        if !self.eat_punct(".") {
            return self.fail(format!(
                "a statement expected after '{subject}', {} found",
                self.found()
            ));
        }
        let operation = self.name()?;
        let statement = match operation.as_str() {
            "send" => {
                self.punct("(")?;
                let template = self.call()?;
                self.punct(")")?;
                Statement::Send {
                    port: subject,
                    template,
                }
            }
            "start" if self.eat_punct("(") => {
                let starts_function = matches!(self.peek(), Some(Token::Word(_)))
                    && matches!(self.peek_ahead(1), Some(Token::Punct("(")));
                let statement = if starts_function {
                    let function = self.name()?;
                    self.punct("(")?;
                    self.punct(")")?;
                    Statement::StartComponent {
                        reference: subject,
                        function,
                    }
                } else {
                    Statement::StartTimer {
                        timer: subject,
                        duration: Some(self.expression()?),
                    }
                };
                self.punct(")")?;
                statement
            }
            "start" => Statement::StartTimer {
                timer: subject,
                duration: None,
            },
            "stop" => Statement::StopTimer { timer: subject },
            "kill" => Statement::KillComponent { reference: subject },
            other => {
                return self.fail(format!(
                    "'{subject}.{other}' is no operation the export writes: send, start, stop or \
                     kill"
                ))
            }
        };
        self.punct(";")?;
        Ok(statement)
    }

    /// A template as a send or a receive names it: `t` or `t(a, b)`.
    fn call(&mut self) -> Parsed<Call> {
        let name = self.name()?;
        let mut args = Vec::new();
        if self.eat_punct("(") {
            loop {
                args.push(self.name()?);
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.punct(")")?;
        }
        Ok(Call { name, args })
    }

    /// `(a:p, b:q)` of a `connect` or a `map`.
    fn connection(&mut self, operation: &str) -> Parsed<Connection> {
        self.punct("(")?;
        let left = self.name()?;
        self.punct(":")?;
        let left_port = self.name()?;
        self.punct(",")?;
        let right = self.name()?;
        self.punct(":")?;
        let right_port = self.name()?;
        self.punct(")")?;
        Ok(Connection {
            operation: operation.to_string(),
            left,
            left_port,
            right,
            right_port,
            when: None,
        })
    }

    /// `if (c) { … } [else { … }]`, read as the one statement it is.
    fn conditional(&mut self) -> Parsed<Statement> {
        self.word("if")?;
        self.punct("(")?;
        let condition = self.expression()?;
        self.punct(")")?;
        let then = self.block()?;
        let otherwise = if self.eat_word("else") {
            self.block()?
        } else {
            Vec::new()
        };
        Ok(classified(condition, then, otherwise))
    }

    /// `alt { [guard] event { … } … }`
    fn alt(&mut self) -> Parsed<Statement> {
        self.word("alt")?;
        self.punct("{")?;
        let mut alternatives = Vec::new();
        while !self.eat_punct("}") {
            self.punct("[")?;
            let guard = if self.eat_punct("]") {
                None
            } else {
                let guard = self.expression()?;
                self.punct("]")?;
                Some(guard)
            };
            let event = self.event()?;
            let block = self.block()?;
            alternatives.push(Alternative {
                guard,
                event,
                block,
            });
        }
        Ok(Statement::Alt(alternatives))
    }

    /// `p.receive[(t)] [-> value v]`, `T.timeout` or `c.done`.
    fn event(&mut self) -> Parsed<Event> {
        let subject = self.name()?;
        self.punct(".")?;
        let operation = self.name()?;
        match operation.as_str() {
            "receive" => {
                let template = if self.eat_punct("(") {
                    let template = self.call()?;
                    self.punct(")")?;
                    Some(template)
                } else {
                    None
                };
                let redirect = if self.eat_punct("->") {
                    self.word("value")?;
                    Some(self.name()?)
                } else {
                    None
                };
                Ok(Event::Receive {
                    port: subject,
                    template,
                    redirect,
                })
            }
            "timeout" => Ok(Event::Timeout { timer: subject }),
            "done" => Ok(Event::Done { reference: subject }),
            other => self.fail(format!(
                "'{subject}.{other}' is no event an alternative waits for: receive, timeout or \
                 done"
            )),
        }
    }

    // ── Expressions ────────────────────────────────────────────

    /// An expression, as its text: operands joined by the operators the
    /// export writes, each operand that is an operation in parentheses.
    fn expression(&mut self) -> Parsed<String> {
        let start = self.at;
        self.operand()?;
        while self.binary_operator() {
            self.operand()?;
        }
        Ok(self.since(start))
    }

    fn binary_operator(&mut self) -> bool {
        let operator = match self.peek() {
            Some(Token::Punct(punct)) => {
                matches!(*punct, "==" | "!=" | "<" | ">" | "<=" | ">=" | "&")
            }
            Some(Token::Word(word)) => matches!(word.as_str(), "and" | "or" | "xor"),
            _ => false,
        };
        if operator {
            self.at += 1;
        }
        operator
    }

    fn operand(&mut self) -> Parsed<()> {
        if self.eat_word("not") {
            return self.operand();
        }
        if self.eat_punct("-") {
            return match self.peek() {
                Some(Token::Integer(_) | Token::Float(_)) => {
                    self.at += 1;
                    Ok(())
                }
                _ => self.fail(format!(
                    "a number expected after '-', {} found",
                    self.found()
                )),
            };
        }
        if self.eat_punct("(") {
            self.expression()?;
            return self.punct(")");
        }
        match self.peek() {
            Some(Token::Integer(_) | Token::Float(_) | Token::Octets(_)) => {
                self.at += 1;
                Ok(())
            }
            Some(Token::Text(_)) => self.charstring(),
            Some(Token::Word(word))
                if word == "char" && matches!(self.peek_ahead(1), Some(Token::Punct("("))) =>
            {
                self.charstring()
            }
            Some(Token::Word(_)) => {
                self.at += 1;
                while self.eat_punct(".") {
                    self.name()?;
                }
                Ok(())
            }
            _ => self.fail(format!("an operand expected, {} found", self.found())),
        }
    }
}

/// The statement an `if` of `condition`, `then` and `otherwise` is: the
/// start of a PTC, its kill, an execute the suite selects, or an `if`.
fn classified(condition: String, then: Vec<Statement>, otherwise: Vec<Statement>) -> Statement {
    if otherwise.is_empty() {
        if let Some(kill) = kill(&condition, &then) {
            return kill;
        }
        if let Some(create) = create(&condition, &then) {
            return create;
        }
        if let [Statement::Execute { test_case, guard }] = then.as_slice() {
            return Statement::SelectedExecute {
                condition,
                test_case: test_case.clone(),
                guard: guard.clone(),
            };
        }
    }
    Statement::If {
        condition,
        then,
        otherwise,
    }
}

/// `if (v != null) { v.kill; v := null; }`
fn kill(condition: &str, then: &[Statement]) -> Option<Statement> {
    let [Statement::KillComponent { reference }, Statement::Assign { location, value }] = then
    else {
        return None;
    };
    let killed = location == reference
        && value == "null"
        && condition.strip_suffix(" != null") == Some(reference.as_str());
    killed.then(|| Statement::Kill {
        reference: reference.clone(),
    })
}

/// `if (v == null) { v := T.create("ptc") alive; connections; v.start(f()); }`,
/// where a connection to another PTC stands in `if (w != null) { … }`.
fn create(condition: &str, then: &[Statement]) -> Option<Statement> {
    let (first, rest) = then.split_first()?;
    let Statement::CreateAssign {
        reference,
        component_type,
        ptc,
    } = first
    else {
        return None;
    };
    let (last, middle) = rest.split_last()?;
    let Statement::StartComponent {
        reference: started,
        function,
    } = last
    else {
        return None;
    };
    if started != reference || condition.strip_suffix(" == null") != Some(reference.as_str()) {
        return None;
    }
    let mut connections = Vec::new();
    for statement in middle {
        match statement {
            Statement::Connect(connection) => connections.push(connection.clone()),
            Statement::If {
                condition,
                then,
                otherwise,
            } if otherwise.is_empty() => {
                let [Statement::Connect(connection)] = then.as_slice() else {
                    return None;
                };
                let other = condition.strip_suffix(" != null")?;
                let mut connection = connection.clone();
                connection.when = Some(other.to_string());
                connections.push(connection);
            }
            _ => return None,
        }
    }
    Some(Statement::Create {
        reference: reference.clone(),
        component_type: component_type.clone(),
        ptc: ptc.clone(),
        connections,
        function: function.clone(),
    })
}
