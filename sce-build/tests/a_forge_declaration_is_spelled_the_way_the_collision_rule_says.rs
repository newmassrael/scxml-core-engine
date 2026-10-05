// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The collision rule compares names as the generated code spells them, so
//! the table of how it spells them (`forge::declared_names::SCOPES`) is a
//! claim about the templates. This holds the claim to them.
//!
//! A rule that decides two names collide is only as right as its table. A
//! convention the table has wrong is a pair refused that builds, or one
//! accepted that does not, and neither shows in the rule's own tests, which
//! read the same table.
//!
//! # What is measured
//!
//! Every declaration of every forge document this tree commits is renamed to
//! a probe whose five spellings (as written, snake, Pascal, camel, upper
//! snake) all differ, in every place the document writes it, and the document
//! is generated in each backend the table gives a spelling for. Each
//! declaring attribute (a row) is judged on its own, because a slot's rows are
//! not spelled alike: C++ writes a codec `field` as written and an `embed`
//! snake_case. The probe is then looked for in the output:
//!
//! - when the generated code mentions the probe in any spelling, the one the
//!   table gives must be among them. A folded spelling is written only by a
//!   declaration or a use, since a comment and a string write the name as
//!   written;
//! - where the table says a backend writes the name as written, no folded
//!   spelling of it may appear;
//! - when the generated code mentions it in none, the kind does not emit the
//!   name (a CBOR codec's `<data id>` is not a member), nothing can collide,
//!   and the document is counted and left out.
//!
//! A document that cannot be renamed or generated in a backend (an import the
//! rename breaks, a kind a backend does not emit, a fixture refused on
//! purpose) is counted and left out too, and a floor on what is verified per
//! row and backend keeps the rest from being vacuous.
//!
//! `measure_the_spelling_of_every_declaring_attribute` is how a row is added:
//! it prints, for every declaring attribute and backend, which spellings the
//! generated code used, and is run by hand (`-- --ignored --nocapture`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_build::forge::declared_names::{declares, Ns, Row, Slot, SCOPES};
use sce_build::generator::Language;
use sce_build::reader_names::Case;
use sce_build::scxml_identifier::Dialect;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

/// Five spellings, all different: written, snake, Pascal, camel, upper snake.
const PROBE: &str = "min_rpmMax";

const CASES: [Case; 5] = [
    Case::Verbatim,
    Case::Snake,
    Case::Pascal,
    Case::Camel,
    Case::UpperSnake,
];

fn case_name(case: Case) -> &'static str {
    match case {
        Case::Verbatim => "verbatim",
        Case::Snake => "snake",
        Case::Pascal => "pascal",
        Case::Camel => "camel",
        Case::UpperSnake => "upper",
    }
}

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The forge fixtures, in a fixed order. A directory a `paths:` filter can
/// name, which is why this reads it and does not ask git for every document
/// the tree holds.
fn forge_fixtures() -> Vec<PathBuf> {
    let mut documents: Vec<PathBuf> = std::fs::read_dir(resource_dir())
        .expect("the forge fixtures are read")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .collect();
    documents.sort();
    documents
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// `text` with every occurrence of the identifier `from` written as a value
/// or an expression replaced by `to`.
///
/// An occurrence is one the identifier stands alone in: not part of a longer
/// name, and not an element's or attribute's own name (`<sce:field`,
/// `</field>`, ` value=`), which a rename must leave alone.
fn replace_identifier(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (at, _) in text.match_indices(from) {
        if at < last {
            continue;
        }
        let end = at + from.len();
        let before = text[..at].chars().next_back();
        let after = text[end..].chars().next();
        let standalone =
            before.is_none_or(|c| !is_word_char(c)) && after.is_none_or(|c| !is_word_char(c));
        let a_tag_or_prefix = matches!(before, Some('<' | '/' | ':'));
        let an_attribute_name = before.is_some_and(char::is_whitespace)
            && text[end..].trim_start_matches([' ', '\t']).starts_with('=');
        if standalone && !a_tag_or_prefix && !an_attribute_name {
            out.push_str(&text[last..at]);
            out.push_str(to);
            last = end;
        }
    }
    out.push_str(&text[last..]);
    out
}

/// Whether `word` appears in `text` as an identifier.
fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        before.is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'))
            && after.is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'))
    })
}

