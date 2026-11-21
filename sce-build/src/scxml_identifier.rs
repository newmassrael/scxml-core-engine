// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-3.3.1 — the grammar W3C gives an identifier-bearing attribute,
//! checked on the attribute itself.
//!
//! # What was wrong
//!
//! W3C types a state's `id` as an XML Schema `ID` (§3.3.1, §3.4.1, §3.7.1 —
//! *"A valid id as defined in [XML Schema]"*) and an event name as
//! dot-separated tokens (§3.12.1). SCE enforced neither.
//! `schemas/sce-forge.xsd` is `xs:any processContents="lax"` for W3C
//! structural elements, so the schema calls every spelling valid, and no
//! parse-time check stood in for it. Measured 2026-09-13, a document with
//! `<state id="s0*/X">` generated without a diagnostic and the id became a
//! code identifier — `…_STATE_S0*/X` in C, `S0*/X = 1` in Python — so the
//! emitted source did not compile. The author learned this from a C compiler
//! pointing at generated code they had never seen.
//!
//! # Why a document sweep rather than a check at each read
//!
//! `parser.rs` reads these attributes at roughly twenty sites, and a guard
//! per site is a guard the twenty-first forgets. The check is one pass over
//! the parsed tree instead, driven by [`IDENTIFIER_ATTRIBUTES`]: it is
//! exhaustive over the DOCUMENT rather than over the parser's call graph, so
//! an attribute a future parser reads somewhere new is already covered, and
//! extending it to a new attribute is a row rather than a call.
//!
//! It runs at parse — `parser::SCXMLParser::parse_impl`, immediately after
//! `reject_unexpanded_directives` — and not in a later analysis pass, for
//! two reasons. The tree is still in hand, so a rejection can name the
//! attribute's own line and column; and every later stage is downstream of
//! the model, where the offending text has already become a field.
//!
//! ⚠ Position matters for a second reason: the sweep must see the document
//! AFTER `<xi:include>` and `<sce:use>` expansion. Four values in
//! `tests/w3c_template_parity/fixtures/` are pre-expansion placeholders —
//! `s_{$id}`, `tick_{$n}` — and are the only values in this tree that any of
//! these grammars refuses. They are placeholders precisely because expansion
//! replaces them, so running after it is what makes them legal.
//!
//! # The grammars, and where they part company
//!
//! Two, not one, because W3C gives two.
//!
//! * [`Grammar::Id`] and [`Grammar::IdRefs`] — an XML Name, which
//!   [`is_ncname`] spells: a token starts with a letter or `_`, continues
//!   with alphanumerics, `_` or `-`, and `.` separates tokens.
//! * [`Grammar::EventDescriptors`] and [`Grammar::EventName`] — §3.12.1's
//!   token grammar, owned by [`crate::event_descriptor`] because that module
//!   is already the one definition of what a descriptor means.
//!
//! The two differ at the first character: an XML Name may not begin with a
//! digit and an event token may. That is not a tidy-up, it is what the two
//! specifications say, and both halves were measured rather than recalled:
//! reading §3.12.1's "alphanumeric" literally rejects W3C's own conformance
//! documents 364 and 576 (`In-s11p112`), and reading it as an XML Name
//! rejects §3.12.1's own calculator example (`<transition event="DIGIT.0">`).
//!
//! # What SCE narrows, stated rather than hidden
//!
//! Both grammars here are ASCII, and the event grammar refuses `:`. W3C is
//! wider on both counts — an XML Name admits most of Unicode's letters, and
//! §3.12.1 shows `<transition event="ccxml:connection.alerting"/>` under the
//! words *"This markup is legal"*.
//!
//! SCE is narrower because these values do not stay in the document: an id
//! becomes a code identifier in C, C++, Kotlin, Rust, Go and Python, and an
//! event name becomes one too. A value this sweep admits is one all six can
//! carry. A value it refuses is refused at the author's own line rather than
//! by a compiler reading generated source.
//!
//! Measured 2026-09-14 over the 735 SCXML documents this repository COMMITS
//! — the tracked-file enumeration `scxml_documents` performs in
//! `tests/an_identifier_is_checked_against_the_grammar_w3c_gives_it.rs`,
//! which is the file that spells the command — across
//! every row of [`IDENTIFIER_ATTRIBUTES`]: the narrowing costs nothing here —
//! zero documents use a Unicode or `:`-bearing identifier — but it is a
//! boundary SCE draws and not one W3C drew, which is why it is written down
//! here and registered in `docs/SCE_ACCEPTED_SUBSET.md` §1 rather than left
//! for a rejected author to discover.
//!
//! ⚠ The enumeration is named because the number alone rotted once. This
//! paragraph read "794 documents" until 2026-09-14, a count taken by walking
//! the filesystem past a hand-written skip list — which counted generated
//! build output as source, and which the corpus test dropped for exactly that
//! reason. A bare total invites the reader to re-take it with whatever walk
//! comes to hand, and the two walks do not agree — a `find` over this tree
//! answers a different number on every machine, because it counts whatever
//! the last build wrote.
//!
//! ⚠ The enumeration is NAMED here and spelled only in that test, and the
//! asymmetry is required rather than stylistic:
//! `workflow_trigger_coverage::no_library_this_suite_links_reads_past_a_filter_unattributed`
//! refuses a tree-wide read signature appearing in a LIBRARY source, because
//! a test that calls into one inherits its input set while the detector
//! follows only the module tree a test declares. Writing the command back
//! into this paragraph reddens that gate — measured 2026-09-14, by doing it.
//!
//! # A third grammar: the names the generated code spells
//!
//! W3C's table covers W3C's elements. An `sce:` element names things too —
//! a codec field, an import alias, an enum variant, an algorithm variable —
//! and those names reach generated source verbatim, in all six languages,
//! and are read by name in forge expressions, whose lexer is ASCII. So is a
//! forge document's `<data id>` ([`Dialect`] says why a statechart's is
//! not). They are held to [`Grammar::CodeIdentifier`] — no `-`, no `.`,
//! since both are operators in every target — and a reference through a
//! record to [`Grammar::CodePath`], listed in [`SCE_IDENTIFIER_ATTRIBUTES`].
//!
//! ⚠ The forge pipeline ran no sweep at all until 2026-09-22, measured
//! twice: a transform whose `<data id>` was two Hangul letters passed the
//! parse and was refused by the expression lexer with no line, and one
//! whose `<data id>` was `raw-value` passed `check` in all six languages
//! and generated C++ that does not compile. Both parsers now run the same
//! sweep, each under its own [`Dialect`], and the refusal is
//! `validation/malformed-code-identifier` on the attribute's own line —
//! the ASCII boundary W3C's table already draws, drawn for the names SCE
//! owns.
//!
//! One step further, for the same reason: a code identifier of the right
//! shape can still be a word one target language reserves — `override` in
//! Rust, `pass` in Python, `auto` in C++ — and then that backend cannot
//! declare it. Such a name is refused for every backend at once
//! (`validation/reserved-code-identifier`, naming the language and the
//! spelling), from the one keyword list `crate::reader_names::reserved_in`
//! keeps. Refusing rather than escaping, because the forge generator spells
//! these names at many sites per kind, and an escape applied at some of them
//! is a mismatch at the rest. A statechart's `<data id>` is W3C's name, not
//! SCE's, and is escaped where it becomes a reader instead.
//!
//! ⚠ "Reserved" is asked of the name AS EACH BACKEND SPELLS IT, which
//! [`spellings_of`] records per attribute: a const is `UPPER_SNAKE`
//! everywhere, a variant is `Pascal` in Rust and C++, a codec field is
//! `Pascal` in Go and `camelCase` in Kotlin. The first version folded every
//! name to snake_case and refused a const `DEFAULT` as C++'s `default` —
//! measured by CI, 2026-09-26.

