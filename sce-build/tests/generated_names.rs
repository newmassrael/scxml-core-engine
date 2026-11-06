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

use sce_build::forge::codegen_matrix::{emitting, emitting_in_part};
use sce_build::forge::error::ForgeError;
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::member_names::{Clash, Declared, Enumeration};
use sce_build::reader_names::{reserved_in, Case, Spellings, UnreadableReason, NOT_DECLARED};
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

/// Every backend that declares members, as a refusal names them: by canonical
/// name, in `Language::ALL` order.
///
/// The refusal is asked of every backend ([`refusal`]) and is the same for all
/// of them, but it names the backends that FOLD the pair, and a backend folds
/// only names it declares. One with no template declares none, so it is
/// neither in this list nor absent from the refusal.
fn every_backend() -> Vec<&'static str> {
    emitting(ForgeKind::Statechart)
        .map(|language| language.canonical_name())
        .collect()
}

/// [`every_backend`] and the backends that write a statechart in part
/// ([`emitting_in_part`]): Swift declares its members and is asked the refusal
/// like the rest, so a pair it folds names it, as one it keeps apart (a pair of
/// states that differ only in case) leaves it out.
fn every_backend_that_declares() -> Vec<&'static str> {
    emitting_in_part(ForgeKind::Statechart)
        .map(|language| language.canonical_name())
        .collect()
}

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
    assert_eq!(backends, every_backend());
}

#[test]
fn camel_and_snake_case_states_are_refused_where_the_backend_folds_them() {
    // C++ keeps `DoorOpen` and `Door_open` apart and C11 `DOOROPEN` and
    // `DOOR_OPEN`; the PascalCase backends and Swift's lowerCamel (`doorOpen`
    // for both) do not.
    let (_, name, clash, backends) = refusal(
        r#"<state id="doorOpen"><transition event="go" target="door_open"/></state>
           <state id="door_open"><transition event="go" target="doorOpen"/></state>"#,
    );
    assert_eq!(
        (name.as_str(), clash),
        ("door_open", Clash::Name("doorOpen".into()))
    );
    assert_eq!(backends, ["rust", "kotlin", "go", "python", "swift"]);
}

