// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! No method of a generated Kotlin machine grows with its document.
//!
//! The code of one JVM method is at most 65,535 bytes (JVM Specification,
//! section 4.7.3), and kotlinc refuses a method past it with
//! `MethodTooLargeException`. A method the generator writes one arm or one
//! statement into for every state, transition, history, invoke, event or
//! variable of the document therefore makes some document fail to compile,
//! however correct it is.
//!
//! # What was wrong
//!
//! Measured 2026-11-05: a large `datamodel="sce-static"` statechart generated
//! for Kotlin did not compile, kotlinc refusing
//! `executeTransitionContent(state, index)`, the one method that then held the
//! content of every transition of the document. The selection, the entry and
//! exit dispatches, the companion object's class initialiser and the
//! per-variable sequences had the same shape.
//!
//! Each element's code is now a method of its own, and every dispatch, table
//! and sequence is split into parts of at most
//! [`sce_build::kotlin::METHOD_ITEMS`] items
//! (`tools/codegen/templates/kotlin/dispatch.kt.jinja2`).
//!
//! # The proxy, and why it is conservative
//!
//! No JVM runs here, so the size of a method is read from the Kotlin text: the
//! statements of each method, and of each object's initialiser (an object's
//! properties, less the functions it declares). A statement is a line of code
//! that begins outside any open parenthesis and is not a comment, an
//! annotation, a closing brace or one of the clauses that compile to almost
//! nothing (`try {`, `} catch …`, `} finally {`, `} else {`); a construction
//! spread over several lines is one statement.
//!
//! The documents below are written so that each of their statements is
//! short: an arm of a dispatch, the constant of one transition with one
//! target, an integer field updated by a checked addition, a variable's
//! initialisation. Each compiles to fewer than [`STATEMENT_CODE_BYTES`], so a
//! method of at most [`MAX_STATEMENTS`] of them is at most half the limit. The
//! proxy would not hold for an element whose one statement is itself huge — a
//! text, a guard — and that is not what a document grows by: it is the size of
//! what one element writes, which no split of the machine changes.
//!
//! # What this does not hold
//!
//! - The class's own property initialisers (its constructor): each variable's
//!   field is declared with its value, and is not split.
//! - A file other than the machine's (a transition table or function emitted
//!   on request).

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::kotlin::{JVM_METHOD_CODE_BYTES, METHOD_ITEMS};
use tempfile::tempdir;

/// The statements one generated method may hold here.
///
/// A part of a dispatch holds [`METHOD_ITEMS`] arms and the few lines around
/// them; the widest sequence part of the documents below holds three
/// statements for each of its variables.
const MAX_STATEMENTS: usize = 2 * METHOD_ITEMS;

/// What one statement of the documents below compiles to at most: the widest
/// is the constant of a transition (`new`, the source, a one-target list, the
/// index and two flags, `invokespecial`, `putstatic`: about 40 bytes).
const STATEMENT_CODE_BYTES: usize = 64;

/// What one counted statement of the transition content below compiles to at
/// least, on average: a field updated by an addition is `aload_0`, a load of
/// the field, the constant, the addition and `putfield` (11 bytes); checked,
/// it is that inside a `try` whose failure ends the block, two statements of
/// more than 22 bytes together.
const CONTENT_STATEMENT_MIN_BYTES: usize = 8;

// A method of at most MAX_STATEMENTS statements is at most half the limit.
const _: () = assert!(MAX_STATEMENTS * STATEMENT_CODE_BYTES < JVM_METHOD_CODE_BYTES);

/// States of the `sce-static` document: past two parts of every dispatch over
/// its states.
const STATIC_STATES: usize = 2 * METHOD_ITEMS + 88;

/// Assignments in the content of each transition of the `sce-static` document.
const ASSIGNMENTS: usize = 14;