use crate::forge::error::{ForgeError, Located, ValidationError};

/// The grammar one attribute is held to — W3C's, or SCE's for a name the
/// generated code spells verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    /// An XML Schema `ID`: exactly one XML Name, declaring it.
    Id,
    /// An XML Schema `IDREFS`: whitespace-separated XML Names, referencing.
    IdRefs,
    /// §3.12.1 event descriptors, whitespace-separated. Wildcards legal.
    EventDescriptors,
    /// §3.12.1 event name — one event, so no wildcard spelling is legal.
    EventName,
    /// A name the generated code spells as it is written — a forge field,
    /// an import alias, a codec member: [`is_code_identifier`].
    CodeIdentifier,
    /// A reference to such a name through the records that hold it —
    /// `header.S`, `telemetry.reset`: code identifiers joined by `.`.
    CodePath,
}

/// Which pipeline reads the document, and so which grammar its `<data id>`
/// is held to.
///
/// The two share every row but that one. A statechart's `<data id>` names
/// a data-model location, and W3C types it `xs:ID`. A forge document's
/// names a value its own expressions read by that name and its generated
/// code declares under it, so it is a [`Grammar::CodeIdentifier`]: an XML
/// Name admits `raw-value`, and measured 2026-09-22 a transform declaring
/// it passed `check` in all six languages and generated the C++ parameter
/// `int32_t raw - value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Statechart,
    Forge,
}

impl Dialect {
    /// The dialect of the document `root` is the root element of: a forge
    /// document names its kind on the root, a statechart names none or
    /// names `statechart`.
    pub fn of(root: &roxmltree::Node) -> Self {
        match root.attribute((crate::forge::model::SCE_NAMESPACE, "kind")) {
            Some(kind) if kind != "statechart" => Dialect::Forge,
            _ => Dialect::Statechart,
        }
    }
}

/// Every identifier-bearing attribute of W3C SCXML, with the grammar its
/// clause gives it.
///
/// The table is what makes this rule extensible: a new attribute is a row.
///
/// ⚠ It is deliberately not "every attribute that looks like a name".
/// `<param name>` and `<scxml name>` are `NMTOKEN`, a different XML Schema
/// type with a different grammar — a leading digit is legal in an NMTOKEN —
/// and `<invoke idlocation>` names a data-model location rather than an id.
/// Putting any of the three here under [`Grammar::Id`] would refuse
/// documents W3C accepts, so the table's membership rule is the W3C
/// TYPE of the attribute, not the spelling of its name.
pub const IDENTIFIER_ATTRIBUTES: &[(&str, &str, Grammar)] = &[
    // ── Declarations: W3C types each of these `ID`. ──────────────
    ("state", "id", Grammar::Id),    // §3.3.1
    ("parallel", "id", Grammar::Id), // §3.4.1
    ("final", "id", Grammar::Id),    // §3.7.1
    ("history", "id", Grammar::Id),  // §3.10.1
    ("data", "id", Grammar::Id),     // §5.2.1
    ("invoke", "id", Grammar::Id),   // §6.4.1
    ("send", "id", Grammar::Id),     // §6.2.1
    // ── References: `IDREFS`, so a list. ─────────────────────────
    ("transition", "target", Grammar::IdRefs), // §3.13
    ("scxml", "initial", Grammar::IdRefs),     // §3.2.1
    ("state", "initial", Grammar::IdRefs),     // §3.3.1
    // ⚠ No `("parallel", "initial", …)` row. §3.4.1 gives `<parallel>` one
    // attribute, `id`, and a row for an attribute W3C does not define is a
    // row no document can reach — dead weight that reads as coverage.
    // ── Event names and descriptors, §3.12.1. ────────────────────
    ("transition", "event", Grammar::EventDescriptors), // §3.13
    ("raise", "event", Grammar::EventName),             // §4.5
    ("send", "event", Grammar::EventName),              // §6.2.1
];

