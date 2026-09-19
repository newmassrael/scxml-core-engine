// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A pseudocode rendering of a forge document — the surface a
//! specification author reviews instead of the XML.
//!
//! ```text
//!   algorithm crc16(data: bytes, seed: uint16) -> uint16
//!     const POLY: uint16 = 0x1021
//!     var crc: uint16 = seed
//!     foreach b in data:
//!       crc = crc ^ b
//!     return crc
//! ```
//!
//! # Why this is not the review table
//!
//! [`crate::forge::review_table`] already renders a non-statechart kind
//! for a human, and this module does not replace it. That one is a
//! *projection*: one row per requirement-bearing node, sorted so that
//! behaviour nobody required collects into a `(none)` block. A
//! projection answers "what does this document claim" and is
//! deliberately not invertible.
//!
//! This one answers a different question — *does this document say what
//! the specification says* — and to answer it the rendering must be
//! **total**: every field of the model reaches the output, so that a
//! reader who approves the pseudocode has approved the document and not
//! a summary of it.
//!
//! # ⚠ Totality is the contract, and it is why a kind is refused
//!
//! A renderer that silently omits a field produces a text that reads
//! perfectly well and is not the document. The reviewer signs it, and
//! the generated code carries something they never saw. That failure is
//! worse than having no review surface at all, because it converts an
//! unreviewed document into a signed one.
//!
//! So a kind is either rendered in full or refused by name
//! ([`Unsupported`]) — never rendered partially. The `match` in
//! [`render`] lists every kind on its own line rather than collapsing
//! the remainder behind `_`, so a kind added to
//! [`ForgeDocument`](crate::forge::model::ForgeDocument) stops the build
//! until somebody decides which of the two it is. That is the same
//! discipline [`crate::forge::requirement_nodes`] states for itself.
//!
//! # Determinism
//!
//! The output is a pure function of the model: no clock, no path, no
//! environment, and no hash-ordered container. Two runs of one build on
//! one document write the same bytes, which is what makes a round-trip
//! comparison an instrument rather than a coincidence. The same contract
//! `docs/SCE_CODEGEN_DETERMINISM.md` states for codegen applies here,
//! minus the two pinned inputs — this renderer prints neither a stamp
//! nor a provenance path, so it has nothing to pin.
//!
//! # Opaque text
//!
//! Identifiers and expressions are opaque by contract: an id may be any
//! run of non-whitespace and an expression any string the datamodel
//! language accepts, including one carrying a newline written as
//! `&#10;`. A newline reaching the output would end a line that the
//! grammar says continues, so every author-supplied string passes
//! through [`crate::comment_text::encode`] — the encoder the generated
//! comments already use, whose [`crate::comment_text::decode`] is its
//! exact inverse. Sharing it rather than writing a second escape
//! grammar is what lets one decoder serve both surfaces.
//!
//! # Grammar
//!
//! Indentation is two spaces per level and carries block structure;
//! order of lines is order in the document.
//!
//! ```text
//!   algorithm <name>(<param>: <type>, ...) [-> <type> [returns-max <n>]]
//!     const <name>: <type> = <expr>
//!     const <name>: array<<type>, <n>> = fold <var> in <a>..<b> -> <type>:
//!       <stmt>...
//!       yield <expr>
//!     var <name>: <type> [cap <n>] = <expr>
//!     <target> = <expr>
//!     append <target> <- <expr>
//!     if <cond>:
//!       <stmt>...
//!     else:
//!       <stmt>...
//!     while <cond> [max <n>]:
//!       <stmt>...
//!     foreach <item> in <source>:
//!       <stmt>...
//!     return [<expr>]
//!     call <target>(<arg>, ...)
//!     test <hex> -> (bool|uint|int) <literal> @line <n>
//!
//!   procedure <name> initial <state>
//!     in <id>: <type> <field-clause>...
//!     internal <id>: <type> <field-clause>...
//!     helper <name>(<type>, ...) -> <type> [returns-max <n>]
//!     state <id>:
//!       send <service> [subfunc <e>] [addr <e>] [payload <e>] [response-max <n>]
//!       on <event> when <cond> -> <target>
//!         <location> = <expr>
//!     final <id>:
//!       done <name> = <expr>
//! ```
//!
//! A `<field-clause>` is any of `= <expr>`, `max-size <n>`,
//! `quantity <scale> <offset> <unit>`, `retain <scope> initial <expr>`,
//! `default-covers <a> <b> ...`, each written only when the model
//! carries it, always in that order.
//!
//! A transition writes `on <event>` only when it has one and
//! `when <cond>` only when it has one, so an unconditional eventless
//! transition is the bare `-> <target>`.

use std::fmt::Write as _;

