// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! W3C SCXML §3.3.1 + §3.12.1 — an identifier-bearing attribute is checked
//! against the grammar W3C gives it, at parse, on the attribute itself.
//!
//! # What this holds that a grep cannot
//!
//! `docs/SCE_ACCEPTED_SUBSET.md` §1 recorded the defect for a day with the
//! sentence *"no parse-time check stands in for it"*, and the closure
//! ledger's row C1 is measured by a gate that greps for a function name and
//! a wire code. Both would go green for a validator nothing calls — which is
//! the failure the accepted-subset section names in its own words. This file
//! is the part that cannot: every assertion below drives a document through
//! `SCXMLParser`, so it fails the moment the sweep stops being reached,
//! whatever the source still spells.
//!
//! # The four things it measures
//!
//! 1. A hostile value is refused, under the code its clause owns, naming
//!    the value and the line the author must open.
//! 2. Every row of `IDENTIFIER_ATTRIBUTES` is REACHABLE. A table is only
//!    coverage if each row can be made to fire; a row for an attribute no
//!    document can carry reads as protection and is none.
//! 3. The spellings W3C itself writes are ACCEPTED — including the two
//!    where the two clauses disagree, which is where a grammar derived from
//!    one of them alone breaks the other.
//! 4. Every standalone document in this tree still parses. The claim "this
//!    grammar refuses nothing here" is re-measured against the tree rather
//!    than carried as a number in a document.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::parser::SCXMLParser;
use sce_build::scxml_identifier::{Grammar, IDENTIFIER_ATTRIBUTES};

/// The wire code a rejection must carry, as JSON, so the test reads the
/// published spelling rather than a Rust variant name.
fn refusal_code(document: &str, label: &str) -> Option<String> {
    let error = SCXMLParser::new().parse_string(document, label).err()?;
    let diagnostics = error.error.to_diagnostics();
    Some(serde_json::to_string(&diagnostics[0].code).unwrap())
}

fn parse_error(
    document: &str,
    label: &str,
) -> sce_build::forge::error::Located<sce_build::forge::error::ForgeError> {
    SCXMLParser::new()
        .parse_string(document, label)
        .err()
        .unwrap_or_else(|| panic!("{label}: the document was accepted and must not be"))
}

/// The value SCE used to turn into `…_STATE_S0*/X` in C and `S0*/X = 1` in
/// Python, taken from the accepted subset's own measurement.
///
/// Nothing targets the bad state on purpose: the sweep reports the FIRST
/// violation in document order, so a document that also referenced it would
/// be answered for by the `target`, and this case would be witnessing the
/// wrong attribute while looking like it witnessed the id.
const HOSTILE_ID: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="ok">
  <state id="ok"/>
  <state id="s0*/X"/>
</scxml>
"#;

const HOSTILE_EVENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="ok">
  <state id="ok">
    <transition event="go*/Y" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// The document `docs/SCE_ACCEPTED_SUBSET.md` §1 measured generating clean
/// on 2026-09-13, quoted whole. Its own words are the assertion.
const THE_MEASURED_DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="s0*/X">
  <state id="s0*/X">
    <transition event="go*/Y" target="done*/Z"/>
  </state>
  <final id="done*/Z"/>
</scxml>
"#;

#[test]
fn a_hostile_id_is_refused_at_parse_under_its_own_clause() {
    assert_eq!(
        refusal_code(HOSTILE_ID, "hostile-id").as_deref(),
        Some("\"validation/malformed-identifier\""),
        "an id W3C types xs:ID and this document spells `s0*/X` reached the model"
    );
    assert_eq!(
        refusal_code(HOSTILE_EVENT, "hostile-event").as_deref(),
        Some("\"validation/event-name-grammar\""),
        "an event descriptor spelled `go*/Y` reached the model"
    );
    assert!(
        refusal_code(THE_MEASURED_DOCUMENT, "accepted-subset-1").is_some(),
        "the document §1 of the accepted subset measured generating without \
         a diagnostic still generates without one"
    );
}