/// Every `sce:` element attribute whose value is a name the generated code
/// spells, with the grammar it is held to — in a statechart's `sce:`
/// elements and a forge document's alike.
///
/// ⚠ The membership rule is what the VALUE becomes, as it is for W3C's
/// table: a name this document declares, or refers to, that reaches
/// generated source as an identifier. A name of ANOTHER document — a
/// pool's, a framer's, a machine's `ref` — is that document's name, whose
/// grammar is the document's, and is not here. Neither is a key or a value
/// (`<sce:entry key>`), which the code holds as data.
///
/// Measured 2026-09-22 over every tracked `.scxml` document: each row's
/// values already satisfy its grammar, including the four dotted
/// references (`header.S`, `telemetry.reset`) that are why the reference
/// rows are paths.
pub const SCE_IDENTIFIER_ATTRIBUTES: &[(&str, &str, Grammar)] = &[
    // ── Declarations: the name the generated code declares. ──────
    ("import", "as", Grammar::CodeIdentifier),
    ("context", "id", Grammar::CodeIdentifier),
    ("field", "id", Grammar::CodeIdentifier),
    ("peek-byte", "id", Grammar::CodeIdentifier),
    ("flags", "id", Grammar::CodeIdentifier),
    ("flag", "name", Grammar::CodeIdentifier),
    ("flag-input", "name", Grammar::CodeIdentifier),
    ("repeat", "id", Grammar::CodeIdentifier),
    ("tlv-chain", "id", Grammar::CodeIdentifier),
    ("embed", "id", Grammar::CodeIdentifier),
    ("variant", "name", Grammar::CodeIdentifier),
    ("cycle", "id", Grammar::CodeIdentifier),
    ("var", "name", Grammar::CodeIdentifier),
    ("const", "name", Grammar::CodeIdentifier),
    ("fold", "as", Grammar::CodeIdentifier),
    ("helper", "name", Grammar::CodeIdentifier),
    ("extern", "name", Grammar::CodeIdentifier),
    ("action", "name", Grammar::CodeIdentifier),
    ("arg", "name", Grammar::CodeIdentifier),
    // An algorithm's parameter: a name in the same function as its variables,
    // spelled by the same rule, and held to the same grammar and reserved words.
    ("param", "name", Grammar::CodeIdentifier),
    // ── References to one name of this document or its imports. ──
    ("flag-bind", "input", Grammar::CodeIdentifier),
    ("entry-flag-bind", "input", Grammar::CodeIdentifier),
    ("decoded", "field", Grammar::CodeIdentifier),
    ("cycle", "of", Grammar::CodeIdentifier),
    ("step", "name", Grammar::CodeIdentifier),
    // ── References through a record: dotted paths. ────────────────
    ("flag-bind", "source", Grammar::CodePath),
    ("assign", "target", Grammar::CodePath),
    ("append", "target", Grammar::CodePath),
    ("call", "target", Grammar::CodePath),
    // `<sce:lift>` (docs/adr/0087, section 2.1) names only what is declared
    // elsewhere, so a path's rule holds each, with no keyword refused here: the
    // text it reads (a variable, a field of a record variable or of the payload,
    // `_event.data.text`), the alias of the schema, and the two variables it
    // writes, each one segment.
    ("lift", "text", Grammar::CodePath),
    ("lift", "schema", Grammar::CodePath),
    ("lift", "location", Grammar::CodePath),
    ("lift", "refusal", Grammar::CodePath),
    // `<sce:render>` (docs/adr/0087, section 3.1, item 6) names the alias of
    // the template it renders and the variable it writes, each declared
    // elsewhere, so a path's rule holds each, with no keyword refused here.
    ("render", "template", Grammar::CodePath),
    ("render", "location", Grammar::CodePath),
];

/// Whether `name` is an XML Name in the ASCII form SCE accepts — the
/// grammar W3C's `ID` and `IDREF` defer to XML Schema for.
///
/// A token starts with a letter or `_`, continues with alphanumerics, `_` or
/// `-`, and `.` separates tokens.
///
/// ⚠ `.` is written as a SEPARATOR, and that is a third narrowing beside the
/// two the module header states. XML makes `.` an ordinary Name character, so
/// a true NCName admits `a.2b` — first character a letter, everything after
/// it a NameChar — and this refuses it, because the token after the dot
/// begins with a digit. The shape is deliberate: it is how §3.12.1 segments
/// an event name, and one shape for both is what lets the two grammars be
/// compared at the only place they differ. Measured 2026-09-14, no document
/// in this tree writes a dotted identifier whose later token starts with a
/// digit, so the narrowing costs nothing here — but it is a narrowing, and a
/// reader who finds `a.2b` refused should find the reason here rather than
/// conclude the check is broken.
pub fn is_ncname(name: &str) -> bool {
    !name.is_empty() && name.split('.').all(is_ncname_token)
}

/// One token of an XML Name: the unit `.` separates.
fn is_ncname_token(token: &str) -> bool {
    let mut chars = token.chars();
    match chars.next() {
        None => false,
        Some(first) if !(first.is_ascii_alphabetic() || first == '_') => false,
        Some(_) => chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
    }
}

/// Whether `name` is a code identifier: an ASCII letter or `_`, then ASCII
/// letters, digits or `_` — the spelling C, C++, Kotlin, Rust, Go and
/// Python all accept as an unquoted identifier, and the one the forge
/// expression language lexes.
///
/// Deliberately narrower than any one of those languages, which admit `$`
/// or Unicode letters in some form: the value is emitted verbatim into all
/// six, so the grammar is their intersection, not what one compiler takes.
/// The narrowing is registered in `docs/SCE_ACCEPTED_SUBSET.md` §2.14.
pub fn is_code_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// The production name a rejection reports in `expected`.
///
/// A grammar production, not a repair candidate: `SCE_ERROR_CONTRACT.md` §3.2
/// keeps the two fields disjoint, and there is no closed set of legal
/// replacements for a name, so these records carry `expected` and no `fix` —
/// the shape `expression/parse-mismatch` already uses.
/// Named for the TOKEN rather than the whole attribute: `target="a b*c"`
/// fails on one of its two names, and telling the author their attribute is
/// not an `xs:IDREFS` when one word in it is the problem is the less useful
/// half of what the parser knows.
fn production(grammar: Grammar) -> &'static str {
    match grammar {
        Grammar::Id => "xs:ID",
        Grammar::IdRefs => "xs:IDREF",
        Grammar::EventDescriptors => "event descriptor",
        Grammar::EventName => "event name",
        Grammar::CodeIdentifier => "code identifier",
        Grammar::CodePath => "code identifier path",
    }
}

