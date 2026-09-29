// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart's names as members of its generated code
//! (`docs/SCE_ACCEPTED_SUBSET.md` §2.14.1, `sce_build::member_names`).
//!
//! Two halves, and the second is what makes the first trustworthy:
//!
//! - **The refusal.** Each shape below passed `check` and `generate` with
//!   exit status 0 before the rule existed, and generated code that some
//!   backend's compiler rejects. Each is now refused for every backend at
//!   once, naming exactly the backends that fold it.
//! - **The spelling is the templates'.** `member_names::declared` is what the
//!   refusal reasons about; the second half renders a document through all
//!   six backends and reads the members each one actually declared back out
//!   of the source. A template that spells a member another way, or adds a
//!   sentinel the module does not list, fails here.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use tempfile::tempdir;

use sce_build::forge::error::ForgeError;
use sce_build::generator::Language;
use sce_build::member_names::{Clash, Enumeration};
use sce_build::scxml_semantic::ScxmlSemanticError;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn document(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" name=\"names\">\n\
         {body}\n\
         </scxml>\n"
    )
}

fn write(dir: &Path, text: &str) -> PathBuf {
    let path = dir.join("names.scxml");
    fs::write(&path, text).expect("write fixture");
    path
}

/// The refusal, asked of every backend: it must be the same one each time,
/// since whether SCE accepts a document must not depend on which backend a
/// deployment builds.
fn refusal(body: &str) -> (Enumeration, String, Clash, Vec<&'static str>) {
    let dir = tempdir().expect("tempdir");
    let path = write(dir.path(), &document(body));
    let mut seen = None;
    for &language in Language::ALL {
        let err = match compile_scxml_lang_typed(
            path.to_str().unwrap(),
            &find_template_dir_for(language),
            language,
        ) {
            Ok(_) => panic!("{language:?} accepted a document whose names collide:\n{body}"),
            Err(e) => e,
        };
        let ForgeError::Scxml(semantic) = &err.error else {
            panic!("{language:?}: expected the collision, got {:?}", err.error);
        };
        let ScxmlSemanticError::GeneratedNameCollision {
            enumeration,
            name,
            clash,
            spellings,
        } = semantic.as_ref()
        else {
            panic!("{language:?}: expected GeneratedNameCollision, got {semantic:?}");
        };
        let this = (
            *enumeration,
            name.clone(),
            clash.clone(),
            spellings.iter().map(|(l, _)| *l).collect::<Vec<_>>(),
        );
        if let Some(first) = &seen {
            assert_eq!(first, &this, "{language:?} refused differently");
        }
        seen = Some(this);
    }
    seen.expect("six backends")
}

const EVERY_BACKEND: [&str; 6] = ["rust", "cpp", "kotlin", "go", "python", "c11"];

#[test]
fn two_states_that_differ_only_in_case_are_refused_for_every_backend() {
    // XML Names are case-sensitive, so these are two conforming states
    // (W3C SCXML 3.14); every backend spells both `Idle` or `IDLE`.
    let (enumeration, name, clash, backends) = refusal(
        r#"<state id="idle"><transition event="go" target="Idle"/></state>
           <state id="Idle"><transition event="go" target="idle"/></state>"#,
    );
    assert_eq!(enumeration, Enumeration::State);
    assert_eq!((name.as_str(), clash), ("idle", Clash::Name("Idle".into())));
    assert_eq!(backends, EVERY_BACKEND);
}

#[test]
fn camel_and_snake_case_states_are_refused_where_the_backend_folds_them() {
    // C++ keeps `DoorOpen` and `Door_open` apart and C11 `DOOROPEN` and
    // `DOOR_OPEN`; the four PascalCase and UPPER_SNAKE backends do not.
    let (_, name, clash, backends) = refusal(
        r#"<state id="doorOpen"><transition event="go" target="door_open"/></state>
           <state id="door_open"><transition event="go" target="doorOpen"/></state>"#,
    );
    assert_eq!(
        (name.as_str(), clash),
        ("door_open", Clash::Name("doorOpen".into()))
    );
    assert_eq!(backends, ["rust", "kotlin", "go", "python"]);
}

#[test]
fn a_dash_and_an_underscore_in_a_state_are_one_member_everywhere() {
    // `-` is written as a delimiter by every backend, so `door-open` and
    // `door_open` meet in all six.
    let (_, name, clash, backends) = refusal(
        r#"<state id="door-open"><transition event="go" target="door_open"/></state>
           <state id="door_open"><transition event="go" target="door-open"/></state>"#,
    );
    assert_eq!(
        (name.as_str(), clash),
        ("door_open", Clash::Name("door-open".into()))
    );
    assert_eq!(backends, EVERY_BACKEND);
}