#[test]
fn the_refusal_names_the_value_and_the_line_holding_it() {
    // SCE_ERROR_CONTRACT.md §3.1.1: when a record carries `location.line`,
    // the value in `actual` occurs on that line. The attribute's own
    // position is what makes that true — the element's would not, for an
    // attribute written on a continuation line.
    let error = parse_error(HOSTILE_ID, "hostile-id");
    let diagnostic = &error.error.to_diagnostics()[0];
    assert_eq!(
        diagnostic.actual.as_deref(),
        Some("s0*/X"),
        "the record must carry the rejected value for the consumer to locate"
    );

    let line = error
        .location
        .line
        .expect("a parse-time refusal must carry a line");
    let text = HOSTILE_ID
        .lines()
        .nth(line as usize - 1)
        .unwrap_or_else(|| panic!("line {line} is past the end of the document"));
    assert!(
        text.contains("s0*/X"),
        "line {line} is {text:?}, which does not hold the value the record names"
    );
    assert!(
        text.contains("<state id="),
        "the reported line is not the declaration the record is about: {text:?}"
    );
    // The column must land inside the value, not on the element. Nothing
    // else in the record tells a consumer which of several attributes on
    // one line it means.
    let column = error
        .location
        .col
        .expect("a parse-time refusal must carry a column") as usize;
    assert_eq!(
        text.get(column - 1..column - 1 + "s0*/X".len()),
        Some("s0*/X"),
        "column {column} of {text:?} is not where the rejected value starts"
    );
}

/// A row of the table that no document can trigger is dead weight that
/// reads as coverage. Each row is fed a hostile value in a document that
/// carries it, and must be refused.
#[test]
fn every_row_of_the_table_is_reachable_from_a_document() {
    // The hostile token is the same everywhere so the row, not the value,
    // is what varies. `*` and `/` are refused by both grammars.
    const BAD: &str = "x*/y";

    let mut unreachable: Vec<String> = Vec::new();
    for &(element, attr, grammar) in IDENTIFIER_ATTRIBUTES {
        let document = document_carrying(element, attr, BAD);
        let label = format!("{element}@{attr}");
        let expected = match grammar {
            Grammar::Id | Grammar::IdRefs => "\"validation/malformed-identifier\"",
            Grammar::EventDescriptors | Grammar::EventName => "\"validation/event-name-grammar\"",
        };
        match refusal_code(&document, &label) {
            Some(code) if code == expected => {}
            Some(code) => unreachable.push(format!(
                "  <{element} {attr}> was refused as {code}, not {expected}"
            )),
            None => unreachable.push(format!(
                "  <{element} {attr}=\"{BAD}\"> was ACCEPTED — the row protects nothing"
            )),
        }
    }
    assert!(
        unreachable.is_empty(),
        "rows of scxml_identifier::IDENTIFIER_ATTRIBUTES that do not hold:\n{}",
        unreachable.join("\n"),
    );
    assert!(
        IDENTIFIER_ATTRIBUTES.len() >= 13,
        "the table shrank to {} rows; every W3C attribute typed ID, IDREF(S) \
         or event belongs in it, and dropping one drops the check with it",
        IDENTIFIER_ATTRIBUTES.len(),
    );
}

