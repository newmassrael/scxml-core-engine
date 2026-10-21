// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The statements of a generated source that follow a terminating statement
//! in their own list, as the 1-based line and its text — what a compiler's
//! unreachable-code reading reports (`go vet`'s `unreachable` pass, rustc's
//! `unreachable_code`), read the same way for every language SCE emits.
//!
//! Read from the code alone: comments, directives and string literals are
//! blanked first ([`classify`]), so a brace or a `return` inside a literal or
//! a raw Lua chunk is not structure.
//!
//! A brace language ([`unreachable_brace_statements`]) is read line by line.
//! A terminating statement is one of the shapes a generated machine writes —
//! a `return`, a `break` or a `throw` in the spelling of the language
//! ([`ends_its_list`]) — a `{ ... }` block that ends in one, and an `if` with
//! an `else` whose every branch ends in one. A `case` or `default` label opens
//! a clause the switch can reach, and a line that starts inside an open `(`
//! or `[`, or that continues an expression (`.`, `&&`, `?`), continues the
//! statement before it. A closing `}` on one line and the `else` that
//! continues the chain on the next are read as one `} else`, which is how
//! the C11 machine writes an `<elseif>`.
//!
//! Python ([`unreachable_python_statements`]) is read by its indentation: a
//! suite ends with a `raise`, `return`, `break` or `continue`, and an
//! `if`/`elif`/`else` chain whose every branch ends ends the suite it stands
//! in. A `try` whose body ends does not: its `except` is reached.

use crate::template_lexing::{classify, Class, Syntax};

/// The statements of `source`, written in `syntax`, that follow a statement
/// which ended their list.
pub(crate) fn unreachable_statements(source: &str, syntax: Syntax) -> Vec<(usize, String)> {
    match syntax {
        Syntax::Python => unreachable_python_statements(source),
        _ => unreachable_brace_statements(source, syntax),
    }
}

/// `source` with every character that is not code blanked to a space, line
/// breaks kept, so a line of it is the line of `source` with the same index.
fn code_only(source: &str, syntax: Syntax) -> String {
    source
        .chars()
        .zip(classify(source, syntax))
        .map(|(c, class)| {
            if c == '\n' || class == Class::Code {
                c
            } else {
                ' '
            }
        })
        .collect()
}

/// The bracket depth a line of code moves by: `(` and `[` open, `)` and `]`
/// close. Braces are the lists the readers track themselves.
fn brackets(code: &str) -> i32 {
    code.chars()
        .map(|c| match c {
            '(' | '[' => 1,
            ')' | ']' => -1,
            _ => 0,
        })
        .sum()
}

/// Whether `line`, the code of one statement, ends the list it stands in.
fn ends_its_list(line: &str, syntax: Syntax) -> bool {
    let word = |keyword: &str| {
        line == keyword
            || line
                .strip_prefix(keyword)
                .is_some_and(|rest| rest.starts_with([' ', ';', '@', '\'']))
    };
    match syntax {
        Syntax::Go => {
            line == "return"
                || line.starts_with("return ")
                || line.starts_with("panic(")
                || line.starts_with("goto ")
        }
        Syntax::Rust => {
            word("return")
                || word("break")
                || word("continue")
                || line.starts_with("panic!(")
                || line.starts_with("unreachable!(")
        }
        Syntax::Kotlin | Syntax::Swift => {
            word("return") || word("break") || word("continue") || word("throw")
        }
        Syntax::CFamily => {
            word("return") || word("break") || word("continue") || word("goto") || word("throw")
        }
        Syntax::Python => word("return") || word("raise") || word("break") || word("continue"),
    }
}

/// Whether `line` is a label that opens a clause control reaches from
/// elsewhere: a `case` or `default` of a switch, or a C++ access label.
fn opens_a_clause(line: &str, syntax: Syntax) -> bool {
    match syntax {
        Syntax::Go | Syntax::Swift => line.starts_with("case ") || line.starts_with("default:"),
        Syntax::CFamily => {
            line.starts_with("case ")
                || line.starts_with("default:")
                || matches!(line, "public:" | "protected:" | "private:")
        }
        Syntax::Rust | Syntax::Kotlin | Syntax::Python => false,
    }
}