#[test]
fn a_dotted_and_an_underscored_event_are_refused_where_they_fold() {
    // Kotlin keeps them apart: `door.open` is the nested `Door.Open`,
    // `door_open` the top-level `DoorOpen`.
    let (enumeration, name, clash, backends) = refusal(
        r#"<state id="a">
             <transition event="door.open" target="b"/>
             <transition event="door_open" target="b"/>
           </state>
           <final id="b"/>"#,
    );
    assert_eq!(enumeration, Enumeration::Event);
    assert_eq!(
        (name.as_str(), clash),
        ("door_open", Clash::Name("door.open".into()))
    );
    assert_eq!(backends, ["rust", "cpp", "go", "python", "c11"]);
}

#[test]
fn an_event_spelled_as_the_member_cpp_and_c11_reserve_is_refused() {
    // `Event::NONE` in C++ and `<M>_EVENT_NONE` in C11 are declared by the
    // templates for eventless dispatch.
    let (_, name, clash, backends) =
        refusal(r#"<state id="a"><transition event="NONE" target="b"/></state><final id="b"/>"#);
    assert_eq!((name.as_str(), clash), ("NONE", Clash::Generated));
    assert_eq!(backends, ["cpp", "c11"]);
}

#[test]
fn an_event_spelled_as_the_eventless_null_is_refused_where_it_is_declared() {
    // Rust and Go declare `Null`; Python spells the event `NULL_` to avoid
    // its own sentinel, and C++ and C11 reserve a different word.
    let (_, name, clash, backends) =
        refusal(r#"<state id="a"><transition event="null" target="b"/></state><final id="b"/>"#);
    assert_eq!((name.as_str(), clash), ("null", Clash::Generated));
    assert_eq!(backends, ["rust", "go"]);
}

#[test]
fn a_state_spelled_as_the_c11_state_count_is_refused() {
    let (enumeration, name, clash, backends) =
        refusal(r#"<state id="count"><transition event="go" target="b"/></state><final id="b"/>"#);
    assert_eq!(enumeration, Enumeration::State);
    assert_eq!((name.as_str(), clash), ("count", Clash::Generated));
    assert_eq!(backends, ["c11"]);
}

#[test]
fn a_history_spelled_as_the_c11_empty_history_is_refused() {
    let (enumeration, name, clash, backends) = refusal(
        r#"<state id="p" initial="a">
             <history id="none"><transition target="a"/></history>
             <state id="a"><transition event="go" target="b"/></state>
             <state id="b"><transition event="back" target="none"/></state>
           </state>"#,
    );
    assert_eq!(enumeration, Enumeration::History);
    assert_eq!((name.as_str(), clash), ("none", Clash::Generated));
    assert_eq!(backends, ["c11"]);
}

#[test]
fn a_token_spelled_as_kotlins_self_member_is_refused() {
    // `foo` is an event and the prefix of `foo.zoo`, so Kotlin declares
    // `Foo.Self` for the event itself — which `foo.self` would declare too.
    let (enumeration, name, clash, backends) = refusal(
        r#"<state id="a">
             <transition event="foo" target="b"/>
             <transition event="foo.zoo" target="b"/>
             <transition event="foo.self" target="b"/>
           </state>
           <final id="b"/>"#,
    );
    assert_eq!(enumeration, Enumeration::Event);
    assert_eq!((name.as_str(), clash), ("foo.self", Clash::Generated));
    assert_eq!(backends, ["kotlin"]);
}

// ── The spelling is the templates' ────────────────────────────────────

/// Names that exercise every conversion — a dash, a dot, an underscore,
/// camelCase, a history, the wildcard, an event that is a prefix of
/// another, entry and exit blocks — and collide nowhere.
const EVERY_CONVERSION: &str = r#"
  <state id="idle">
    <onentry><log label="in" expr="'idle'"/></onentry>
    <transition event="go" target="door-open"/>
    <transition event="door.open" target="coolDown"/>
  </state>
  <state id="door-open">
    <onexit><log label="out" expr="'door'"/></onexit>
    <transition event="cool-down" target="s_1"/>
    <transition event="a.b.c" target="a.b"/>
  </state>
  <state id="coolDown" initial="warm">
    <history id="h_deep" type="deep"><transition target="warm"/></history>
    <state id="warm"><transition event="*" target="done"/></state>
    <transition event="back" target="h_deep"/>
  </state>
  <state id="a.b"><transition event="a.b" target="s_1"/></state>
  <state id="s_1"><transition event="go" target="done"/></state>
  <final id="done"/>"#;

/// The members one backend's output declares in one family, read from the
/// source text by the declaration shape that backend uses for it.
fn rendered(language: Language, label: &str, files: &[(String, String)]) -> BTreeSet<String> {
    let all: String = files
        .iter()
        .map(|(_, code)| code.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let block = |open: &str, close: &str| -> String {
        let start = all
            .find(open)
            .unwrap_or_else(|| panic!("{language:?}: no `{open}`"));
        let rest = &all[start + open.len()..];
        rest[..rest.find(close).expect("block closes")].to_string()
    };
    let capture = |text: &str, pattern: &str| -> BTreeSet<String> {
        Regex::new(pattern)
            .unwrap()
            .captures_iter(text)
            .map(|c| c[1].to_string())
            .collect()
    };
    // A C++ or C enum body: members separated by commas, `//` comments
    // stripped.
    let enumerators = |body: String, prefix: &str| -> BTreeSet<String> {
        body.lines()
            .map(|l| l.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ")
            .split(',')
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(|m| m.strip_prefix(prefix).unwrap_or(m).to_string())
            .collect()
    };
    match (language, label) {
        (Language::Rust, "State" | "History" | "Event") => capture(
            &block(&format!("pub enum Names{label} {{"), "\n}"),
            r"(?m)^\s+([A-Za-z_][A-Za-z0-9_]*),\s*$",
        ),
        (Language::Cpp, "State" | "History" | "Event") => {
            enumerators(block(&format!("enum class {label} : uint8_t {{"), "}"), "")
        }
        (Language::Kotlin, "State") => capture(&all, r"data object (\w+) : NamesState\b"),
        (Language::Kotlin, "History") => capture(&all, r"val history(\w+) = HistoryId\("),
        (Language::Go, "State") => capture(&all, r"(?m)^\s+NamesState(\w+) NamesState = \d+"),
        (Language::Go, "History") => {
            capture(&all, r"(?m)^\s+NamesHistory(\w+) sce\.HistoryID = \d+")
        }
        (Language::Go, "Event") => capture(&all, r"(?m)^\s+NamesEvent(\w+) NamesEvent = \d+"),
        (Language::Python, "State" | "Event") => capture(
            &block(&format!("class Names{label}(IntEnum):"), "\n\n"),
            r"(?m)^    ([A-Z0-9_]+) = \d+",
        ),
        (Language::C11, "State") => enumerators(block("enum names_state_e {", "}"), "NAMES_STATE_"),
        (Language::C11, "History") => {
            enumerators(block("enum names_history_e {", "}"), "NAMES_HIST_")
        }
        (Language::C11, "Event") => enumerators(block("enum names_event_e {", "}"), "NAMES_EVENT_"),
        (Language::C11, "on_entry") => capture(&all, r"names_on_entry_(\w+)_block_\d+\("),
        (Language::C11, "on_exit") => capture(&all, r"names_on_exit_(\w+)_block_\d+\("),
        other => panic!("no reader for {other:?}: add one beside the family"),
    }
}

#[test]
fn every_member_a_backend_declares_is_the_spelling_the_refusal_asks() {
    let dir = tempdir().expect("tempdir");
    let path = write(dir.path(), &document(EVERY_CONVERSION));
    let path_str = path.to_str().unwrap();

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_file(path_str).expect("parses");
    sce_build::analyzer::analyze(&mut model, path_str);
    assert!(
        sce_build::member_names::first_collision(&model).is_none(),
        "the conformance document must collide nowhere"
    );

    let declared = sce_build::member_names::declared(&model);
    // Every backend has at least its state family here, so a language whose
    // families vanished from the table would not pass by having none.
    for &language in Language::ALL {
        assert!(declared
            .iter()
            .any(|d| d.language == language && d.label == "State"));
    }
    for &language in Language::ALL {
        let output = compile_scxml_lang_typed(path_str, &find_template_dir_for(language), language)
            .unwrap_or_else(|e| panic!("{language:?} refused the conformance document: {e:?}"));
        for family in declared.iter().filter(|d| d.language == language) {
            let actual = rendered(language, family.label, &output.files);
            assert_eq!(
                actual, family.members,
                "{language:?} {}: the rendered members differ from what \
                 `member_names` spells — a template and the refusal disagree",
                family.label
            );
            assert!(
                family.sentinels.is_subset(&actual),
                "{language:?} {}: a listed sentinel is not declared",
                family.label
            );
        }
    }
}

#[test]
fn a_dotted_and_a_dashed_state_become_valid_identifiers_in_every_backend() {
    // Both are legal XML Names. Before the spelling was fixed, `a.b` became
    // `A.b` in Rust, C++, Go and Kotlin, and `door-open` became C++'s
    // `Door - open`.
    let dir = tempdir().expect("tempdir");
    let path = write(dir.path(), &document(EVERY_CONVERSION));
    let path_str = path.to_str().unwrap();
    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_file(path_str).expect("parses");
    sce_build::analyzer::analyze(&mut model, path_str);
    let identifier = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    for family in sce_build::member_names::declared(&model) {
        for member in &family.members {
            assert!(
                identifier.is_match(member),
                "{:?} {} declares `{member}`, which is not an identifier",
                family.language,
                family.label
            );
        }
    }
}