fn namespace_uri(ns: Ns) -> &'static str {
    match ns {
        Ns::Scxml => "http://www.w3.org/2005/07/scxml",
        Ns::Sce => sce_build::forge::model::SCE_NAMESPACE,
    }
}

/// Every distinct name the declaring attribute `row` carries in `text`.
///
/// With a `slot`, the names the rule compares for it, by the rule's own
/// predicate — its kinds and its direction included — so that what is measured
/// is what is compared. Without one, every name the attribute carries, for
/// measuring a row the table does not list yet.
fn declared_by(row: &Row, slot: Option<&Slot>, text: &str) -> Vec<String> {
    let Ok(parsed) = roxmltree::Document::parse(text) else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    for node in parsed.descendants().filter(|n| n.is_element()) {
        if node.tag_name().namespace() != Some(namespace_uri(row.ns))
            || node.tag_name().name() != row.element
        {
            continue;
        }
        match slot {
            Some(slot) => {
                if !declares(slot, &node, row.attr, Dialect::Forge) {
                    continue;
                }
            }
            // Inside `<sce:peek-byte>` a flag is only a mask lookup.
            None => {
                if row.element == "flag"
                    && node
                        .parent_element()
                        .is_some_and(|p| p.tag_name().name() == "peek-byte")
                {
                    continue;
                }
            }
        }
        if let Some(value) = node.attribute(row.attr) {
            if !names.iter().any(|n| n == value) {
                names.push(value.to_string());
            }
        }
    }
    names
}