/// The first token of `value` that `grammar` refuses, or `None`.
///
/// Public so a caller holding a value without a document — a generator
/// synthesising an event name, say — can ask the same question the sweep
/// asks, rather than writing a second answer to it.
pub fn offending_token(grammar: Grammar, value: &str) -> Option<&str> {
    match grammar {
        // A single value. Present-and-empty is refused: `id=""` declares a
        // name that is not one, and no document in this tree writes it.
        Grammar::Id => (!is_ncname(value)).then_some(value),
        Grammar::EventName => (!crate::event_descriptor::is_event_name(value)).then_some(value),
        // A list. An attribute holding no tokens at all is a different rule
        // (`validation/empty-value`) and not this one's to raise, so an
        // empty list has nothing to refuse.
        Grammar::IdRefs => value.split_whitespace().find(|token| !is_ncname(token)),
        Grammar::EventDescriptors => value
            .split_whitespace()
            .find_map(crate::event_descriptor::malformed_token),
        Grammar::CodeIdentifier => (!is_code_identifier(value)).then_some(value),
        // The segment that offends, as a list reports its token: in
        // `header.S-1` it is `S-1`, and an empty segment (`a..b`) is
        // reported as the empty name it is.
        Grammar::CodePath => value
            .split('.')
            .find(|segment| !is_code_identifier(segment)),
    }
}

/// The grammar `dialect` holds `node`'s unqualified attribute `attr` to, or
/// `None` when that attribute carries no name.
///
/// A `<data id>` is a code identifier wherever it names a field of generated
/// code: in a forge document, and in a statechart whose data model is
/// `sce-static` (docs/SCE_ACCEPTED_SUBSET.md §2.15) — judged by the nearest
/// `<scxml>`, since an inline `<invoke>` document declares its own.
fn grammar_of(dialect: Dialect, node: &roxmltree::Node, attr: &str) -> Option<Grammar> {
    let element = node.tag_name().name();
    let table = match node.tag_name().namespace() {
        Some(crate::model::SCXML_NAMESPACE) => {
            if element == "data"
                && attr == "id"
                && (dialect == Dialect::Forge || declares_static_datamodel(node))
            {
                return Some(Grammar::CodeIdentifier);
            }
            IDENTIFIER_ATTRIBUTES
        }
        Some(crate::forge::model::SCE_NAMESPACE) => SCE_IDENTIFIER_ATTRIBUTES,
        _ => return None,
    };
    table
        .iter()
        .find(|(tag, name, _)| *tag == element && *name == attr)
        .map(|&(_, _, grammar)| grammar)
}

/// What the generated Python makes of the name `node`'s attribute `attr`
/// declares, when it is a name of the class or call the codec generates:
/// a field, flag, chain, repeat or embed is the dataclass attribute or its
/// accessor, and so is an entry of a CBOR map or of a content-line component
/// (a `<data>` of its datamodel that is not the input frame, a parameter of a
/// line too), a flag-input is a parameter of `decode` and `encode`; and a
/// procedure's input or helper is given a public setter `set_<name>`, which the
/// base class already has for the service handler. `None` for a name that is
/// only ever a local of a function body, which the generator keeps apart from
/// an author's names itself, and for one that is not declared in a codec or a
/// procedure.
fn python_declaration(
    node: &roxmltree::Node,
    attr: &str,
) -> Option<crate::reader_names::PythonDeclaration> {
    use crate::reader_names::PythonDeclaration::{CallParameter, ClassAttribute, ProcedureSetter};
    let root = node.document().root_element();
    let kind = root.attribute((crate::forge::model::SCE_NAMESPACE, "kind"));
    let in_a_procedure = kind == Some("procedure");
    if node.tag_name().namespace() == Some(crate::model::SCXML_NAMESPACE) {
        if node.tag_name().name() != "data" || attr != "id" {
            return None;
        }
        let an_input =
            node.attribute((crate::forge::model::SCE_NAMESPACE, "direction")) == Some("in");
        // A procedure's `<data>` is an input, and has a setter, when it
        // declares `sce:direction="in"`; an internal has none.
        if in_a_procedure {
            return an_input.then_some(ProcedureSetter);
        }
        // A CBOR map's or a content-line component's entries are `<data>`, and
        // each is an attribute of the value's dataclass and a member of the
        // `Protocol` its source satisfies, as a positional codec's field is;
        // the input frame names no field.
        let map_or_line = kind == Some("codec")
            && matches!(
                root.attribute((crate::forge::model::SCE_NAMESPACE, "encoding")),
                Some("cbor" | "content-line")
            );
        return (map_or_line && !an_input).then_some(ClassAttribute);
    }
    if node.tag_name().namespace() != Some(crate::forge::model::SCE_NAMESPACE) {
        return None;
    }
    match (node.tag_name().name(), attr) {
        ("helper", "name") if in_a_procedure => Some(ProcedureSetter),
        ("field" | "flags" | "repeat" | "tlv-chain" | "embed", "id") => Some(ClassAttribute),
        // Inside `<sce:peek-byte>` a flag is only a mask lookup.
        ("flag", "name")
            if !node
                .parent_element()
                .is_some_and(|p| p.tag_name().name() == "peek-byte") =>
        {
            Some(ClassAttribute)
        }
        ("flag-input", "name") => Some(CallParameter),
        _ => None,
    }
}

/// Why a parameter [`line_record_member`] finds is refused, as the clause
/// `validation/reserved-code-identifier` gives after the name.
const LINE_RECORD_MEMBER: &str = "a member the record of one line of a repeated property \
                                  declares for the line's value, beside the line's parameters";

/// How every backend spells a member of a content-line line record, a
/// parameter's and the line's value's alike (`LangCtx::codec_field_id`), in
/// [`crate::generator::Language::ALL`] order: Rust, Python and C11 snake_case,
/// Go Pascal, C++, Kotlin and Swift as written.
const LINE_RECORD_SPELLING: [crate::reader_names::Case; 7] = [
    crate::reader_names::Case::Snake,
    crate::reader_names::Case::Verbatim,
    crate::reader_names::Case::Verbatim,
    crate::reader_names::Case::Pascal,
    crate::reader_names::Case::Snake,
    crate::reader_names::Case::Snake,
    crate::reader_names::Case::Verbatim,
];