/// Whether `line` continues the expression of the line before it rather than
/// starting a statement: a method chain, an operator carried to the next line.
fn continues_an_expression(line: &str) -> bool {
    line.starts_with('.')
        || line.starts_with("&&")
        || line.starts_with("||")
        || line.starts_with('?')
        || line.starts_with('+')
}

/// The reading of a brace language: see the module documentation.
pub(crate) fn unreachable_brace_statements(source: &str, syntax: Syntax) -> Vec<(usize, String)> {
    enum Opened {
        Block,
        If,
        Other,
    }
    struct Frame {
        opened: Opened,
        // The bracket depth this list's statements start at.
        depth: i32,
        // A statement of this list has ended it.
        terminated: bool,
        // An `if` chain: every branch closed so far ended the list.
        branches_end: bool,
        // An `if` chain: the branch now open is the `else`.
        in_else: bool,
    }
    impl Frame {
        fn new(opened: Opened, depth: i32) -> Frame {
            Frame {
                opened,
                depth,
                terminated: false,
                branches_end: true,
                in_else: false,
            }
        }
    }

    let code = code_only(source, syntax);
    let text: Vec<&str> = source.lines().collect();
    // A preprocessor line (C, C++) or an attribute (Rust) is not a statement.
    let skips_hash_lines = matches!(syntax, Syntax::CFamily | Syntax::Rust);

    // The lines of code, each with the index of its line in `source`; a line
    // holding only closing braces takes the `else` on the line after it.
    let mut lines: Vec<(usize, String)> = Vec::new();
    for (index, line) in code.lines().enumerate() {
        let mut line = line.trim();
        // A C or C++ line splice joins the next physical line to this one
        // and is no code of its own: the `<foreach>` body template leaves
        // lines that hold nothing else.
        if syntax == Syntax::CFamily {
            line = line.trim_end_matches('\\').trim_end();
        }
        if line.is_empty() || (skips_hash_lines && line.starts_with('#')) {
            continue;
        }
        if line.starts_with("else") {
            if let Some((_, previous)) = lines.last_mut() {
                if previous.chars().all(|c| c == '}' || c.is_whitespace()) {
                    previous.push(' ');
                    previous.push_str(line);
                    continue;
                }
            }
        }
        lines.push((index, line.to_string()));
    }

    let mut found = Vec::new();
    let mut depth = 0i32;
    let mut stack = vec![Frame::new(Opened::Other, 0)];
    for (index, line) in &lines {
        let line = line.as_str();
        if line.starts_with('}') {
            let mut rest = line;
            while let Some(after) = rest.strip_prefix('}') {
                rest = after.trim_start();
                if stack.len() == 1 {
                    continue;
                }
                let closed = stack.pop().expect("a list inside the file");
                if rest.starts_with("else") && rest.ends_with('{') {
                    let mut branch = Frame::new(Opened::If, closed.depth);
                    branch.branches_end = closed.branches_end && closed.terminated;
                    branch.in_else = rest == "else {";
                    stack.push(branch);
                    rest = "";
                    break;
                }
                let ends = match closed.opened {
                    Opened::Block => closed.terminated,
                    Opened::If => closed.in_else && closed.branches_end && closed.terminated,
                    Opened::Other => false,
                };
                if ends && (rest.is_empty() || rest.starts_with('}') || rest == ";") {
                    if let Some(parent) = stack.last_mut() {
                        parent.terminated = true;
                    }
                }
            }
            // `}()`, `})`, `}, {`: the rest belongs to the statement that
            // opened the list just closed.
            depth += brackets(rest);
            if rest.ends_with('{') {
                stack.push(Frame::new(Opened::Other, depth));
            }
            continue;
        }
        let starts_statement = stack.last().is_some_and(|frame| frame.depth == depth)
            && !continues_an_expression(line);
        depth += brackets(line);
        if starts_statement {
            if opens_a_clause(line, syntax) {
                if let Some(frame) = stack.last_mut() {
                    frame.terminated = false;
                }
            } else if let Some(frame) = stack.last_mut() {
                if frame.terminated {
                    found.push((index + 1, text[*index].to_string()));
                    frame.terminated = false;
                }
                if ends_its_list(line, syntax) {
                    frame.terminated = true;
                }
            }
        }
        if line.ends_with('{') {
            let opened = if !starts_statement || opens_a_clause(line, syntax) {
                Opened::Other
            } else if line == "{" {
                Opened::Block
            } else if line.starts_with("if ") || line.starts_with("if(") {
                Opened::If
            } else {
                Opened::Other
            };
            stack.push(Frame::new(opened, depth));
        }
    }
    found
}