/// The `sce:direction` of the element that declares `name` through `row`, or
/// `-` where it carries none: the role a `<data>` plays in its kind.
fn direction_of(row: &Row, text: &str, name: &str) -> String {
    let Ok(parsed) = roxmltree::Document::parse(text) else {
        return "-".to_string();
    };
    parsed
        .descendants()
        .filter(|n| {
            n.is_element()
                && n.tag_name().namespace() == Some(namespace_uri(row.ns))
                && n.tag_name().name() == row.element
                && n.attribute(row.attr) == Some(name)
        })
        .find_map(|n| {
            n.attribute((sce_build::forge::model::SCE_NAMESPACE, "direction"))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "-".to_string())
}

/// The kind a document declares (`sce:kind`), or `statechart` for one that
/// declares none.
fn kind_of(text: &str) -> String {
    roxmltree::Document::parse(text)
        .ok()
        .and_then(|d| {
            d.root_element()
                .attribute((sce_build::forge::model::SCE_NAMESPACE, "kind"))
                .map(str::to_string)
        })
        .unwrap_or_else(|| "statechart".to_string())
}

/// Everything `language` generates for `text`, or why it cannot.
fn generate(text: &str, language: Language) -> Result<String, String> {
    // An algorithm names itself in its signature, not on the root.
    let name = roxmltree::Document::parse(text)
        .ok()
        .and_then(|d| d.root_element().attribute("name").map(str::to_string))
        .unwrap_or_else(|| "probe".to_string());
    let options = ForgeCompileOptions {
        // A Go document that imports another needs the module it is in.
        go_module_prefix: Some("github.com/test/codec".to_string()),
        ..ForgeCompileOptions::default()
    };
    let output = compile_forge_with_imports(
        text,
        DocumentLabel::symmetric(&name),
        language,
        &resource_dir(),
        &options,
    )
    .map_err(|e| e.error.to_string())?;
    Ok(output
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n"))
}

/// `source` without the lines that are only a comment, which write a name as
/// the author wrote it whatever the code does with it.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !(t.starts_with("//")
                || t.starts_with('#')
                || t.starts_with("/*")
                || t.starts_with('*')
                || t.starts_with("\"\"\""))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Which spellings of the probe `source` mentions as identifiers in code.
fn spellings_used(source: &str) -> Vec<Case> {
    let code = code_only(source);
    CASES
        .iter()
        .copied()
        .filter(|c| contains_word(&code, &c.spell(PROBE)))
        .collect()
}

/// A line of code that mentions the probe spelled `case`, so that a mismatch
/// can be read without regenerating.
fn sample_line(source: &str, case: Case) -> String {
    let word = case.spell(PROBE);
    code_only(source)
        .lines()
        .find(|l| contains_word(l, &word))
        .map(|l| l.trim().chars().take(110).collect())
        .unwrap_or_default()
}

/// What one declaration, in one backend, was found to be.
enum Verdict {
    Spelled,
    Wrong(String),
    Skipped(String),
}

fn judge(case: Case, source: &str) -> Verdict {
    let used = spellings_used(source);
    if used.is_empty() {
        return Verdict::Skipped("the kind does not emit the name".to_string());
    }
    if !used.contains(&case) {
        let sample = sample_line(source, used[0]);
        let used: Vec<&str> = used.iter().map(|c| case_name(*c)).collect();
        return Verdict::Wrong(format!(
            "{} expected, {used:?} used: `{sample}`",
            case_name(case)
        ));
    }
    if case == Case::Verbatim {
        // Written as written: no fold of it is the name's spelling.
        if let Some(folded) = used.iter().find(|c| **c != Case::Verbatim) {
            return Verdict::Wrong(format!(
                "written as written expected, and the {} spelling appears: `{}`",
                case_name(*folded),
                sample_line(source, *folded)
            ));
        }
    }
    Verdict::Spelled
}

#[test]
fn the_probe_is_spelled_five_ways() {
    // The measure needs five distinguishable spellings, or it measures
    // nothing.
    let spelled: Vec<String> = CASES.iter().map(|c| c.spell(PROBE)).collect();
    for (i, a) in spelled.iter().enumerate() {
        for b in &spelled[i + 1..] {
            assert_ne!(a, b, "{spelled:?}");
        }
    }
}

#[test]
fn every_row_is_spelled_the_way_the_table_says() {
    let documents = forge_fixtures();
    assert!(documents.len() > 150, "{} documents", documents.len());

    // (scope, row, backend) -> verified count.
    let mut verified: BTreeMap<(String, String, &str), usize> = BTreeMap::new();
    let mut skipped = 0usize;
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let mut wrong: Vec<String> = Vec::new();

    for path in &documents {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for scope in SCOPES {
            for slot in scope.slots {
                for row in slot.rows {
                    let row_id = format!("{}@{}", row.element, row.attr);
                    for name in declared_by(row, Some(slot), &text) {
                        let renamed = replace_identifier(&text, &name, PROBE);
                        for (backend, language) in Language::ALL.iter().enumerate() {
                            let Some(case) = slot.cases[backend] else {
                                continue;
                            };
                            let verdict = match generate(&renamed, *language) {
                                Err(why) => Verdict::Skipped(why),
                                Ok(source) => judge(case, &source),
                            };
                            let lang = language.canonical_name();
                            match verdict {
                                Verdict::Spelled => {
                                    *verified
                                        .entry((scope.id.to_string(), row_id.clone(), lang))
                                        .or_default() += 1;
                                }
                                Verdict::Skipped(why) => {
                                    skipped += 1;
                                    let why: String = why.chars().take(110).collect();
                                    *reasons.entry(format!("{lang}: {why}")).or_default() += 1;
                                }
                                Verdict::Wrong(why) => wrong.push(format!(
                                    "{} [{}/{row_id}] `{name}` in {lang}: {why}",
                                    path.file_name().unwrap().to_string_lossy(),
                                    scope.id
                                )),
                            }
                        }
                    }
                }
            }
        }
    }

    // What the table gets wrong is what this exists to find, so it is said
    // before anything about how much was checked.
    wrong.sort();
    wrong.dedup();
    assert!(
        wrong.is_empty(),
        "{} declaration(s) are not spelled the way the table says:\n{}",
        wrong.len(),
        wrong
            .iter()
            .take(60)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );

    // A row and backend the table claims and nothing verified is a claim
    // nobody checked.
    let mut why_skipped: Vec<(usize, &String)> = reasons.iter().map(|(r, n)| (*n, r)).collect();
    why_skipped.sort_by(|a, b| b.cmp(a));
    let top: Vec<String> = why_skipped
        .iter()
        .take(12)
        .map(|(n, r)| format!("  {n} x {r}"))
        .collect();
    for scope in SCOPES {
        for slot in scope.slots {
            for row in slot.rows {
                for (backend, language) in Language::ALL.iter().enumerate() {
                    if slot.cases[backend].is_none() {
                        continue;
                    }
                    let key = (
                        scope.id.to_string(),
                        format!("{}@{}", row.element, row.attr),
                        language.canonical_name(),
                    );
                    let count = verified.get(&key).copied().unwrap_or(0);
                    assert!(
                        count >= 1,
                        "the table says {} spells `{}@{}` of `{}` and nothing confirmed it \
                         ({skipped} skipped in all; the commonest reasons:\n{})",
                        language.canonical_name(),
                        row.element,
                        row.attr,
                        scope.id,
                        top.join("\n")
                    );
                }
            }
        }
    }
}

/// Every attribute that might declare a name, whether or not the table lists
/// it yet: how each backend spells it, measured.
#[test]
#[ignore = "a measuring tool for adding a row to the table; run with --ignored --nocapture"]
fn measure_the_spelling_of_every_declaring_attribute() {
    const CANDIDATES: &[(Ns, &str, &str)] = &[
        (Ns::Scxml, "data", "id"),
        (Ns::Sce, "field", "id"),
        (Ns::Sce, "flags", "id"),
        (Ns::Sce, "repeat", "id"),
        (Ns::Sce, "tlv-chain", "id"),
        (Ns::Sce, "embed", "id"),
        (Ns::Sce, "peek-byte", "id"),
        (Ns::Sce, "flag", "name"),
        (Ns::Sce, "flag-input", "name"),
        (Ns::Sce, "variant", "name"),
        (Ns::Sce, "const", "name"),
        (Ns::Sce, "var", "name"),
        (Ns::Sce, "param", "name"),
        (Ns::Sce, "helper", "name"),
        (Ns::Sce, "action", "name"),
        (Ns::Sce, "arg", "name"),
        (Ns::Sce, "extern", "name"),
        (Ns::Sce, "fold", "as"),
        (Ns::Sce, "cycle", "id"),
        (Ns::Sce, "import", "as"),
        (Ns::Sce, "context", "id"),
        // A procedure's states are the members of an enum.
        (Ns::Scxml, "state", "id"),
        (Ns::Scxml, "final", "id"),
    ];
    let documents = forge_fixtures();

    // (row, backend) -> per-case count of renamings that used that spelling,
    // renamings generated, renamings that mentioned the name in no spelling.
    #[derive(Default)]
    struct Tally {
        per_case: [usize; 5],
        generated: usize,
        unmentioned: usize,
    }
    let mut tally: BTreeMap<(String, &str), Tally> = BTreeMap::new();
    // (row, backend) -> renamings that could not be generated, and the first
    // reason, so that a row nothing generated can be told from a row that is
    // not emitted.
    let mut failures: BTreeMap<(String, &str), (usize, String)> = BTreeMap::new();

    for path in &documents {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for (ns, element, attr) in CANDIDATES {
            let row = Row {
                ns: *ns,
                element,
                attr,
            };
            let kind = kind_of(&text);
            for name in declared_by(&row, None, &text) {
                let label = format!(
                    "{element}@{attr}[{kind}/{}]",
                    direction_of(&row, &text, &name)
                );
                let renamed = replace_identifier(&text, &name, PROBE);
                for language in Language::ALL {
                    let source = match generate(&renamed, *language) {
                        Ok(source) => source,
                        Err(why) => {
                            let slot = failures
                                .entry((label.clone(), language.canonical_name()))
                                .or_insert((0, why.chars().take(130).collect()));
                            slot.0 += 1;
                            continue;
                        }
                    };
                    let entry = tally
                        .entry((label.clone(), language.canonical_name()))
                        .or_default();
                    entry.generated += 1;
                    let used = spellings_used(&source);
                    if used.is_empty() {
                        entry.unmentioned += 1;
                    }
                    for case in used {
                        entry.per_case[CASES.iter().position(|c| *c == case).unwrap()] += 1;
                    }
                }
            }
        }
    }

    eprintln!("row@attr  backend  generated  unmentioned | verbatim snake pascal camel upper");
    for ((row, language), t) in &tally {
        eprintln!(
            "{row:<16} {language:<7} {:>4} {:>4} | {:>4} {:>4} {:>4} {:>4} {:>4}",
            t.generated,
            t.unmentioned,
            t.per_case[0],
            t.per_case[1],
            t.per_case[2],
            t.per_case[3],
            t.per_case[4]
        );
    }
    eprintln!("-- renamings that could not be generated: row@attr backend count first-reason");
    for ((row, language), (count, why)) in &failures {
        eprintln!("{row:<16} {language:<7} {count:>4} {why}");
    }
}