use crate::comment_text;
use crate::forge::model::{
    AlgorithmConst, AlgorithmConstType, AlgorithmModel, AlgorithmStmt, Direction, FoldBody,
    ForgeDocument, ForgeField, ProcedureHelper, ProcedureModel, ProcedureState,
    ProcedureTransition, SceType, TestVector, TestVectorValue,
};

/// Why a document has no pseudocode rendering, as opposed to an empty
/// one.
///
/// ⚠ Carries the kind's own name for the reason
/// [`crate::forge::requirement_nodes::ReviewScope`] gives: a bare marker
/// would make one unanswerable question look like every other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    /// `sce:kind` as authored, e.g. `codec`.
    pub kind: &'static str,
}

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SCE renders no pseudocode for the '{}' kind yet — a partial \
             rendering would read like a complete one, so the kind is \
             refused rather than abbreviated",
            self.kind
        )
    }
}

/// The pseudocode for this document, or why there is none.
///
/// The returned string ends in a newline when it is non-empty, so a
/// caller writing it to a file or to stdout needs no terminator of its
/// own.
pub fn render(doc: &ForgeDocument) -> Result<String, Unsupported> {
    // One arm per kind, listed rather than collapsed behind `_`, so a
    // kind added to the enum stops the build until somebody decides
    // whether it is rendered or refused.
    match doc {
        ForgeDocument::Algorithm(m) => Ok(render_algorithm(m)),
        ForgeDocument::Procedure(m) => Ok(render_procedure(m)),
        ForgeDocument::Statechart(_) => Err(Unsupported { kind: "statechart" }),
        ForgeDocument::Transform(_) => Err(Unsupported { kind: "transform" }),
        ForgeDocument::Lookup(_) => Err(Unsupported { kind: "lookup" }),
        ForgeDocument::Condition(_) => Err(Unsupported { kind: "condition" }),
        ForgeDocument::Codec(_) => Err(Unsupported { kind: "codec" }),
        ForgeDocument::Validator(_) => Err(Unsupported { kind: "validator" }),
        ForgeDocument::Filter(_) => Err(Unsupported { kind: "filter" }),
        ForgeDocument::Interpolation(_) => Err(Unsupported {
            kind: "interpolation",
        }),
        ForgeDocument::Timer(_) => Err(Unsupported { kind: "timer" }),
        ForgeDocument::Observer(_) => Err(Unsupported { kind: "observer" }),
        ForgeDocument::Link(_) => Err(Unsupported { kind: "link" }),
        ForgeDocument::BufferPool(_) => Err(Unsupported {
            kind: "buffer-pool",
        }),
        ForgeDocument::Worker(_) => Err(Unsupported { kind: "worker" }),
        ForgeDocument::BoundedCollection(_) => Err(Unsupported {
            kind: "bounded-collection",
        }),
        ForgeDocument::Enum(_) => Err(Unsupported { kind: "enum" }),
        ForgeDocument::EventSchema(_) => Err(Unsupported {
            kind: "event-schema",
        }),
    }
}

// ── Output buffer ──────────────────────────────────────────────

/// Accumulates lines at an indentation depth.
///
/// A plain `String` plus a depth counter rather than a formatter: the
/// grammar's only structural device is leading whitespace, and every
/// writer here appends whole lines.
struct Out {
    buf: String,
    depth: usize,
}

impl Out {
    fn new() -> Self {
        Out {
            buf: String::new(),
            depth: 0,
        }
    }

    /// One line at the current depth.
    fn line(&mut self, text: &str) {
        for _ in 0..self.depth {
            self.buf.push_str("  ");
        }
        self.buf.push_str(text);
        self.buf.push('\n');
    }

    /// Run `body` one level deeper.
    fn nested(&mut self, body: impl FnOnce(&mut Self)) {
        self.depth += 1;
        body(self);
        self.depth -= 1;
    }
}

/// Author-supplied text, encoded so no newline or escape introducer
/// reaches the output. See the module comment.
fn text(s: &str) -> std::borrow::Cow<'_, str> {
    comment_text::encode(s)
}

// ── Algorithm ──────────────────────────────────────────────────

fn render_algorithm(m: &AlgorithmModel) -> String {
    let mut out = Out::new();

    let params: Vec<String> = m
        .signature
        .params
        .iter()
        .map(|p| format!("{}: {}", text(&p.name), p.sce_type.as_attr()))
        .collect();
    let mut head = format!("algorithm {}({})", text(&m.name), params.join(", "));
    if let Some(ret) = &m.signature.return_type {
        let _ = write!(head, " -> {}", ret.as_attr());
        if let Some(max) = m.signature.returns_max_size {
            let _ = write!(head, " returns-max {max}");
        }
    }
    out.line(&head);

    out.nested(|out| {
        for c in &m.consts {
            render_const(c, out);
        }
        for stmt in &m.body {
            render_stmt(stmt, out);
        }
        for tv in &m.test_vectors {
            render_test_vector(tv, out);
        }
    });

    out.buf
}