#[test]
fn a_dash_and_an_underscore_in_a_state_are_one_member_everywhere() {
    // `-` is written as a delimiter by every backend, so `door-open` and
    // `door_open` meet in all seven.
    let (_, name, clash, backends) = refusal(
        r#"<state id="door-open"><transition event="go" target="door_open"/></state>
           <state id="door_open"><transition event="go" target="door-open"/></state>"#,
    );
    assert_eq!(
        (name.as_str(), clash),
        ("door_open", Clash::Name("door-open".into()))
    );
    assert_eq!(backends, every_backend_that_declares());
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
    let folding: Vec<&str> = emitting_in_part(ForgeKind::Statechart)
        // Kotlin: keeps the dotted and the underscored event apart.
        .filter(|language| *language != Language::Kotlin)
        .map(Language::canonical_name)
        .collect();
    assert_eq!(backends, folding);
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
fn an_event_spelled_as_the_wildcards_member_is_refused_for_every_backend() {
    // `event="*"` is a member of every backend's event type: the one an arriving
    // name that no event of the document matches is delivered as. An event named
    // `wildcard` therefore spells the same member in all six.
    let (enumeration, name, clash, backends) = refusal(
        r#"<state id="a"><transition event="wildcard" target="b"/><transition event="*" target="b"/></state><final id="b"/>"#,
    );
    assert_eq!(enumeration, Enumeration::Event);
    assert_eq!(
        (name.as_str(), clash),
        ("wildcard", Clash::Name("Wildcard".to_string()))
    );
    assert_eq!(backends, every_backend());
}

#[test]
fn an_event_spelled_as_the_eventless_null_is_refused_where_it_is_declared() {
    // Rust and Go declare `Null` and Swift `null`; Python spells the event
    // `NULL_` to avoid its own sentinel, and C++ and C11 reserve a different word.
    let (_, name, clash, backends) =
        refusal(r#"<state id="a"><transition event="null" target="b"/></state><final id="b"/>"#);
    assert_eq!((name.as_str(), clash), ("null", Clash::Generated));
    assert_eq!(backends, ["rust", "go", "swift"]);
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
        // A Rust variant, with the discriminant that is its number in the
        // machine's tables (docs/adr/0064).
        (Language::Rust, "State" | "History" | "Event") => capture(
            &block(&format!("pub enum Names{label} {{"), "\n}"),
            r"(?m)^\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*=\s*\d+)?,\s*$",
        ),
        (Language::Cpp, "State" | "History" | "Event") => {
            enumerators(block(&format!("enum class {label} : uint8_t {{"), "}"), "")
        }
        (Language::Kotlin, "State") => capture(&all, r"data object (\w+) : NamesState\b"),
        // A getter with nothing to initialise, so the companion object's class
        // initialiser does not grow with the document's histories.
        (Language::Kotlin, "History") => {
            capture(&all, r"val history(\w+): HistoryId get\(\) = HistoryId\(")
        }
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
        // A case or a static may be a reserved word in backticks, which is
        // how the spelling declares it (`filters::to_swift_variant`).
        (Language::Swift, "State" | "Event") => capture(
            &block(
                &format!("public enum Names{label}: Int, Hashable, Sendable {{"),
                "\n}",
            ),
            r"(?m)^    case (`?\w+`?) = \d+",
        ),
        (Language::Swift, "History") => capture(
            &block("public enum NamesHistory {", "\n}"),
            r"(?m)^    public static let (`?\w+`?) = HistoryID\(",
        ),
        (Language::C11, "on_entry") => capture(&all, r"names_on_entry_(\w+)_block_\d+\("),
        (Language::C11, "on_exit") => capture(&all, r"names_on_exit_(\w+)_block_\d+\("),
        // A family no reader above covers, for any backend. The backends are
        // named so that one added to `Language` is a compile error here.
        other @ (
            Language::Rust
            | Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11
            | Language::Swift,
            _,
        ) => panic!("no reader for {other:?}: add one beside the family"),
    }
}

/// What [`EVERY_CONVERSION`] holds for Swift, which lowers a statechart in part
/// and refuses the `<log>` that document carries: a `datamodel="null"` machine,
/// with the one action it lowers.
const SWIFT_CONVERSION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="names" datamodel="null">
  <state id="idle">
    <onentry><raise event="go"/></onentry>
    <transition event="go" target="door-open"/>
    <transition event="door.open" target="coolDown"/>
  </state>
  <state id="door-open">
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
  <final id="done"/>
</scxml>
"#;

#[test]
fn every_member_a_backend_declares_is_the_spelling_the_refusal_asks() {
    every_member_is_the_spelling_the_refusal_asks(
        &document(EVERY_CONVERSION),
        emitting(ForgeKind::Statechart).collect(),
    );
}

#[test]
fn every_member_swift_declares_is_the_spelling_the_refusal_asks() {
    every_member_is_the_spelling_the_refusal_asks(SWIFT_CONVERSION, vec![Language::Swift]);
}

fn every_member_is_the_spelling_the_refusal_asks(text: &str, languages: Vec<Language>) {
    let dir = tempdir().expect("tempdir");
    let path = write(dir.path(), text);
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
    for &language in &languages {
        assert!(declared
            .iter()
            .any(|d| d.language == language && d.label == "State"));
    }
    for &language in &languages {
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

// ── A keyword is escaped, never declared bare ─────────────────────────

/// State ids, a history id, event names and data ids that are words some
/// backend reserves: the codegen smoke fixture, which every compile check of
/// `codegen_smoke` also builds, read here so both ask one document.
fn keyword_names() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codegen_smoke/keyword_names.scxml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The keyword document, written as `names.scxml` (the machine every reader
/// above looks for), parsed and analyzed as the generator does.
fn keyword_model(dir: &Path) -> (PathBuf, sce_build::model::SCXMLModel) {
    let path = write(dir, &keyword_names());
    let path_str = path.to_str().expect("a UTF-8 path").to_string();
    let mut model = sce_build::parser::SCXMLParser::new()
        .parse_file(&path_str)
        .expect("the keyword document parses");
    sce_build::analyzer::analyze(&mut model, &path_str);
    (path, model)
}

/// Whether `language` reserves `spelled`, asked of the one keyword list
/// `reader_names` keeps for every backend.
fn reserved_word(language: Language, spelled: &str) -> bool {
    let mut spellings: Spellings = NOT_DECLARED;
    let column = Language::ALL
        .iter()
        .position(|l| *l == language)
        .expect("a language of Language::ALL");
    spellings[column] = &[Case::Verbatim];
    reserved_in(spelled, &spellings).is_some()
}

/// Whether a family's members stand alone in the source. Go writes each after
/// `<Machine>State`, `<Machine>Event` or `<Machine>History`, C11 after its
/// machine prefix, and Kotlin a history as `history<X>`, so no member of
/// theirs can be a keyword.
fn written_bare(language: Language, label: &str) -> bool {
    !matches!(
        (language, label),
        (Language::Go | Language::C11, _) | (Language::Kotlin, "History")
    )
}

/// The members `language` declares for one family of the document.
fn members_of<'a>(
    declared: &'a [Declared],
    language: Language,
    label: &str,
) -> &'a BTreeSet<String> {
    &declared
        .iter()
        .find(|d| d.language == language && d.label == label)
        .unwrap_or_else(|| panic!("{language:?} declares no {label} family"))
        .members
}

/// The words Swift's grammar reads after a `.`, which a backtick does not
/// escape where the template names a case (`filters::to_swift_variant`).
const SWIFT_WORDS_AFTER_A_DOT: [&str; 5] = ["Protocol", "Self", "Type", "init", "self"];

#[test]
fn a_keyword_is_spelled_as_a_member_every_backend_can_declare() {
    let dir = tempdir().expect("tempdir");
    let (_, model) = keyword_model(dir.path());
    assert!(
        sce_build::member_names::first_collision(&model).is_none(),
        "no escaped spelling may meet another name of the document"
    );
    let declared = sce_build::member_names::declared(&model);
    let identifier = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    let mut asked = 0usize;
    for family in declared
        .iter()
        .filter(|f| written_bare(f.language, f.label))
    {
        for member in &family.members {
            let quoted = member.strip_prefix('`').and_then(|m| m.strip_suffix('`'));
            match quoted {
                // A backticked word is Swift's escape: only a word Swift
                // reserves takes it, and never one it reads after a `.`.
                Some(word) => {
                    assert_eq!(family.language, Language::Swift, "`{member}`");
                    assert!(identifier.is_match(word), "`{member}`");
                    assert!(
                        reserved_word(Language::Swift, word),
                        "Swift {}: `{member}` is quoted but no reserved word",
                        family.label
                    );
                    assert!(
                        !SWIFT_WORDS_AFTER_A_DOT.contains(&word),
                        "Swift {}: `{member}` is a word a backtick does not reach after a `.`",
                        family.label
                    );
                }
                None => {
                    assert!(
                        identifier.is_match(member),
                        "{:?} {} declares `{member}`, which is not an identifier",
                        family.language,
                        family.label
                    );
                    assert!(
                        !reserved_word(family.language, member),
                        "{:?} {} declares `{member}`, a word the language reserves",
                        family.language,
                        family.label
                    );
                }
            }
            asked += 1;
        }
    }
    assert!(asked > 0, "no member was asked, so this measured nothing");

    // Each escape where it is needed, and the bare word nowhere beside it.
    for (language, label, escaped, bare) in [
        (Language::Rust, "State", "Self_", "Self"),
        (Language::Rust, "Event", "Self_", "Self"),
        (Language::Kotlin, "State", "Self_", "Self"),
        (Language::Go, "State", "Self_", "Self"),
        (Language::Go, "Event", "Self_", "Self"),
        (Language::Swift, "State", "self_", "`self`"),
        (Language::Swift, "State", "init_", "`init`"),
        (Language::Swift, "Event", "Self_", "`Self`"),
        (Language::Swift, "Event", "init_", "`init`"),
        (Language::Swift, "State", "`in`", "in"),
        (Language::Swift, "State", "`default`", "default"),
        (Language::Swift, "History", "`is`", "is"),
        (Language::Swift, "Event", "`operator`", "operator"),
    ] {
        let spelled = members_of(&declared, language, label);
        assert!(
            spelled.contains(escaped) && !spelled.contains(bare),
            "{language:?} {label}: expected `{escaped}` and no `{bare}` in {spelled:?}"
        );
    }
    // Where no keyword has the shape of the spelling, the name is spelled as
    // it always was: C++ capitalises, Python and C11 write capitals.
    for (language, label, spelled) in [
        (Language::Cpp, "State", "Self"),
        (Language::Cpp, "Event", "Self"),
        (Language::Cpp, "State", "None"),
        (Language::Python, "State", "SELF"),
        (Language::Python, "State", "NONE"),
        (Language::Python, "Event", "IN"),
        (Language::C11, "State", "SELF"),
        (Language::Rust, "State", "None"),
        (Language::Rust, "History", "Is"),
    ] {
        assert!(
            members_of(&declared, language, label).contains(spelled),
            "{language:?} {label}: expected `{spelled}`"
        );
    }
}

/// The generated source of every backend declares the escaped members: the
/// spelling the test above asks is the one each template writes.
#[test]
fn every_member_a_backend_declares_for_a_keyword_is_the_spelling_the_refusal_asks() {
    every_member_is_the_spelling_the_refusal_asks(&keyword_names(), Language::ALL.to_vec());
}

#[test]
fn a_keyword_data_id_is_read_through_an_escaped_reader_or_has_none() {
    let dir = tempdir().expect("tempdir");
    let (path, model) = keyword_model(dir.path());
    let reader = |id: &str| {
        model
            .readable_variables
            .iter()
            .find(|v| v.id == id)
            .and_then(|v| v.reader.clone())
            .unwrap_or_else(|| panic!("`{id}` has a reader"))
    };
    assert_eq!(reader("fn").rust, "r#fn");
    assert_eq!(reader("match").rust, "r#match");
    assert_eq!(reader("when").kotlin, "`when`");
    assert_eq!(reader("lambda").python, "lambda_");
    assert_eq!(reader("guard").swift, "`guard`");
    assert_eq!(reader("chan").go, "Chan");

    // A word no escape reaches in one backend has a reader in none, and says
    // which backend and why.
    let mut withheld: Vec<(String, UnreadableReason, Option<&str>)> = model
        .unreadable_variables
        .iter()
        .map(|u| (u.var.clone(), u.reason, u.language))
        .collect();
    withheld.sort();
    assert_eq!(
        withheld,
        vec![
            ("Self".to_string(), UnreadableReason::Keyword, Some("rust")),
            ("auto".to_string(), UnreadableReason::Keyword, Some("cpp")),
        ]
    );

    // Every reader that is not escaped is a word its language leaves free.
    // C11's is the tail of a prefixed name, so it is not asked.
    for var in &model.readable_variables {
        let names = var
            .reader
            .as_ref()
            .expect("a readable variable has a reader");
        for &language in Language::ALL {
            if language == Language::C11 {
                continue;
            }
            let spelled = names.get(language);
            if spelled.starts_with("r#") || spelled.starts_with('`') {
                continue;
            }
            assert!(
                !reserved_word(language, spelled),
                "{language:?}: the reader of `{}` is `{spelled}`, a reserved word",
                var.id
            );
        }
    }

    // And the generated source declares the escaped readers, and no reader
    // for the word Rust cannot escape.
    let path_str = path.to_str().unwrap();
    let source = |language: Language| -> String {
        compile_scxml_lang_typed(path_str, &find_template_dir_for(language), language)
            .unwrap_or_else(|e| panic!("{language:?} refused the keyword document: {e:?}"))
            .files
            .into_iter()
            .map(|(_, code)| code)
            .collect::<Vec<_>>()
            .join("\n")
    };
    let rust = source(Language::Rust);
    assert!(rust.contains("pub fn r#fn(&self)"), "{rust}");
    assert!(rust.contains("pub fn r#match(&self)"), "{rust}");
    assert!(
        !rust.contains("r#self") && !rust.contains("r#Self"),
        "{rust}"
    );
    let kotlin = source(Language::Kotlin);
    assert!(kotlin.contains("fun `when`()"), "{kotlin}");
    let python = source(Language::Python);
    assert!(python.contains("def lambda_(self)"), "{python}");
    let go = source(Language::Go);
    assert!(go.contains("Policy) Chan() ("), "{go}");
}