/// The first backend, in [`crate::generator::Language::ALL`] order, that
/// spells the parameter `node`'s attribute `attr` declares as a member the
/// record of its line already declares for the line's value, with that
/// spelling; `None` for any other name.
///
/// A parameter of a repeated property is a member of the record each line of
/// the property is read into (docs/adr/0014), and every backend names the
/// line's value in that record `value`, or `values` where `sce:separator`
/// makes it a list (`content_line_codec::entries_context`, `value_member`).
/// C11 holds a text beside its length and a list beside its length and count
/// (`codec_content_line.h.jinja2`), so it also declares `value_len` for a
/// text, and `values_len` and `values_count` for a list. A parameter spelled
/// as one of them declared that member twice: a duplicate member in Rust, C11,
/// C++ and Go, two properties of one name in Kotlin and Swift, and in Python
/// one dataclass field whose second annotation replaced the first, so the
/// parameter was read into the value's place. No name of the member is out of
/// an author's reach, since an author may write any code identifier, so the
/// member is not renamed: the parameter is refused, for every backend at once,
/// as a reserved word is.
fn line_record_member(
    node: &roxmltree::Node,
    attr: &str,
) -> Option<(crate::generator::Language, String)> {
    use crate::forge::model::SCE_NAMESPACE;
    if node.tag_name().namespace() != Some(crate::model::SCXML_NAMESPACE)
        || node.tag_name().name() != "data"
        || attr != "id"
    {
        return None;
    }
    let root = node.document().root_element();
    if root.attribute((SCE_NAMESPACE, "kind")) != Some("codec")
        || root.attribute((SCE_NAMESPACE, "encoding")) != Some("content-line")
    {
        return None;
    }
    node.attribute((SCE_NAMESPACE, "param"))?;
    let property = node.attribute((SCE_NAMESPACE, "property"))?;
    // The property's own entry, which its parameters follow
    // (`parse_content_line_entry`); without one the parser refuses the
    // parameter for that.
    let owner = node.prev_siblings().skip(1).find(|sibling| {
        sibling.is_element()
            && sibling.tag_name().name() == "data"
            && sibling.attribute((SCE_NAMESPACE, "param")).is_none()
            && sibling
                .attribute((SCE_NAMESPACE, "property"))
                .is_some_and(|p| p.eq_ignore_ascii_case(property))
    })?;
    // A parameter of a property that occurs once is a member of the codec's
    // value, beside the property's own entry, and is no record's.
    if owner.attribute((SCE_NAMESPACE, "repeated")) != Some("true") {
        return None;
    }
    let text = !owner
        .attribute((SCE_NAMESPACE, "type"))
        .is_some_and(|ty| ty.contains("enum:"));
    let (member, c11_beside): (&str, &[&str]) =
        if owner.attribute((SCE_NAMESPACE, "separator")).is_some() {
            ("values", &["values_len", "values_count"])
        } else if text {
            ("value", &["value_len"])
        } else {
            ("value", &[])
        };
    let id = node.attribute(attr)?;
    crate::generator::Language::ALL
        .iter()
        .copied()
        .zip(LINE_RECORD_SPELLING)
        .find_map(|(language, case)| {
            let spelled = case.spell(id);
            let beside: &[&str] = if language == crate::generator::Language::C11 {
                c11_beside
            } else {
                &[]
            };
            std::iter::once(member)
                .chain(beside.iter().copied())
                .any(|declared| case.spell(declared) == spelled)
                .then_some((language, spelled))
        })
}