fn render_const(c: &AlgorithmConst, out: &mut Out) {
    match (&c.sce_type, &c.init, &c.fold) {
        // Scalar form. `compute_at_build` is false here by the parser's
        // "exactly one of init / fold" invariant, so the keyword below
        // carries it without a second spelling.
        (AlgorithmConstType::Scalar(t), Some(init), _) => {
            out.line(&format!(
                "const {}: {} = {}",
                text(&c.name),
                t.as_attr(),
                text(init)
            ));
        }
        (AlgorithmConstType::Array { elem, len }, _, Some(fold)) => {
            out.line(&format!(
                "const {}: array<{}, {}> = {}",
                text(&c.name),
                elem.as_attr(),
                len,
                fold_head(fold)
            ));
            out.nested(|out| {
                for stmt in &fold.body {
                    render_stmt(stmt, out);
                }
                out.line(&format!("yield {}", text(&fold.yield_expr)));
            });
        }
        // The parser admits no other combination — a scalar with a fold
        // body, or an array with a literal init, is refused at parse
        // time. Rendering the shape verbatim rather than guessing keeps
        // this total without inventing a reading the parser forbids.
        (t, init, fold) => {
            out.line(&format!(
                "const {}: {} = <unrepresentable init={} fold={}>",
                text(&c.name),
                const_type_text(t),
                init.is_some(),
                fold.is_some()
            ));
        }
    }
}

fn const_type_text(t: &AlgorithmConstType) -> String {
    match t {
        AlgorithmConstType::Scalar(s) => s.as_attr(),
        AlgorithmConstType::Array { elem, len } => {
            format!("array<{}, {}>", elem.as_attr(), len)
        }
    }
}

fn fold_head(fold: &FoldBody) -> String {
    format!(
        "fold {} in {}..{} -> {}:",
        text(&fold.iter_var),
        fold.range_start,
        fold.range_end,
        fold.elem_type.as_attr()
    )
}

fn render_stmt(stmt: &AlgorithmStmt, out: &mut Out) {
    match stmt {
        AlgorithmStmt::Var {
            name,
            sce_type,
            init,
            capacity,
        } => {
            let cap = match capacity {
                Some(n) => format!(" cap {n}"),
                None => String::new(),
            };
            out.line(&format!(
                "var {}: {}{} = {}",
                text(name),
                sce_type.as_attr(),
                cap,
                text(init)
            ));
        }
        AlgorithmStmt::Assign { target, expr } => {
            out.line(&format!("{} = {}", text(target), text(expr)));
        }
        AlgorithmStmt::Append { target, expr } => {
            out.line(&format!("append {} <- {}", text(target), text(expr)));
        }
        AlgorithmStmt::If {
            cond,
            then_body,
            else_body,
        } => {
            out.line(&format!("if {}:", text(cond)));
            out.nested(|out| {
                for s in then_body {
                    render_stmt(s, out);
                }
            });
            // An absent `else` and an empty one are different documents
            // — `<sce:else/>` with no children is a block the author
            // wrote — so the keyword is printed whenever the model
            // carries `Some`, even for an empty body.
            if let Some(body) = else_body {
                out.line("else:");
                out.nested(|out| {
                    for s in body {
                        render_stmt(s, out);
                    }
                });
            }
        }
        AlgorithmStmt::While {
            cond,
            body,
            max_iter,
        } => {
            let max = match max_iter {
                Some(n) => format!(" max {n}"),
                None => String::new(),
            };
            out.line(&format!("while {}{}:", text(cond), max));
            out.nested(|out| {
                for s in body {
                    render_stmt(s, out);
                }
            });
        }
        AlgorithmStmt::Foreach { item, source, body } => {
            out.line(&format!("foreach {} in {}:", text(item), text(source)));
            out.nested(|out| {
                for s in body {
                    render_stmt(s, out);
                }
            });
        }
        AlgorithmStmt::Return { expr } => match expr {
            Some(e) => out.line(&format!("return {}", text(e))),
            None => out.line("return"),
        },
        AlgorithmStmt::Call { target, args } => {
            let rendered: Vec<String> = args.iter().map(|a| text(a).into_owned()).collect();
            out.line(&format!("call {}({})", text(target), rendered.join(", ")));
        }
    }
}