/// States and variables of the ECMAScript document.
const SCRIPT_STATES: usize = METHOD_ITEMS + 44;
const SCRIPT_VARIABLES: usize = 150;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A `sce-static` statechart of [`STATIC_STATES`] states in one compound
/// state with a deep history, each entering and leaving with an assignment
/// and owning a transition whose content is [`ASSIGNMENTS`] checked
/// additions.
fn static_document() -> String {
    let mut doc = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\"\n\
         \x20      version=\"1.0\" initial=\"all\" datamodel=\"sce-static\" name=\"wide_static\">\n\
         \x20 <datamodel>\n\
         \x20   <data id=\"steps\" sce:type=\"uint32\" expr=\"0\" sce:direction=\"out\"/>\n\
         \x20   <data id=\"entered\" sce:type=\"uint32\" expr=\"0\"/>\n\
         \x20   <data id=\"left\" sce:type=\"uint32\" expr=\"0\"/>\n",
    );
    for a in 0..ASSIGNMENTS - 1 {
        doc.push_str(&format!(
            "    <data id=\"a{a}\" sce:type=\"uint32\" expr=\"0\"/>\n"
        ));
    }
    doc.push_str(
        "  </datamodel>\n\
         \x20 <state id=\"all\" initial=\"s0\">\n\
         \x20   <history id=\"back\" type=\"deep\">\n\
         \x20     <transition target=\"s0\"/>\n\
         \x20   </history>\n",
    );
    for i in 0..STATIC_STATES {
        let next = (i + 1) % STATIC_STATES;
        doc.push_str(&format!("    <state id=\"s{i}\">\n"));
        doc.push_str(
            "      <onentry><assign location=\"entered\" expr=\"entered + 1\"/></onentry>\n",
        );
        doc.push_str("      <onexit><assign location=\"left\" expr=\"left + 1\"/></onexit>\n");
        doc.push_str(&format!(
            "      <transition event=\"next\" target=\"s{next}\">\n"
        ));
        doc.push_str("        <assign location=\"steps\" expr=\"steps + 1\"/>\n");
        for a in 0..ASSIGNMENTS - 1 {
            let step = (i + a) % 7 + 1;
            doc.push_str(&format!(
                "        <assign location=\"a{a}\" expr=\"a{a} + {step}\"/>\n"
            ));
        }
        doc.push_str("      </transition>\n    </state>\n");
    }
    doc.push_str(
        "    <transition event=\"pause\" target=\"paused\"/>\n\
         \x20 </state>\n\
         \x20 <state id=\"paused\">\n\
         \x20   <transition event=\"resume\" target=\"back\"/>\n\
         \x20 </state>\n\
         </scxml>\n",
    );
    doc
}

/// An ECMAScript statechart of [`SCRIPT_STATES`] states over
/// [`SCRIPT_VARIABLES`] variables, each state's transition guarded by the
/// engine and carrying an assignment.
fn script_document() -> String {
    let mut doc = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\"\n\
         \x20      initial=\"s0\" datamodel=\"ecmascript\" name=\"wide_script\">\n\
         \x20 <datamodel>\n",
    );
    for v in 0..SCRIPT_VARIABLES {
        doc.push_str(&format!("    <data id=\"x{v}\" expr=\"{v}\"/>\n"));
    }
    doc.push_str("  </datamodel>\n");
    for i in 0..SCRIPT_STATES {
        let next = (i + 1) % SCRIPT_STATES;
        let v = i % SCRIPT_VARIABLES;
        let w = (i + 1) % SCRIPT_VARIABLES;
        doc.push_str(&format!(
            "  <state id=\"s{i}\">\n\
             \x20   <onentry><assign location=\"x{v}\" expr=\"x{v} + 1\"/></onentry>\n\
             \x20   <transition event=\"next\" cond=\"x{v} &gt;= 0\" target=\"s{next}\">\n\
             \x20     <assign location=\"x{w}\" expr=\"x{w} + 1\"/>\n\
             \x20   </transition>\n\
             \x20 </state>\n"
        ));
    }
    doc.push_str("</scxml>\n");
    doc
}

/// `sce-codegen generate -l kotlin` of `document`: the text of every `.kt`
/// file it wrote.
fn generate_kotlin(name: &str, document: &str) -> Vec<String> {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join(format!("{name}.scxml"));
    std::fs::write(&input, document).expect("write the document");
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "kotlin"])
        .arg(&input)
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "Kotlin generates `{name}`:\n{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let mut written = Vec::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some("kt") {
            written.push(std::fs::read_to_string(&path).expect("a generated file"));
        }
    }
    assert!(!written.is_empty(), "`{name}` generated no Kotlin file");
    written
}

/// One method kotlinc compiles from the text: a function, or the initialiser
/// of an object.
#[derive(Debug)]
struct Unit {
    name: String,
    statements: usize,
}

/// A unit whose body is still being read.
struct Open {
    name: String,
    indent: usize,
    statements: usize,
    /// Parentheses open at the end of the last line read.
    depth: i64,
}