/// How each backend spells the name `node`'s attribute `attr` declares, for
/// the reserved-word check — the folds a keyword could equal, in
/// [`crate::generator::Language::ALL`] order (Rust, C++, Kotlin, Go,
/// Python, C11, Swift). Swift has no emitter yet, so its column states the
/// spelling its templates will give each declaration (the API Design
/// Guidelines: lowerCamelCase for values and cases), which is what a name is
/// refused against now so that a document accepted today is not refused the
/// day Swift generates.
///
/// Measured from the forge generator and its templates, 2026-09-26, one row
/// per declaration; a form that adds to the name (`alias_`, `set_<name>`,
/// `TYPE_<NAME>`, `<struct>_<name>`) is left out because no keyword has that
/// shape. A reference names something declared elsewhere, and is refused
/// there; a name that never reaches source (`peek-byte`'s id is a fixed
/// local, a `cycle` is expanded away, a `fold` binding is build-time only)
/// declares nothing.
fn spellings_of(
    dialect: Dialect,
    node: &roxmltree::Node,
    attr: &str,
) -> &'static crate::reader_names::Spellings {
    use crate::reader_names::{Case::*, Spellings, NOT_DECLARED};
    // A forge `<data id>` is a parameter, a local, an output field, a payload
    // field and a Lookup's Pascal type name across its kinds.
    const FORGE_DATA: Spellings = [
        &[Verbatim, Snake, Pascal],
        &[Verbatim, Pascal],
        &[Verbatim, Camel, Pascal],
        &[Verbatim, Pascal],
        &[Verbatim, Snake, Pascal],
        &[Verbatim, Snake, Pascal],
        &[Verbatim, Camel, Pascal],
    ];
    // Rust and Kotlin lower an `sce-static` statechart (a Rust field and its
    // accessor snake_case, a Kotlin property camelCase); the rest refuse it.
    const STATIC_DATA: Spellings = [&[Snake], &[], &[Camel], &[], &[], &[], &[]];
    const IMPORT_AS: Spellings = [
        &[Verbatim],
        &[],
        &[Verbatim],
        &[Pascal],
        &[Verbatim],
        &[],
        &[Verbatim],
    ];
    const CONTEXT_ID: Spellings = [&[], &[Verbatim], &[Camel], &[], &[], &[], &[Camel]];
    // Go names a codec member's method Pascal and, where the value is printed
    // from its plan, holds the member in an unexported field of the Pascal
    // name with its first letter lowered (`go_source::member`), where a
    // keyword is no field name: `map`, `Map` and `map_` are all `map` there.
    const CODEC_FIELD: Spellings = [
        &[Snake],
        &[Verbatim],
        &[Verbatim, Camel],
        &[Pascal, LowerPascal],
        &[Snake],
        &[Snake],
        &[Verbatim, Camel],
    ];
    const FLAG: Spellings = [
        &[Snake],
        &[Snake],
        &[Camel],
        &[Pascal],
        &[Snake],
        &[],
        &[Camel],
    ];
    const FLAG_INPUT: Spellings = [
        &[Snake],
        &[Snake],
        &[Camel],
        &[Camel],
        &[Snake],
        &[Snake],
        &[Camel],
    ];
    // Swift names an enum case in lowerCamelCase, so `internal` and `init` are
    // refused where they are declared (the reserved-word rule, not an escape).
    const VARIANT: Spellings = [
        &[Pascal],
        &[Pascal],
        &[UpperSnake],
        &[],
        &[UpperSnake],
        &[],
        &[Camel],
    ];
    const VAR: Spellings = [
        &[Snake],
        &[Verbatim],
        &[Verbatim],
        &[Verbatim],
        &[Snake],
        &[Snake],
        &[Verbatim],
    ];
    // Swift writes a constant in lowerCamelCase, not in capitals.
    const CONST: Spellings = [
        &[UpperSnake],
        &[UpperSnake],
        &[UpperSnake],
        &[UpperSnake],
        &[UpperSnake],
        &[UpperSnake],
        &[Camel],
    ];
    // Python writes a helper where it is called (`compute_key(seed)`), so the
    // name reaches source there as it does in the rest, even though a procedure
    // stores it as `self._<name>`: a helper called `def` was `def(seed)`.
    const HELPER: Spellings = [
        &[Snake],
        &[Verbatim],
        &[Verbatim],
        &[Verbatim],
        &[Snake],
        &[Snake],
        &[Verbatim],
    ];
    // A procedure's states are the members of one enum, spelled Pascal in Rust,
    // C++, Kotlin and Python (`procedure-state` in `forge::declared_names`); Go
    // and C11 write one under a prefix, so no keyword can be one there. A state
    // called `false` is the member `False` in Python, which cannot be assigned.
    // Swift's members are lowerCamelCase like every Swift enum case, so a state
    // called `init` is the case `init`, which Swift reads as the initializer.
    const STATE: Spellings = [
        &[Pascal],
        &[Pascal],
        &[Pascal],
        &[],
        &[Pascal],
        &[],
        &[Camel],
    ];
    // Go, Kotlin, Python and Swift refuse `<sce:extern>`.
    const EXTERN: Spellings = [&[Verbatim], &[Verbatim], &[], &[], &[], &[Verbatim], &[]];
    const ACTION: Spellings = [
        &[Snake],
        &[Camel],
        &[Camel],
        &[Pascal],
        &[Snake],
        &[Snake],
        &[Camel],
    ];
    const ARG: Spellings = [
        &[Snake],
        &[Camel],
        &[Camel],
        &[Camel],
        &[Snake],
        &[Snake],
        &[Camel],
    ];

    let element = node.tag_name().name();
    match node.tag_name().namespace() {
        Some(crate::model::SCXML_NAMESPACE) if element == "data" && attr == "id" => {
            if dialect == Dialect::Forge {
                &FORGE_DATA
            } else {
                &STATIC_DATA
            }
        }
        // A forge procedure's `<state>` and `<final>`. A statechart's are the
        // reader's (`reader_names`), spelled and escaped there.
        Some(crate::model::SCXML_NAMESPACE)
            if matches!(element, "state" | "final")
                && attr == "id"
                && dialect == Dialect::Forge =>
        {
            &STATE
        }
        Some(crate::forge::model::SCE_NAMESPACE) => match (element, attr) {
            ("import", "as") => &IMPORT_AS,
            ("context", "id") => &CONTEXT_ID,
            ("field" | "flags" | "repeat" | "tlv-chain" | "embed", "id") => &CODEC_FIELD,
            // Inside `<sce:peek-byte>` a flag is only a mask lookup.
            ("flag", "name")
                if node
                    .parent_element()
                    .is_some_and(|p| p.tag_name().name() == "peek-byte") =>
            {
                &NOT_DECLARED
            }
            ("flag", "name") => &FLAG,
            ("flag-input", "name") => &FLAG_INPUT,
            ("variant", "name") => &VARIANT,
            // A parameter and a variable are names of one function, spelled by
            // one rule in every backend (`LangCtx::place_param`, `forge_local_id`).
            ("var" | "param", "name") => &VAR,
            ("const", "name") => &CONST,
            ("helper", "name") => &HELPER,
            ("extern", "name") => &EXTERN,
            ("action", "name") => &ACTION,
            ("arg", "name") => &ARG,
            _ => &NOT_DECLARED,
        },
        _ => &NOT_DECLARED,
    }
}

/// Whether the `<scxml>` that owns `node` declares `datamodel="sce-static"`.
fn declares_static_datamodel(node: &roxmltree::Node) -> bool {
    node.ancestors()
        .find(|a| {
            a.tag_name().name() == "scxml"
                && a.tag_name().namespace() == Some(crate::model::SCXML_NAMESPACE)
        })
        .and_then(|scxml| scxml.attribute("datamodel"))
        == Some(crate::model::Datamodel::SceStatic.as_str())
}

/// `node`'s tag as the document spells it — `sce:field`, `state` — so a
/// refusal names the element the author wrote.
pub(crate) fn spelled_tag(node: &roxmltree::Node) -> String {
    let local = node.tag_name().name();
    match node
        .tag_name()
        .namespace()
        .and_then(|namespace| node.lookup_prefix(namespace))
    {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}:{local}"),
        _ => local.to_string(),
    }
}