fn render_test_vector(tv: &TestVector, out: &mut Out) {
    let hex: String = tv.hex.iter().map(|b| format!("{b:02x}")).collect();
    let value = match &tv.value {
        TestVectorValue::Bool(b) => format!("bool {b}"),
        TestVectorValue::Uint(v) => format!("uint {v}"),
        TestVectorValue::Int(v) => format!("int {v}"),
    };
    // ⚠ `source_line` is printed although it is a position, not
    // behaviour. Unlike every other position in the IR it is NOT named
    // `source_location`, so a round-trip comparison that strips that one
    // key by name would still see this field differ. Printing it keeps
    // the rendering total and keeps the strip list at exactly one key;
    // the alternative — a second key in the strip list — is the
    // hand-kept exclusion list this design exists to avoid.
    out.line(&format!("test 0x{hex} -> {value} @line {}", tv.source_line));
}

// ── Procedure ──────────────────────────────────────────────────

fn render_procedure(m: &ProcedureModel) -> String {
    let mut out = Out::new();
    out.line(&format!(
        "procedure {} initial {}",
        text(&m.name),
        text(&m.initial)
    ));

    out.nested(|out| {
        for f in &m.inputs {
            render_field(f, out);
        }
        for f in &m.internals {
            render_field(f, out);
        }
        for h in &m.helpers {
            render_helper(h, out);
        }
        for s in &m.states {
            render_state(s, out);
        }
    });

    out.buf
}

/// One `<data>` field, with every clause the model carries.
///
/// `direction` is printed as the leading keyword rather than inferred
/// from which list the field came out of: the two agree today, and a
/// rendering that relies on them continuing to agree would be a
/// rendering that stops being total the day they do not.
fn render_field(f: &ForgeField, out: &mut Out) {
    let dir = match f.direction {
        Direction::In => "in",
        Direction::Out => "out",
        Direction::Internal => "internal",
    };
    let mut line = format!("{} {}: {}", dir, text(&f.id), f.sce_type.as_attr());
    if let Some(expr) = &f.expr {
        let _ = write!(line, " = {}", text(expr));
    }
    if let Some(max) = f.max_size {
        let _ = write!(line, " max-size {max}");
    }
    if let Some(q) = &f.quantity {
        let _ = write!(line, " quantity {} {} {}", q.scale, q.offset, q.unit);
    }
    if let Some(r) = &f.retain {
        let _ = write!(
            line,
            " retain {} initial {}",
            text(&r.scope),
            text(&r.initial)
        );
    }
    if !f.default_covers.is_empty() {
        let covers: Vec<String> = f
            .default_covers
            .iter()
            .map(|c| text(c).into_owned())
            .collect();
        let _ = write!(line, " default-covers {}", covers.join(" "));
    }
    out.line(&line);
}

fn render_helper(h: &ProcedureHelper, out: &mut Out) {
    let args: Vec<String> = h.args.iter().map(SceType::as_attr).collect();
    let mut line = format!(
        "helper {}({}) -> {}",
        text(&h.name),
        args.join(", "),
        h.returns.as_attr()
    );
    if let Some(max) = h.returns_max_size {
        let _ = write!(line, " returns-max {max}");
    }
    out.line(&line);
}

fn render_state(s: &ProcedureState, out: &mut Out) {
    let keyword = if s.is_final { "final" } else { "state" };
    out.line(&format!("{} {}:", keyword, text(&s.id)));
    out.nested(|out| {
        for send in &s.on_entry_sends {
            let mut line = format!("send {}", text(&send.service));
            if let Some(v) = &send.subfunc {
                let _ = write!(line, " subfunc {}", text(v));
            }
            if let Some(v) = &send.addr {
                let _ = write!(line, " addr {}", text(v));
            }
            if let Some(v) = &send.payload {
                let _ = write!(line, " payload {}", text(v));
            }
            if let Some(n) = send.response_max_size {
                let _ = write!(line, " response-max {n}");
            }
            out.line(&line);
        }
        for t in &s.transitions {
            render_transition(t, out);
        }
        for p in &s.done_params {
            out.line(&format!("done {} = {}", text(&p.name), text(&p.expr)));
        }
    });
}

fn render_transition(t: &ProcedureTransition, out: &mut Out) {
    let mut line = String::from("on");
    match &t.event {
        Some(e) => {
            let _ = write!(line, " {}", text(e));
        }
        // An eventless transition is the bare arrow: `on` with no event
        // would read as an event named by the empty string.
        None => line.clear(),
    }
    if let Some(c) = &t.cond {
        if !line.is_empty() {
            line.push(' ');
        }
        let _ = write!(line, "when {}", text(c));
    }
    if !line.is_empty() {
        line.push(' ');
    }
    let _ = write!(line, "-> {}", text(&t.target));
    out.line(&line);

    out.nested(|out| {
        for a in &t.assigns {
            out.line(&format!("{} = {}", text(&a.location), text(&a.expr)));
        }
    });
}