/// The reading of Python: see the module documentation.
pub(crate) fn unreachable_python_statements(source: &str) -> Vec<(usize, String)> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Body {
        // A branch of an `if` chain.
        Branch,
        // Any other suite: a `try`, `except`, `for`, `with`, `def`, ...
        Other,
    }
    // The compound statement last opened in a suite, whose clauses continue
    // at the suite's own indentation.
    enum Compound {
        If {
            every_branch_ends: bool,
            has_else: bool,
        },
        Other,
    }
    struct Suite {
        indent: usize,
        body: Body,
        terminated: bool,
        open: Option<Compound>,
    }
    impl Suite {
        // The suite's open compound statement is over: an `if` with an
        // `else` whose every branch ended ends the suite.
        fn close_compound(&mut self) {
            if let Some(Compound::If {
                every_branch_ends: true,
                has_else: true,
            }) = self.open.take()
            {
                self.terminated = true;
            }
        }
    }

    let code = code_only(source, Syntax::Python);
    let text: Vec<&str> = source.lines().collect();

    let mut found = Vec::new();
    let mut stack = vec![Suite {
        indent: 0,
        body: Body::Other,
        terminated: false,
        open: None,
    }];
    let mut pending: Option<Body> = None;
    let mut depth = 0i32;
    let mut joined = false;
    for (index, raw) in code.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // A line inside an open bracket, or after a backslash, continues the
        // statement before it.
        let continued = depth > 0 || joined;
        depth += brackets(line);
        joined = line.ends_with('\\');
        if continued {
            continue;
        }
        let indent = raw.len() - raw.trim_start().len();

        if let Some(body) = pending.take() {
            if stack.last().is_some_and(|suite| indent > suite.indent) {
                stack.push(Suite {
                    indent,
                    body,
                    terminated: false,
                    open: None,
                });
            }
        }
        while stack.len() > 1 && stack.last().is_some_and(|suite| suite.indent > indent) {
            let mut closed = stack.pop().expect("a suite inside the file");
            closed.close_compound();
            if let Some(Suite {
                open:
                    Some(Compound::If {
                        every_branch_ends, ..
                    }),
                ..
            }) = stack.last_mut()
            {
                if closed.body == Body::Branch {
                    *every_branch_ends &= closed.terminated;
                }
            }
        }
        let Some(suite) = stack.last_mut() else {
            continue;
        };

        // A clause of the compound statement the suite has open.
        match suite.open.as_mut() {
            Some(Compound::If { has_else, .. })
                if line.starts_with("elif ") || line.starts_with("else:") =>
            {
                *has_else |= line.starts_with("else:");
                pending = Some(Body::Branch);
                continue;
            }
            Some(Compound::Other)
                if line.starts_with("except")
                    || line.starts_with("finally:")
                    || line.starts_with("else:") =>
            {
                pending = Some(Body::Other);
                continue;
            }
            _ => {}
        }

        suite.close_compound();
        if suite.terminated {
            found.push((index + 1, text[index].to_string()));
            suite.terminated = false;
        }
        if ends_its_list(line, Syntax::Python) {
            suite.terminated = true;
        }
        if line.ends_with(':') {
            if line.starts_with("if ") {
                suite.open = Some(Compound::If {
                    every_branch_ends: true,
                    has_else: false,
                });
                pending = Some(Body::Branch);
            } else {
                suite.open = Some(Compound::Other);
                pending = Some(Body::Other);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(source: &str, syntax: Syntax) -> Vec<usize> {
        unreachable_statements(source, syntax)
            .into_iter()
            .map(|(line, _)| line)
            .collect()
    }

    /// The reader finds a statement after a `return`, after a block that ends
    /// in one and after an `if`/`else` whose branches all do, and nothing in a
    /// literal, a raw string, a continued call or a later `switch` clause.
    #[test]
    fn the_reader_finds_what_go_vet_reports() {
        let go = r#"package p

func f(a bool) {
    func() {
        {
            g("} return {")
            return
        }
        after()
    }()
    if a {
        return
    } else if !a {
        return
    } else {
        return
    }
    later()
}

func h(a bool) int {
    switch {
    case a:
        return 1
    default:
        g(
            "x",
        )
    }
    if a {
        return k(
            1,
        )
    }
    s := `
return
}
`
    _ = s
    return 0
}
"#;
        assert_eq!(lines(go, Syntax::Go), vec![9, 18], "{go}");
    }

    /// Rust: a `break` out of a labelled block ends the list it stands in, and
    /// an `if`/`else` whose branches all leave is the end of its block — but
    /// the labelled block itself is left by it, so what follows the block is
    /// reached. A lifetime is not a character literal, and an attribute is
    /// not a statement.
    #[test]
    fn the_reader_finds_what_rustc_reports() {
        let rust = r#"fn f(a: bool) {
    'action_block: {
        g("break 'action_block;");
        break 'action_block;
        after();
    }
    reached();
    'action_block: {
        if a {
            break 'action_block;
        } else {
            break 'action_block;
        }
        later();
    }
    #[allow(unused)]
    let x = a
        .then(|| 1);
    if a {
        return;
    }
    kept(x);
}
"#;
        assert_eq!(lines(rust, Syntax::Rust), vec![5, 14], "{rust}");
    }

    /// Kotlin: `return@run` ends the `run` lambda's list, and a `throw` ends
    /// its own. An exit the condition of an `if` decides is not one.
    #[test]
    fn the_reader_finds_what_kotlinc_reports() {
        let kotlin = r#"fun f(a: Boolean) {
    run {
        if (run send@{
            g()
            true
        }) {
            return@run
        }
        kept()
        return@run
        after()
    }
    if (a) {
        throw IllegalStateException("}")
    } else {
        return
    }
    later()
}
"#;
        assert_eq!(lines(kotlin, Syntax::Kotlin), vec![11, 18], "{kotlin}");
    }

    /// C and C++: a `case` label opens a clause the switch reaches, a block
    /// that ends in `return` ends its list, and a `}` and the `else` on the
    /// line after it continue one chain — the C11 shape of an `<elseif>`.
    #[test]
    fn the_reader_finds_what_a_c_compiler_reads_as_unreachable() {
        let c = r#"static void f(int s, bool a, bool b) {
    switch (s) {
    case 1: {
        {
            raise("}");
            return;
        }
        break;
    }
    case 2:
        return;
    default:
        break;
    }
#line 12 "x.scxml"
    {
        if (a) {
            return;
        }
        else {
            if (b) {
                return;
            }
            else {
                return;
            }
        }
        if (failed) {
            return;
        }
    }
    after();
}
"#;
        assert_eq!(lines(c, Syntax::CFamily), vec![8, 28], "{c}");
    }

    /// Python: a `raise` ends its suite, an `if`/`elif`/`else` whose every
    /// branch ends ends the suite it stands in, and a `try` whose body ends
    /// does not — its `except` is reached, and so is what follows it.
    #[test]
    fn the_reader_finds_what_python_never_reaches() {
        let python = r#"def f(a, b):
    try:
        g(")")
        raise _ActionAbort  # the error ends the block
        after()
    except _ActionAbort:
        pass
    if a:
        raise _ActionAbort
    elif b:
        h(
            1,
        )
        return
    else:
        raise _ActionAbort
    later()
    if a:
        raise _ActionAbort
    kept()
"#;
        assert_eq!(lines(python, Syntax::Python), vec![5, 17], "{python}");
    }
}