/// What `code` declares, when it opens a unit: the function's or the object's
/// name. Annotations and modifiers before the keyword are skipped.
fn opens_unit(code: &str) -> Option<String> {
    let mut rest = code;
    loop {
        if let Some(after) = rest.strip_prefix('@') {
            let end = match after.find('(') {
                Some(open) if !after[..open].contains(' ') => {
                    after[open..].find(')').map(|close| open + close + 1)
                }
                _ => after.find(' '),
            }?;
            rest = after[end..].trim_start();
            continue;
        }
        let modifier = [
            "private ",
            "override ",
            "protected ",
            "internal ",
            "public ",
            "inline ",
            "operator ",
            "suspend ",
        ]
        .into_iter()
        .find(|m| rest.starts_with(m));
        match modifier {
            Some(m) => rest = &rest[m.len()..],
            None => break,
        }
    }
    if let Some(after) = rest.strip_prefix("fun ") {
        let end = after.find(['(', '<']).unwrap_or(after.len());
        return Some(after[..end].to_string());
    }
    if rest.starts_with("companion object") {
        return Some("Companion".to_string());
    }
    if let Some(after) = rest.strip_prefix("object ") {
        let end = after.find([' ', '{', ':']).unwrap_or(after.len());
        return Some(after[..end].to_string());
    }
    None
}

/// Whether `code` begins a statement the proxy counts.
fn is_statement(code: &str) -> bool {
    if code.starts_with("//") || code.starts_with("/*") || code.starts_with('*') {
        return false;
    }
    if code.starts_with('@') && !code.contains(' ') {
        return false;
    }
    if code
        .chars()
        .all(|c| matches!(c, '}' | ')' | ']' | ',' | ' '))
    {
        return false;
    }
    let cheap_clause = code == "try {"
        || code.starts_with("} catch")
        || code == "} finally {"
        || code == "} else {";
    !cheap_clause
}

/// The parentheses `code` opens less those it closes, outside its string and
/// character literals and before a line comment.
fn paren_balance(code: &str) -> i64 {
    let mut balance = 0i64;
    let mut chars = code.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '\'' => {
                if chars.next() == Some('\\') {
                    chars.next();
                }
                chars.next();
            }
            '/' if chars.peek() == Some(&'/') => break,
            '(' => balance += 1,
            ')' => balance -= 1,
            _ => {}
        }
    }
    balance
}

/// Every unit of `kotlin` with the statements it holds.
///
/// A unit opens at a line declaring a function or an object whose body follows
/// (the line ends in `{` or `=`), and closes at the first later line indented
/// no deeper than it, a closing brace at its own indent being its last line.
/// A function written on one line is one statement.
fn units(kotlin: &str) -> Vec<Unit> {
    let mut stack: Vec<Open> = Vec::new();
    let mut done: Vec<Unit> = Vec::new();
    'lines: for line in kotlin.lines() {
        let code = line.trim_start();
        if code.is_empty() {
            continue;
        }
        let indent = line.len() - code.len();
        while let Some(top) = stack.last() {
            if indent > top.indent || (indent == top.indent && code.starts_with(')')) {
                break;
            }
            let closes_it = indent == top.indent && code.starts_with('}');
            let top = stack.pop().expect("an open unit");
            done.push(Unit {
                name: top.name,
                statements: top.statements,
            });
            if closes_it {
                continue 'lines;
            }
        }
        if let Some(name) = opens_unit(code) {
            if code.ends_with('{') || code.ends_with('=') || code.ends_with('(') {
                stack.push(Open {
                    name,
                    indent,
                    statements: 0,
                    depth: 0,
                });
            } else {
                done.push(Unit {
                    name,
                    statements: 1,
                });
            }
            continue;
        }
        if let Some(top) = stack.last_mut() {
            if top.depth == 0 && is_statement(code) {
                top.statements += 1;
            }
            top.depth = (top.depth + paren_balance(code)).max(0);
        }
    }
    while let Some(top) = stack.pop() {
        done.push(Unit {
            name: top.name,
            statements: top.statements,
        });
    }
    done
}

/// Every unit of the files, failing on any past [`MAX_STATEMENTS`].
fn units_within_the_bound(name: &str, files: &[String]) -> Vec<Unit> {
    let mut all = Vec::new();
    for text in files {
        all.extend(units(text));
    }
    let over: Vec<String> = all
        .iter()
        .filter(|unit| unit.statements > MAX_STATEMENTS)
        .map(|unit| format!("{} ({} statements)", unit.name, unit.statements))
        .collect();
    assert!(
        over.is_empty(),
        "`{name}`: a generated method holds more than {MAX_STATEMENTS} statements, so it \
         grows with the document and a larger one passes the 65,535 bytes of one JVM \
         method: {}",
        over.join(", ")
    );
    all
}