/// One well-formed document carrying `element`'s `attr` set to `value`.
///
/// Written per element rather than by templating one shape, because the
/// elements sit in different places: `<data>` needs a `<datamodel>`,
/// `<raise>` needs executable content, `<history>` needs a compound parent.
fn document_carrying(element: &str, attr: &str, value: &str) -> String {
    let body = match (element, attr) {
        ("scxml", "initial") => {
            return format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" initial="{value}"><final id="ok"/></scxml>"#
            )
        }
        ("state", "id") => r#"<state id="{V}"/><final id="ok"/>"#.to_string(),
        ("state", "initial") => {
            r#"<state id="outer" initial="{V}"><state id="inner"/></state>"#.to_string()
        }
        ("parallel", "id") => r#"<parallel id="{V}"><state id="a"/></parallel>"#.to_string(),
        ("final", "id") => r#"<final id="{V}"/>"#.to_string(),
        ("history", "id") => r#"<state id="outer"><history id="{V}">
        <transition target="inner"/></history><state id="inner"/></state>"#
            .to_string(),
        ("data", "id") => r#"<datamodel><data id="{V}"/></datamodel><final id="ok"/>"#.to_string(),
        ("invoke", "id") => {
            r#"<state id="s"><invoke id="{V}" type="scxml" src="x.scxml"/></state>"#.to_string()
        }
        ("send", "id") => {
            r#"<state id="s"><onentry><send id="{V}" event="go"/></onentry></state>"#.to_string()
        }
        ("send", "event") => {
            r#"<state id="s"><onentry><send event="{V}"/></onentry></state>"#.to_string()
        }
        ("raise", "event") => {
            r#"<state id="s"><onentry><raise event="{V}"/></onentry></state>"#.to_string()
        }
        ("transition", "event") => {
            r#"<state id="s"><transition event="{V}" target="ok"/></state><final id="ok"/>"#
                .to_string()
        }
        ("transition", "target") => {
            r#"<state id="s"><transition event="go" target="{V}"/></state>"#.to_string()
        }
        other => panic!(
            "{other:?} is a row of IDENTIFIER_ATTRIBUTES with no document here. \
             Adding a row means adding the document that reaches it — that is \
             what makes the row coverage rather than a claim."
        ),
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null">
  {}
</scxml>
"#,
        body.replace("{V}", value)
    )
}

/// The spellings W3C's own text and its own conformance suite write.
///
/// Rows 1 and 2 are the pair that makes this two grammars and not one:
/// reading §3.12.1's "alphanumeric" literally rejects the first, and
/// reading it as an XML Name rejects the second. A repair derived from
/// either clause alone fails the other row.
const W3C_SPELLINGS: &[(&str, &str, &str)] = &[
    // W3C conformance documents 364 and 576 both raise this event.
    ("event", "In-s11p112", "conformance documents 364 and 576"),
    // §3.12.1's calculator example: `<transition event="DIGIT.0">`.
    ("event", "DIGIT.0", "the §3.12.1 calculator example"),
    // The three spellings §3.12.1 calls "functionally equivalent", plus
    // the two wildcards it defines.
    ("event", "error", "§3.12.1 equivalent spelling"),
    ("event", "error.", "§3.12.1 equivalent spelling"),
    ("event", "error.*", "§3.12.1 equivalent spelling"),
    ("event", "*", "§3.12.1 bare wildcard"),
    (
        "event",
        ".*",
        "the empty token prefix W3C tests 311-314 rely on",
    ),
    ("event", "done.state.s1", "an ordinary platform event"),
    ("event", "error.send fail.one", "a two-descriptor attribute"),
    // Names an XML Schema ID admits.
    ("id", "s0", "an ordinary state id"),
    (
        "id",
        "_private",
        "a leading underscore, legal in an XML Name",
    ),
    ("id", "In-s11p112", "a hyphen, legal in an XML Name"),
    ("id", "a.b.c", "dots, which XML Names admit"),
];

#[test]
fn the_spellings_w3c_writes_are_accepted() {
    let mut refused: Vec<String> = Vec::new();
    for (position, value, source) in W3C_SPELLINGS {
        let document = match *position {
            "event" => format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="s">
  <state id="s"><transition event="{value}" target="ok"/></state>
  <final id="ok"/>
</scxml>
"#
            ),
            _ => format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="{value}">
  <state id="{value}"><transition event="go" target="ok"/></state>
  <final id="ok"/>
</scxml>
"#
            ),
        };
        if let Some(code) = refusal_code(&document, value) {
            refused.push(format!(
                "  {position}={value:?} ({source}) refused as {code}"
            ));
        }
    }
    assert!(
        refused.is_empty(),
        "W3C writes these and the grammar must not refuse them:\n{}",
        refused.join("\n"),
    );
    assert!(
        W3C_SPELLINGS.len() >= 13,
        "the accepted-spelling table shrank to {}; a shrinking table is how \
         an over-strict grammar stops being measured",
        W3C_SPELLINGS.len(),
    );
}