/// Refuse a document whose identifier-bearing attributes do not satisfy the
/// grammar W3C gives them — or, for an `sce:` element's name and a forge
/// document's `<data id>`, the code identifier grammar SCE gives them.
///
/// The first violation in document order, with the attribute value's own
/// line and column — `SCE_ERROR_CONTRACT.md` §3.1.1 requires the value in
/// `actual` to occur on the reported line, and an attribute is not always on
/// its element's line.
///
/// `root` must be the post-expansion tree; see the module header.
pub fn reject_malformed(
    root: &roxmltree::Node,
    doc_name: &str,
    dialect: Dialect,
) -> Result<(), Located<ForgeError>> {
    for node in root.descendants() {
        if !node.is_element() {
            continue;
        }
        for attribute in node.attributes() {
            // Unprefixed only. `conf:id` and `sce:*` attributes share a
            // local name with a row here and are governed by whoever owns
            // that namespace, not by these grammars.
            if attribute.namespace().is_some() {
                continue;
            }
            let Some(grammar) = grammar_of(dialect, &node, attribute.name()) else {
                continue;
            };
            let document = node.document();
            let pos = document.text_pos_at(attribute.range_value().start);
            let element = spelled_tag(&node);
            let attr = attribute.name().to_string();
            // As the document spells it, which is what a consumer finds on
            // the row: `a&amp;b` there, where the reader decoded `a&b`.
            let value = document.input_text()[attribute.range_value()].to_string();

            let Some(token) = offending_token(grammar, attribute.value()) else {
                // A code identifier of the right shape can still be a word a
                // target language reserves, and then that backend cannot
                // declare it. A path only references names declared
                // elsewhere, and those are refused where they are declared. A
                // forge procedure's state and final ids are `xs:ID` and still
                // become the members of an enum (`spellings_of`), so a keyword
                // is as much a name they cannot be.
                let declares_code_name = grammar == Grammar::CodeIdentifier
                    || (grammar == Grammar::Id
                        && dialect == Dialect::Forge
                        && matches!(node.tag_name().name(), "state" | "final"));
                if declares_code_name {
                    // A keyword is not the only thing Python cannot take: a
                    // field is the generated class's attribute, a flag its
                    // accessor, a flag-input a parameter of its calls, and the
                    // class already uses a few names for itself.
                    // A parameter of a repeated content-line property cannot
                    // be the member its line record holds the line's value by.
                    let reserved = crate::reader_names::reserved_in(
                        attribute.value(),
                        spellings_of(dialect, &node, attribute.name()),
                    )
                    .map(|(language, spelled)| (language, spelled, None::<&'static str>))
                    .or_else(|| {
                        let kind = python_declaration(&node, attribute.name())?;
                        let spelled = crate::reader_names::Case::Snake.spell(attribute.value());
                        crate::reader_names::python_name_clash(kind, &spelled)?;
                        Some((crate::generator::Language::Python, spelled, None))
                    })
                    .or_else(|| {
                        let (language, spelled) = line_record_member(&node, attribute.name())?;
                        Some((language, spelled, Some(LINE_RECORD_MEMBER)))
                    });
                    if let Some((language, spelled, reason)) = reserved {
                        return Err(Located::new(
                            ValidationError::ReservedCodeIdentifier {
                                element,
                                attr,
                                value,
                                spelled,
                                language: language.canonical_name(),
                                reason,
                            }
                            .into(),
                            doc_name,
                            Some(pos.row),
                            Some(pos.col),
                        ));
                    }
                }
                continue;
            };
            let token = token.to_string();
            let expected = production(grammar);
            let error: ValidationError = match grammar {
                Grammar::EventDescriptors | Grammar::EventName => {
                    ValidationError::EventNameGrammar {
                        element,
                        attr,
                        value,
                        token,
                        expected,
                    }
                }
                Grammar::Id | Grammar::IdRefs => ValidationError::MalformedIdentifier {
                    element,
                    attr,
                    value,
                    token,
                    expected,
                },
                Grammar::CodeIdentifier | Grammar::CodePath => {
                    ValidationError::MalformedCodeIdentifier {
                        element,
                        attr,
                        value,
                        token,
                        expected,
                    }
                }
            };
            return Err(Located::new(
                error.into(),
                doc_name,
                Some(pos.row),
                Some(pos.col),
            ));
        }
    }
    // Every name is well formed and none is reserved; what is left is two
    // that a backend spells as one. After the per-attribute pass, so that a
    // malformed or reserved name keeps its own refusal.
    crate::forge::declared_names::reject_colliding(root, doc_name, dialect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_grammars_differ_only_where_the_two_clauses_do() {
        // A leading digit: legal in §3.12.1's own `DIGIT.0`, illegal in an
        // XML Name. This one case is the whole reason there are two.
        assert!(crate::event_descriptor::is_event_token("0"));
        assert!(!is_ncname_token("0"));
        // Everywhere else they agree, including on the hyphen W3C's own
        // conformance documents 364 and 576 need.
        for token in ["In-s11p112", "s0", "_private", "a-b-c", "x9"] {
            assert!(is_ncname_token(token), "{token}");
            assert!(crate::event_descriptor::is_event_token(token), "{token}");
        }
    }

    #[test]
    fn a_hostile_value_is_refused_by_every_grammar_that_can_hold_one() {
        // The shapes that reach generated code as syntax: the comment
        // terminator, a path separator, a line break, a trailing escape,
        // a quote. Each is refused before it can become an identifier.
        for hostile in ["s0*/X", "a/b", "a\nb", "a\\", "a\"b", "a b", "a'b", ""] {
            assert!(
                offending_token(Grammar::Id, hostile).is_some(),
                "id accepted {hostile:?}"
            );
            assert!(
                offending_token(Grammar::EventName, hostile).is_some(),
                "event name accepted {hostile:?}"
            );
        }
    }

    #[test]
    fn a_list_is_refused_for_the_token_that_offends_and_not_the_whole() {
        assert_eq!(offending_token(Grammar::IdRefs, "good also_good"), None);
        assert_eq!(
            offending_token(Grammar::IdRefs, "good bad*one"),
            Some("bad*one")
        );
        assert_eq!(
            offending_token(Grammar::EventDescriptors, "error.* fine. *"),
            None
        );
        assert_eq!(
            offending_token(Grammar::EventDescriptors, "ok go*/Y"),
            Some("go*/Y")
        );
    }

    /// A forge procedure's `<state id>` is `xs:ID` and still the member of an enum
    /// the backends spell Pascal, so a name that is a keyword there is refused
    /// where the id is written (`false` is the Python member `False`, which cannot
    /// be assigned). A statechart's state is the reader's, escaped and spelled
    /// there, and is not refused here.
    #[test]
    fn a_forge_procedures_state_id_is_as_much_a_name_as_a_data_id() {
        let refuse = |kind: &str, id: &str| -> bool {
            let text = format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          {kind} version="1.0" initial="{id}">
                     <state id="{id}"/>
                   </scxml>"#
            );
            let parsed = roxmltree::Document::parse(&text).expect("a well-formed document");
            let root = parsed.root_element();
            let dialect = Dialect::of(&root);
            reject_malformed(&root, "t", dialect).is_err()
        };
        let procedure = r#"sce:kind="procedure""#;
        assert!(refuse(procedure, "false"), "a state named `false`");
        assert!(refuse(procedure, "none"), "a state named `none`");
        assert!(!refuse(procedure, "running"), "an ordinary state name");
        assert!(
            !refuse("", "false"),
            "a statechart's state is not this rule's"
        );
    }

    /// An entry of a CBOR map or of a content-line component is an attribute of
    /// the generated Python dataclass, as a positional codec's field is, so it is
    /// refused a name that class already uses (`python_name_clash`): a method
    /// every codec class defines, or a name its body evaluates while it is built.
    /// The input frame is no attribute, and a positional codec's `<data>` is
    /// not this rule's.
    #[test]
    fn an_entry_of_a_map_or_of_a_line_is_a_python_class_attribute() {
        let refuse = |encoding: &str, entry: &str| -> bool {
            let text = format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          sce:kind="codec" {encoding} version="1.0">
                     <datamodel>
                       {entry}
                     </datamodel>
                   </scxml>"#
            );
            let parsed = roxmltree::Document::parse(&text).expect("a well-formed document");
            let root = parsed.root_element();
            let dialect = Dialect::of(&root);
            reject_malformed(&root, "t", dialect).is_err()
        };
        let cbor = r#"sce:encoding="cbor""#;
        let line = r#"sce:encoding="content-line""#;
        let entry = |id: &str| format!(r#"<data id="{id}" sce:type="uint8"/>"#);
        for encoding in [cbor, line] {
            for name in [
                "decode",
                "encodeToBytes",
                "_decode_at",
                "classmethod",
                "property",
            ] {
                assert!(
                    refuse(encoding, &entry(name)),
                    "an entry named `{name}` ({encoding})"
                );
            }
            for name in ["list", "staticmethod", "uid"] {
                assert!(
                    !refuse(encoding, &entry(name)),
                    "an entry named `{name}` ({encoding}) is no name the class uses"
                );
            }
            assert!(
                !refuse(
                    encoding,
                    r#"<data id="decode" sce:type="bytes" sce:direction="in"/>"#
                ),
                "the input frame ({encoding}) is no attribute of the class"
            );
        }
        assert!(
            !refuse(
                r#"sce:default-endian="big""#,
                r#"<data id="decode" sce:type="bytes" sce:direction="in"/>"#
            ),
            "a positional codec's input is not this rule's"
        );
    }

    /// A parameter of a repeated content-line property is a member of the
    /// record each of its lines is read into, beside the member every backend
    /// names the line's value by (`line_record_member`): `value`, or `values`
    /// for a list cut at a separator, and in C11 the length and count held
    /// beside it. A parameter spelled as one, in any backend's spelling, is
    /// refused; one of a property that occurs once is a member of the codec's
    /// value instead, and is not.
    #[test]
    fn a_parameter_of_a_repeated_property_is_no_member_its_line_record_declares() {
        let check = |owner: &str, param: &str| -> Result<(), String> {
            let text = format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext"
                          sce:kind="codec" sce:encoding="content-line" version="1.0">
                     <datamodel>
                       <data id="raw" sce:type="bytes" sce:direction="in"/>
                       {owner}
                       <data id="{param}" sce:type="optional&lt;string&gt;" sce:property="ATTENDEE"
                             sce:param="ROLE" sce:max-size="8"/>
                     </datamodel>
                   </scxml>"#
            );
            let parsed = roxmltree::Document::parse(&text).expect("a well-formed document");
            let root = parsed.root_element();
            let dialect = Dialect::of(&root);
            reject_malformed(&root, "t", dialect).map_err(|e| e.error.to_string())
        };
        let text = r#"<data id="who" sce:type="string" sce:property="ATTENDEE"
                            sce:repeated="true" sce:max-count="4" sce:max-size="8"/>"#;
        let listed = r#"<data id="who" sce:type="string" sce:property="ATTENDEE"
                              sce:repeated="true" sce:max-count="4" sce:max-size="8"
                              sce:separator=","/>"#;
        let named = r#"<data id="who" sce:type="enum:role" sce:property="ATTENDEE"
                             sce:repeated="true" sce:max-count="4"/>"#;
        let once = r#"<data id="who" sce:type="string" sce:property="ATTENDEE"
                            sce:max-size="8"/>"#;

        // `Value` and `VALUE` are `value` in Rust's snake_case, `_value` is `Value` in
        // Go's Pascal, and `value_len` is the length C11 holds a text beside.
        for param in ["value", "Value", "VALUE", "_value", "value_len", "valueLen"] {
            let refused = check(text, param)
                .err()
                .unwrap_or_else(|| panic!("a parameter `{param}` of a line of text"));
            assert!(
                refused.contains("declares for the line's value"),
                "the refusal of `{param}` says why: {refused}"
            );
        }
        for param in ["values", "values_len", "values_count", "valuesCount"] {
            assert!(
                check(listed, param).is_err(),
                "a parameter `{param}` of a line of a list of values"
            );
        }
        assert!(
            check(named, "value").is_err(),
            "a parameter `value` of a line of an enum"
        );
        // C11 holds an enum without a length, a list without a single value, and
        // a property that occurs once in the codec's value.
        for (owner, param) in [
            (named, "value_len"),
            (listed, "value"),
            (listed, "value_len"),
            (text, "values"),
            (text, "values_count"),
            (once, "value"),
            (once, "values"),
            (text, "role"),
        ] {
            assert_eq!(
                check(owner, param),
                Ok(()),
                "a parameter `{param}` beside {owner} is no member its line record declares"
            );
        }
    }

    #[test]
    fn the_table_carries_no_attribute_twice() {
        let mut seen: Vec<(&str, &str)> = IDENTIFIER_ATTRIBUTES
            .iter()
            .map(|(tag, attr, _)| (*tag, *attr))
            .collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(before, seen.len(), "a duplicated row would be dead");
    }
}