/// Whether a unit called `name` is among `units`.
fn has_unit(units: &[Unit], name: &str) -> bool {
    units.iter().any(|unit| unit.name == name)
}

/// The `sce-static` document whose transition content alone, written into one
/// method as it was, passed the limit: every method of its machine stays
/// within the bound, and every dispatch over its states is split.
#[test]
fn a_wide_static_machine_keeps_every_method_within_the_bound() {
    let files = generate_kotlin("wide_static", &static_document());
    let units = units_within_the_bound("wide_static", &files);

    // The content of the transitions, which `executeTransitionContent` held
    // whole: past the limit when it is one method. Each is
    // `transitionContentOf<State>At<index>`, beside its state's dispatch
    // `transitionContentOf<State>`.
    let is_content =
        |unit: &&Unit| unit.name.starts_with("transitionContentOfS") && unit.name.contains("At");
    let content: usize = units
        .iter()
        .filter(is_content)
        .map(|unit| unit.statements)
        .sum();
    assert!(
        content * CONTENT_STATEMENT_MIN_BYTES > JVM_METHOD_CODE_BYTES,
        "the document must be large enough that its transition content in one method \
         passes the limit ({content} statements of at least \
         {CONTENT_STATEMENT_MIN_BYTES} bytes); otherwise this test holds nothing"
    );
    assert_eq!(
        units.iter().filter(is_content).count(),
        STATIC_STATES,
        "each transition's content is a method of its own"
    );

    for split in [
        "onEntryPart1",
        "onExitPart1",
        "executeTransitionContentPart1",
        "firstEnabledTransitionPart1",
        "parentOfPart1",
        "resolveStatePart1",
        "stateIdOfPart1",
        "documentOrderOfPart1",
        "TransitionsPart1",
    ] {
        assert!(
            has_unit(&units, split),
            "`{split}` is written: the dispatch over {STATIC_STATES} states is split \
             into parts of at most {METHOD_ITEMS} arms"
        );
    }
    for element in [
        "onEntryOfS0",
        "onExitOfS0",
        "firstEnabledTransitionOfS0",
        "transitionContentOfS0",
        "transitionContentOfS0At0",
    ] {
        assert!(
            has_unit(&units, element),
            "`{element}` is written: a state's code is a method of its own"
        );
    }
}

/// The ECMAScript document: the selection asks the engine for each guard, and
/// the variables are declared by the engine when the session starts.
#[test]
fn a_wide_script_machine_keeps_every_method_within_the_bound() {
    let files = generate_kotlin("wide_script", &script_document());
    let units = units_within_the_bound("wide_script", &files);
    for split in [
        "onEntryPart1",
        "firstEnabledTransitionPart1",
        "executeTransitionContentPart1",
        "declareVariablesPart0",
    ] {
        assert!(
            has_unit(&units, split),
            "`{split}` is written for a machine of {SCRIPT_STATES} states"
        );
    }
}

/// The reading of the text this test rests on: a unit opens and closes where
/// its function or object does, a construction over several lines is one
/// statement, and a dispatch of n arms is n statements and its `when`.
#[test]
fn the_proxy_reads_functions_objects_and_statements() {
    let text = "\
class M {
    override fun onExit(state: S) {
        when (state) {
            is S.A -> exitOfA()
            is S.B -> exitOfB()
        }
    }

    private fun exitOfA() {
        // a marker
        try {
            vX = vX + 1
        } catch (_: Exception) {
            fail(\"(\")
        }
    }

    fun save(): Saved = save(0)

    private companion object {
        val history: HistoryId get() = HistoryId(0)

        private fun tablePart0(table: MutableMap<S, Int>) {
            table[S.A] = 1
        }
    }

    private object TransitionsPart0 {
        val transitionAAt0 = EnabledTransition<S, HistoryId>(
            S.A,
            listOf(
                StateTarget(S.B),
            ),
            0,
        )
    }
}
";
    let found: Vec<(String, usize)> = units(text)
        .into_iter()
        .map(|unit| (unit.name, unit.statements))
        .collect();
    assert_eq!(
        found,
        [
            ("onExit".to_string(), 3),
            ("exitOfA".to_string(), 2),
            ("save".to_string(), 1),
            ("tablePart0".to_string(), 1),
            ("Companion".to_string(), 1),
            ("TransitionsPart0".to_string(), 1),
        ]
    );
}