/// Every standalone SCXML document in this tree still parses past the sweep.
///
/// The ledger row records "refuses 0 of the ids and event descriptors in
/// this tree". That is a measurement, and a measurement written into a
/// document goes stale the day after; here the tree re-takes it.
///
/// ⚠ Template BODIES are excluded, and the exclusion is DERIVED rather than
/// listed: a body is any file some other document names in `<sce:use
/// template>` or `<xi:include href>`. Those four files carry `{$id}`
/// placeholders, are only ever parsed after expansion replaces them, and a
/// hand-written list of them would rot the first time one moved.
#[test]
fn no_document_in_this_tree_is_refused_by_the_grammar() {
    let root = repo_root();
    let documents = scxml_documents(&root);
    assert!(
        documents.len() > 600,
        "found only {} SCXML documents under {}; an empty sweep passes for \
         the wrong reason",
        documents.len(),
        root.display(),
    );

    let fragments = referenced_fragments(&documents);
    assert!(
        !fragments.is_empty(),
        "no <sce:use template> or <xi:include href> target was found, so the \
         exclusion below is not deriving anything and would hide a real \
         refusal in a template body"
    );

    let mut refused: Vec<String> = Vec::new();
    let mut examined = 0usize;
    for path in &documents {
        if fragments.contains(path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(document) = roxmltree::Document::parse(&text) else {
            continue; // not well-formed XML; a different stage's complaint
        };
        examined += 1;
        let relative = path.strip_prefix(&root).unwrap_or(path);
        if let Err(error) = sce_build::scxml_identifier::reject_malformed(
            &document.root_element(),
            &relative.display().to_string(),
        ) {
            refused.push(format!("  {}: {}", relative.display(), error.error));
        }
    }

    assert!(
        examined > 600,
        "only {examined} documents survived to be examined; the sweep must \
         see the corpus, not a handful of it"
    );
    assert!(
        refused.is_empty(),
        "the grammar refuses {} document(s) this tree commits:\n{}",
        refused.len(),
        refused.join("\n"),
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits under the repository root")
        .to_path_buf()
}

/// Every `.scxml` document this repository COMMITS.
///
/// `git ls-files` is the enumeration source, the way `roadmap_marker_gate`
/// and the other tree-wide targets here already do it, and the reason is the
/// same: what a build writes and what a developer leaves lying around are not
/// this tree's documents, and a gate that reads them answers a question about
/// one machine.
///
/// ⚠ It replaced a hand-written directory skip list — `build`, `target`,
/// `node_modules`, `dist`, `.git`, `wasm` — which is the shape this
/// repository has been bitten by before. Measured 2026-09-14 the list was
/// already wrong by 51 documents: it catches a directory literally named
/// `build` and misses every other spelling of build output, so 47 synthesized
/// `<invoke>` documents under `backends/go/tests/generated/`, 2 under
/// `backends/python/tests/generated/` and 2 under a gitignored working-docs
/// directory were being judged as though the repository had authored them —
/// 786 walked against 735 committed.
///
/// That is not a tidiness point. Those documents are the GENERATOR'S OWN
/// OUTPUT, so the sweep was asking whether SCE's codegen emits identifiers
/// SCE accepts — a different question from the one this case states, and one
/// whose answer no other checkout could reproduce. A hostile id emitted into
/// a synthesized document would have reddened this case on a developer's
/// machine and stayed green in CI.
fn scxml_documents(root: &Path) -> Vec<PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "*.scxml"])
        .output()
        .expect("git ls-files runs");
    assert!(
        out.status.success(),
        "git ls-files must succeed; without it this case cannot say which \
         documents the tree has and must not guess"
    );
    let mut documents: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| root.join(p))
        .collect();
    documents.sort();
    documents
}

/// The files other documents pull in — template bodies and XInclude
/// fragments. They are parsed only after expansion, so a placeholder in one
/// is not an identifier yet.
fn referenced_fragments(documents: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    for path in documents {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(document) = roxmltree::Document::parse(&text) else {
            continue;
        };
        let Some(dir) = path.parent() else { continue };
        for node in document.descendants().filter(|n| n.is_element()) {
            let reference = match node.tag_name().name() {
                "use" => node.attribute("template"),
                "include" => node.attribute("href"),
                _ => None,
            };
            if let Some(reference) = reference {
                out.insert(normalise(&dir.join(reference)));
            }
        }
    }
    out
}

/// `a/b/../c` → `a/c`, so a reference and a walked path compare equal.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}
