// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Applicability gate for the repair surface of the diagnostic wire.
//
// `SCE_ERROR_CONTRACT.md` makes two promises about `fix` that no
// other target checks. §2.2 says of `location.file`: "A consumer
// opens it to apply a fix." §3.1 says the deterministic variants
// "can be applied without further judgment". Both describe an
// operation — open the named file, perform the named edit — and
// every existing guard stops short of performing it:
//
//   * `diagnostic_corpus_schema` validates records against the
//     published JSON schema. The schema types `to` and `candidates`
//     as strings; a `replace_one_of` naming a value that appears
//     nowhere in the document validates perfectly.
//   * `diagnostic_goldens_are_byte_stable` pins hand-authored string
//     literals. A golden has no document behind it, so it cannot say
//     whether the coordinates it carries land on the offending token.
//   * `every_code_has_a_golden` proves coverage of the code enum, not
//     of the repair.
//
// So this target performs the edit. For every fix-bearing record the
// CLI emits over the tracked corpus it asks two questions:
//
//   1. **Can the consumer find the edit site?** When `location.line`
//      is present it must be the line `actual` occurs on — a record
//      pointing at the enclosing element sends the consumer to a line
//      where the token is absent. When `location.line` is absent the
//      token must be unambiguous in the document, because a
//      whole-file search is then the only locating strategy the wire
//      leaves and two hits make it a coin flip. ⚠ This question is put
//      to EVERY record carrying `actual`, fix or no fix: §2.1 makes
//      `actual` the observed value, and a record without a repair is
//      still read by a consumer looking for the token it names.
//
//   2. **Does the edit actually resolve the rejection?** For the
//      substitution variants the gate rewrites the document with the
//      proposed replacement and re-runs `check`. The record's `id`
//      must be gone. `id` rather than the code: a document can hold
//      several instances of one code, and only the repaired one is
//      claimed to disappear.
//
// The two questions are independent of each other and of the
// producer: question 1 reads the document, question 2 runs the CLI as
// a subprocess and compares wire ids. Neither reads the rule it
// judges — a producer that changes what it emits moves the observed
// records, never the oracle.
//
// Non-substitution variants (`add_attribute`, `add_one_of`,
// `remove_fields`, `rename_duplicate`) are held to question 1 only.
// Their payloads name an element and an attribute rather than a
// replacement for `actual`, so applying them mechanically means
// synthesising XML — the gate would be asserting its own serialiser,
// not the producer's proposal. `ROUNDTRIPS_PERFORMED_MIN` keeps the
// substitution half from silently emptying out.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use sce_build::forge::codegen_matrix::language_wire_name;
use sce_build::generator::Language;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Tracked `.scxml` documents, repo-relative.
///
/// Tracked rather than walked, for the same reason
/// `diagnostic_corpus_schema` gives: the fixture trees accumulate
/// generator output, and a document nobody committed is not part of
/// the corpus this gate speaks for.
fn tracked_documents() -> Vec<String> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "*.scxml"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

/// Lower bounds. Every per-record assertion is vacuously true over an
/// empty sweep, so the reach of the sweep is asserted rather than
/// assumed — the corpus, the records that carry a repair, and the
/// repairs actually replayed against the CLI.
const MIN_DOCUMENTS: usize = 600;
const MIN_FIX_BEARING_RECORDS: usize = 10;
const ROUNDTRIPS_PERFORMED_MIN: usize = 5;

/// One diagnostic record, reduced to the fields this gate judges.
#[derive(Debug, Clone)]
struct FixRecord {
    doc: String,
    id: String,
    code: String,
    lang: &'static str,
    actual: Option<String>,
    file: Option<String>,
    line: Option<usize>,
    /// Whether the record carries `expanded_from` — §2.3 exempts its
    /// `actual` from occurring on `location`'s row.
    expanded: bool,
    /// The fix's `kind`, or empty for a record with no `fix`.
    fix_kind: String,
    /// Replacement values, in the order the producer listed them.
    /// `replace_with` contributes its single `to`; `replace_one_of`
    /// contributes every candidate.
    replacements: Vec<String>,
    /// The record's `expected`, which §3.2 declares disjoint from
    /// `fix`.
    expected: Vec<String>,
    /// The record's `related` sites (§2.4).
    related: Vec<RelatedEntry>,
}

/// One `related` entry, reduced to what the locating rule reads.
#[derive(Debug, Clone)]
struct RelatedEntry {
    role: String,
    file: String,
    line: Option<usize>,
    actual: Option<String>,
}

/// A record naming the call site a preprocessor substituted from.
#[derive(Debug, Clone)]
struct ExpandedRecord {
    doc: String,
    code: String,
    lang: &'static str,
    actual: Option<String>,
    has_fix: bool,
    fix_kind: Option<String>,
    call_file: String,
    call_line: Option<usize>,
    /// Where the record says the value *is*, as opposed to where its
    /// parameters came from.
    file: Option<String>,
    line: Option<usize>,
}

impl ExpandedRecord {
    /// The record's expansion fields, or `None` when it names no call
    /// site.
    fn from_value(doc: &str, lang: &'static str, value: &serde_json::Value) -> Option<Self> {
        let from = value.get("expanded_from")?;
        Some(ExpandedRecord {
            doc: doc.to_string(),
            code: value
                .get("code")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string(),
            lang,
            actual: value
                .get("actual")
                .and_then(|a| a.as_str())
                .map(str::to_string),
            has_fix: value.get("fix").is_some(),
            fix_kind: value
                .get("fix")
                .and_then(|f| f.get("kind"))
                .and_then(|k| k.as_str())
                .map(str::to_string),
            call_file: from
                .get("file")
                .and_then(|f| f.as_str())
                .unwrap_or_default()
                .to_string(),
            call_line: from
                .get("line")
                .and_then(serde_json::Value::as_u64)
                .map(|n| n as usize),
            file: value
                .get("location")
                .and_then(|l| l.get("file"))
                .and_then(|f| f.as_str())
                .map(str::to_string),
            line: value
                .get("location")
                .and_then(|l| l.get("line"))
                .and_then(serde_json::Value::as_u64)
                .map(|n| n as usize),
        })
    }
}

/// Run `check` over one document in one backend and return every record
/// it wrote, parsed.
///
/// The one place this gate drives the CLI, so the corpus sweep and the
/// replay of a repair read the wire the same way.
fn check_records(root: &Path, doc: &str, lang: Language) -> Vec<serde_json::Value> {
    let out = Command::new(sce_codegen_bin())
        .arg("check")
        .arg(doc)
        .arg("-l")
        .arg(language_wire_name(lang))
        .arg("--error-format")
        .arg("json")
        .current_dir(root)
        .output()
        .expect("invoke sce-codegen check");
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Run `check` over one document in one backend and collect every
/// record, fix-bearing or not.
fn records_for(root: &Path, doc: &str, lang: Language) -> Vec<FixRecord> {
    let wire = language_wire_name(lang);
    check_records(root, doc, lang)
        .iter()
        .map(|value| FixRecord::from_value(doc, wire, value))
        .collect()
}

impl FixRecord {
    fn from_value(doc: &str, lang: &'static str, value: &serde_json::Value) -> Self {
        let fix = value.get("fix");
        let fix_kind = fix
            .and_then(|f| f.get("kind"))
            .and_then(|k| k.as_str())
            .unwrap_or_default()
            .to_string();
        let mut replacements = Vec::new();
        if let Some(to) = fix.and_then(|f| f.get("to")).and_then(|t| t.as_str()) {
            replacements.push(to.to_string());
        }
        if let Some(candidates) = fix
            .and_then(|f| f.get("candidates"))
            .and_then(|c| c.as_array())
        {
            for c in candidates {
                if let Some(s) = c.as_str() {
                    replacements.push(s.to_string());
                }
            }
        }
        FixRecord {
            doc: doc.to_string(),
            id: value
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or_default()
                .to_string(),
            code: value
                .get("code")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string(),
            lang,
            actual: value
                .get("actual")
                .and_then(|a| a.as_str())
                .map(str::to_string),
            file: value
                .get("location")
                .and_then(|l| l.get("file"))
                .and_then(|f| f.as_str())
                .map(str::to_string),
            line: value
                .get("location")
                .and_then(|l| l.get("line"))
                .and_then(serde_json::Value::as_u64)
                .map(|n| n as usize),
            expanded: value.get("expanded_from").is_some(),
            fix_kind,
            replacements,
            expected: value
                .get("expected")
                .and_then(|e| e.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            related: value
                .get("related")
                .and_then(|r| r.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| RelatedEntry {
                            role: entry
                                .get("role")
                                .and_then(|r| r.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            file: entry
                                .get("location")
                                .and_then(|l| l.get("file"))
                                .and_then(|f| f.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            line: entry
                                .get("location")
                                .and_then(|l| l.get("line"))
                                .and_then(serde_json::Value::as_u64)
                                .map(|n| n as usize),
                            actual: entry
                                .get("actual")
                                .and_then(|a| a.as_str())
                                .map(str::to_string),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// Every record `check` wrote over the tracked corpus, in every backend.
struct Corpus {
    documents: Vec<String>,
    /// One entry per `(document, backend)` run, in document order and,
    /// within a document, in [`Language::ALL`] order.
    runs: Vec<(String, Language, Vec<serde_json::Value>)>,
}

/// The corpus sweep, run once per test binary and shared by every test
/// in it.
///
/// The CLI runs once per `(document, backend)` pair — thousands of
/// processes — and sequentially that took over five minutes, paid again
/// by every test that swept, since nothing held the result. The pairs
/// are now spread over the machine's cores and the result is memoised.
/// Order is restored after the parallel run: a violation list that
/// reorders between runs reads as a change when nothing changed.
fn corpus() -> &'static Corpus {
    static CORPUS: OnceLock<Corpus> = OnceLock::new();
    CORPUS.get_or_init(|| {
        let root = repo_root();
        let documents = tracked_documents();
        let pairs: Vec<(usize, Language)> = (0..documents.len())
            .flat_map(|d| Language::ALL.iter().map(move |lang| (d, *lang)))
            .collect();
        let next = AtomicUsize::new(0);
        let workers = std::thread::available_parallelism().map_or(1, |n| n.get());
        let mut done: Vec<(usize, Vec<serde_json::Value>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut mine = Vec::new();
                        loop {
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            let Some(&(d, lang)) = pairs.get(i) else {
                                break;
                            };
                            mine.push((i, check_records(&root, &documents[d], lang)));
                        }
                        mine
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().expect("a sweep worker panicked"))
                .collect()
        });
        done.sort_by_key(|(i, _)| *i);
        assert_eq!(
            done.len(),
            pairs.len(),
            "the sweep ran {} of {} (document, backend) pairs",
            done.len(),
            pairs.len(),
        );
        let runs = done
            .into_iter()
            .map(|(i, values)| {
                let (d, lang) = pairs[i];
                (documents[d].clone(), lang, values)
            })
            .collect();
        Corpus { documents, runs }
    })
}

/// The swept corpus, as the records the applicability questions judge.
fn sweep() -> (&'static [String], Vec<FixRecord>) {
    let corpus = corpus();
    let records = corpus
        .runs
        .iter()
        .flat_map(|(doc, lang, values)| {
            let wire = language_wire_name(*lang);
            values
                .iter()
                .map(move |value| FixRecord::from_value(doc, wire, value))
        })
        .collect();
    (&corpus.documents, records)
}

/// Is `actual` a value a consumer can locate in the document it was
/// told to open?
///
/// Returns the violation text, or `None` when the record locates.
fn locating_violation(root: &Path, rec: &FixRecord) -> Option<String> {
    let Some(actual) = rec.actual.as_deref() else {
        // A substitution proposal with nothing to substitute names no
        // edit at all. Non-substitution variants legitimately omit it.
        return if rec.fix_kind == "replace_with" || rec.fix_kind == "replace_one_of" {
            Some(format!(
                "[{} / {}] {} carries fix={} but no `actual` — \
                 SCE_ERROR_CONTRACT.md §3.1 defines both variants as a \
                 replacement *of* `actual`, so the consumer is handed a \
                 replacement with no site.",
                rec.doc, rec.lang, rec.code, rec.fix_kind,
            ))
        } else {
            None
        };
    };
    let named = rec.file.as_deref().unwrap_or(&rec.doc);
    let site = format!("[{} / {}] {}", rec.doc, rec.lang, rec.code);
    token_violation(root, &site, named, rec.line, actual)
}

/// Is `actual` findable in `named` — on `line` when there is one,
/// exactly once in the file when there is not? The rule a record's own
/// `actual` and each `related` entry's are both held to (§3.1.1, §2.4).
fn token_violation(
    root: &Path,
    site: &str,
    named: &str,
    line: Option<usize>,
    actual: &str,
) -> Option<String> {
    let path = root.join(named);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Some(format!(
            "{site} names location.file={named} which does not \
             open — §2.2: \"A consumer opens it to apply a fix\".",
        ));
    };
    let lines: Vec<&str> = text.lines().collect();
    match line {
        Some(line) => {
            let Some(source_line) = lines.get(line - 1) else {
                return Some(format!(
                    "{site} points at line {line} of {named}, which has {} lines.",
                    lines.len(),
                ));
            };
            if !source_line.contains(actual) {
                return Some(format!(
                    "{site} carries actual={actual:?} and points at \
                     {named}:{line}, but that line does not contain it:\n    \
                     {}\n  The consumer edits the line it was given, so a \
                     coordinate on the enclosing element repairs the wrong \
                     token (or nothing).",
                    source_line.trim(),
                ));
            }
            None
        }
        None => {
            let hits = text.matches(actual).count();
            if hits == 1 {
                return None;
            }
            Some(format!(
                "{site} carries actual={actual:?} with no location.line, and \
                 that token occurs {hits} time(s) in {named}. Without a line \
                 the whole-file search is the only locating strategy the wire \
                 offers, and it is {} here.",
                if hits == 0 {
                    "a miss — the value is not in the file at all"
                } else {
                    "ambiguous"
                },
            ))
        }
    }
}

/// Each `related` entry that names a token, held to the rule the
/// record's own `actual` is.
fn related_violations(root: &Path, rec: &FixRecord) -> Vec<String> {
    rec.related
        .iter()
        .filter_map(|entry| {
            let actual = entry.actual.as_deref()?;
            let site = format!(
                "[{} / {}] {} related[{}]",
                rec.doc, rec.lang, rec.code, entry.role
            );
            token_violation(root, &site, &entry.file, entry.line, actual)
        })
        .collect()
}

/// Every `actual` is a token the consumer can find where `location`
/// says it is — on a fix-bearing record, the edit site; on every other,
/// the observed value itself.
///
/// ⚠ This held only fix-bearing records, and `actual` without a fix
/// was free to be prose: `carrier at index 1, embed at index 0`,
/// `8 < 16`, `supported only as a direct <transition> child`. §2.1
/// calls `actual` "the observed value", and a sentence about a relation
/// is not one — a consumer searching the document for it finds nothing
/// (measured 2026-09-21). A record under `expanded_from` is exempt, as
/// §2.3 says: its `location` row holds the template's shape, not the
/// assembled value.
#[test]
fn every_actual_names_a_site_the_consumer_can_locate() {
    let root = repo_root();
    let (documents, records) = sweep();

    assert!(
        documents.len() >= MIN_DOCUMENTS,
        "corpus holds only {} documents; expected at least \
         {MIN_DOCUMENTS}. A sweep over nothing certifies nothing.",
        documents.len(),
    );
    let fix_bearing = records.iter().filter(|r| !r.fix_kind.is_empty()).count();
    assert!(
        fix_bearing >= MIN_FIX_BEARING_RECORDS,
        "sweep collected only {fix_bearing} fix-bearing records; expected at \
         least {MIN_FIX_BEARING_RECORDS}. Either the corpus stopped \
         exercising the repair surface or the producer stopped \
         emitting `fix`.",
    );

    let judged: Vec<&FixRecord> = records
        .iter()
        .filter(|r| {
            !r.fix_kind.is_empty() || (r.actual.is_some() && !r.expanded) || !r.related.is_empty()
        })
        .collect();
    let violations: Vec<String> = judged
        .iter()
        .flat_map(|rec| {
            let own = if !rec.fix_kind.is_empty() || !rec.expanded {
                locating_violation(&root, rec)
            } else {
                None
            };
            own.into_iter().chain(related_violations(&root, rec))
        })
        .collect();

    assert!(
        violations.is_empty(),
        "{} of {} records carrying `actual`, `fix` or `related` name a site \
         the consumer cannot find:\n{}",
        violations.len(),
        judged.len(),
        violations.join("\n"),
    );
}

/// Applying a substitution proposal makes its diagnostic go away.
///
/// The strongest available statement about a repair signal: not that
/// it is well-formed, not that it points somewhere, but that
/// performing it satisfies the constraint that produced it.
#[test]
fn applying_a_substitution_clears_the_diagnostic_that_proposed_it() {
    let root = repo_root();
    let (_documents, records) = sweep();

    // One document may hold several repairable records; each is
    // replayed against its own pristine copy of the tree so the
    // repairs cannot mask one another.
    let substitutions: Vec<&FixRecord> = records
        .iter()
        .filter(|r| r.fix_kind == "replace_with" || r.fix_kind == "replace_one_of")
        .filter(|r| r.actual.is_some() && !r.replacements.is_empty())
        .collect();

    let mut performed = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for rec in substitutions {
        let actual = rec.actual.as_deref().expect("filtered above");
        let named = rec.file.clone().unwrap_or_else(|| rec.doc.clone());
        // The replacement the consumer would reach for first.
        let replacement = &rec.replacements[0];

        let temp = tempfile::tempdir().expect("tempdir");
        let staged = match stage_document_tree(&root, &named, temp.path()) {
            Ok(p) => p,
            Err(e) => {
                violations.push(format!(
                    "[{} / {}] {} could not be staged for replay: {e}",
                    rec.doc, rec.lang, rec.code,
                ));
                continue;
            }
        };

        // Baseline: the record must reproduce from the staged copy,
        // otherwise the replay says nothing about the repair.
        let before = records_for(temp.path(), &staged, language_of(rec.lang));
        let before_ids: BTreeSet<&str> = before.iter().map(|r| r.id.as_str()).collect();
        if !before_ids.contains(rec.id.as_str()) {
            violations.push(format!(
                "[{} / {}] {} (id {}) does not reproduce from a staged \
                 copy of its own directory — the rejection depends on \
                 state outside the document's directory, which is also \
                 state the consumer holding this record does not have.",
                rec.doc, rec.lang, rec.code, rec.id,
            ));
            continue;
        }

        let staged_path = temp.path().join(&staged);
        let text = std::fs::read_to_string(&staged_path).expect("staged document reads");
        let repaired = apply_substitution(&text, actual, replacement, rec.line);
        std::fs::write(&staged_path, &repaired).expect("staged document writes");

        let after = records_for(temp.path(), &staged, language_of(rec.lang));
        let after_ids: BTreeSet<&str> = after.iter().map(|r| r.id.as_str()).collect();
        performed += 1;
        if after_ids.contains(rec.id.as_str()) {
            violations.push(format!(
                "[{} / {}] {} still fires after its own repair was \
                 applied ({actual:?} → {replacement:?}). The proposal is \
                 on the wire, the consumer performed it, and the \
                 rejection stands.",
                rec.doc, rec.lang, rec.code,
            ));
        }
    }

    assert!(
        performed >= ROUNDTRIPS_PERFORMED_MIN,
        "only {performed} substitution repairs were replayed; expected \
         at least {ROUNDTRIPS_PERFORMED_MIN}. A replay set this small \
         cannot speak for the repair surface.",
    );
    assert!(
        violations.is_empty(),
        "{} substitution repairs did not clear their own diagnostic:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

/// Wire name back to the enum, so the replay re-runs the backend the
/// record came from.
fn language_of(wire: &str) -> Language {
    *Language::ALL
        .iter()
        .find(|l| language_wire_name(**l) == wire)
        .expect("wire name came from language_wire_name")
}

/// Copy the document's directory (recursively) into `dest`,
/// preserving the repo-relative path so relative `<sce:import>` /
/// `<xi:include>` hrefs resolve exactly as they do in the tree.
///
/// Returns the repo-relative path of the staged document.
fn stage_document_tree(root: &Path, rel: &str, dest: &Path) -> std::io::Result<String> {
    let rel_path = Path::new(rel);
    let dir = rel_path.parent().unwrap_or(Path::new(""));
    copy_dir(&root.join(dir), &dest.join(dir))?;
    Ok(rel.to_string())
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Perform the substitution the way a consumer holding only the wire
/// record would: on the named line when the record gives one,
/// otherwise on the document's single occurrence.
fn apply_substitution(text: &str, actual: &str, replacement: &str, line: Option<usize>) -> String {
    match line {
        Some(n) => {
            let mut out: Vec<String> = Vec::new();
            for (i, l) in text.lines().enumerate() {
                if i + 1 == n {
                    out.push(l.replacen(actual, replacement, 1));
                } else {
                    out.push(l.to_string());
                }
            }
            let mut joined = out.join("\n");
            if text.ends_with('\n') {
                joined.push('\n');
            }
            joined
        }
        None => text.replacen(actual, replacement, 1),
    }
}

/// Lower bound for the expansion half. The corpus carries template
/// parity fixtures whose references resolve only after `<sce:use>`
/// substitution, so this sweep reaching nothing means the producer
/// stopped emitting the field, not that the documents changed.
const MIN_EXPANDED_RECORDS: usize = 6;

/// A record whose value a preprocessor synthesised names the call site
/// that chose it, and proposes no substitution.
///
/// The two halves are what make the record usable. Without the call
/// site the consumer has `location` pointing at a template row that
/// does not contain `actual` and no way to learn why. Without the
/// suppression it also has a `replace_one_of` naming candidates for a
/// value that is not there — the shape
/// `every_actual_names_a_site_the_consumer_can_locate` rejects, and the
/// reason this record is exempt from it.
///
/// The oracle is independent of the producer twice over: the call
/// coordinate is checked against the *document's* text (the row it
/// names must actually open an `<sce:use>`), and the absence of a
/// substitution `fix` is read off the emitted JSON.
#[test]
fn a_synthesised_value_names_the_call_site_that_chose_it() {
    let root = repo_root();
    let records: Vec<ExpandedRecord> = corpus()
        .runs
        .iter()
        .flat_map(|(doc, lang, values)| {
            let wire = language_wire_name(*lang);
            values
                .iter()
                .filter_map(move |value| ExpandedRecord::from_value(doc, wire, value))
        })
        .collect();

    assert!(
        records.len() >= MIN_EXPANDED_RECORDS,
        "sweep found only {} records carrying `expanded_from`; expected \
         at least {MIN_EXPANDED_RECORDS}.",
        records.len(),
    );

    let mut violations: Vec<String> = Vec::new();
    for rec in &records {
        let Ok(text) = std::fs::read_to_string(root.join(&rec.call_file)) else {
            violations.push(format!(
                "[{} / {}] {} names expanded_from.file={} which does not \
                 open.",
                rec.doc, rec.lang, rec.code, rec.call_file,
            ));
            continue;
        };
        match rec.call_line {
            Some(line) => match text.lines().nth(line - 1) {
                Some(source) if source.contains("<sce:use") => {}
                Some(source) => violations.push(format!(
                    "[{} / {}] {} names a call site at {}:{line}, but that \
                     line opens no `<sce:use>`:\n    {}",
                    rec.doc,
                    rec.lang,
                    rec.code,
                    rec.call_file,
                    source.trim(),
                )),
                None => violations.push(format!(
                    "[{} / {}] {} names a call site at {}:{line}, past the \
                     end of that file.",
                    rec.doc, rec.lang, rec.code, rec.call_file,
                )),
            },
            None => violations.push(format!(
                "[{} / {}] {} carries `expanded_from` with no line — the \
                 call site is the coordinate that tells one expansion from \
                 another, so a file alone does not distinguish them.",
                rec.doc, rec.lang, rec.code,
            )),
        }
        // `location` must have been resolved out of expanded
        // coordinates too. Suppressing the `fix`, and carrying
        // `expanded_from`, takes this record out of
        // `every_actual_names_a_site_the_consumer_can_locate`'s reach,
        // so without this check a producer that stopped resolving
        // rows would keep a green board while every one of these
        // records pointed at a row of the wrong file — the mutation
        // "recorded rows reach the wire in expanded coordinates"
        // survived exactly that way.
        //
        // The discriminator is that the two coordinates name different
        // files: `<sce:use template="X">` splices from another
        // document, so a record whose value came from a template body
        // cannot have been authored in the file that called it.
        match rec.file.as_deref() {
            Some(file) if file == rec.call_file => violations.push(format!(
                "[{} / {}] {} puts `location` and `expanded_from` in the \
                 same file ({file}). A substituted value lives in the \
                 template body and is called from elsewhere, so one of \
                 the two is unresolved — most likely `location` is still \
                 in expanded coordinates.",
                rec.doc, rec.lang, rec.code,
            )),
            Some(file) => {
                // And the row it names must exist in that file.
                if let (Ok(text), Some(line)) = (std::fs::read_to_string(root.join(file)), rec.line)
                {
                    let count = text.lines().count();
                    if line > count {
                        violations.push(format!(
                            "[{} / {}] {} points at {file}:{line}, which has \
                             {count} lines.",
                            rec.doc, rec.lang, rec.code,
                        ));
                    }
                }
            }
            None => violations.push(format!(
                "[{} / {}] {} carries `expanded_from` but no `location` — \
                 the call site explains a coordinate the record does not \
                 have.",
                rec.doc, rec.lang, rec.code,
            )),
        }
        if rec.has_fix
            && matches!(
                rec.fix_kind.as_deref(),
                Some("replace_with") | Some("replace_one_of")
            )
        {
            violations.push(format!(
                "[{} / {}] {} proposes {} against the synthesised value \
                 {:?}. That value is not in the row `location` names, and \
                 substituting into the row that *is* named rewrites every \
                 expansion of the template rather than this one.",
                rec.doc,
                rec.lang,
                rec.code,
                rec.fix_kind.as_deref().unwrap_or("a substitution"),
                rec.actual.as_deref().unwrap_or("<none>"),
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "{} of {} expansion records are unusable:\n{}",
        violations.len(),
        records.len(),
        violations.join("\n"),
    );
}

/// `expected` and `fix` never say the same thing (§3.2).
///
/// The contract makes the two fields disjoint and says why: a consumer
/// after a repair signal reads `fix`, a consumer after the producer's
/// grammatical expectation reads `expected`, and "the candidate list
/// is never duplicated across both fields". A record carrying one list
/// twice leaves a consumer unable to tell which field it is supposed
/// to be dispatching on — and, worse, teaches it that reading either
/// works, which stops being true at the first code that populates only
/// one.
///
/// The published schema cannot enforce this: both fields are legal and
/// independently typed, so every duplicate validates.
#[test]
fn expected_and_fix_never_carry_the_same_list() {
    let (_documents, records) = sweep();
    let violations: Vec<String> = records
        .iter()
        .filter(|r| !r.expected.is_empty() && !r.replacements.is_empty())
        .filter(|r| {
            let expected: BTreeSet<&str> = r.expected.iter().map(String::as_str).collect();
            let offered: BTreeSet<&str> = r.replacements.iter().map(String::as_str).collect();
            !expected.is_disjoint(&offered)
        })
        .map(|r| {
            format!(
                "[{} / {}] {} carries expected={:?} and fix.{} offering \
                 {:?} — SCE_ERROR_CONTRACT.md §3.2 makes the two fields \
                 disjoint, and the candidate list belongs to `fix`.",
                r.doc, r.lang, r.code, r.expected, r.fix_kind, r.replacements,
            )
        })
        .collect();

    assert!(
        violations.is_empty(),
        "{} records duplicate a list across `expected` and `fix`:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

/// The corpus reaches more than one repair variant.
///
/// A sweep that only ever saw `replace_one_of` would satisfy both
/// tests above while saying nothing about the rest of §3.1, so the
/// spread is asserted directly.
#[test]
fn the_sweep_reaches_more_than_one_fix_variant() {
    let (_documents, records) = sweep();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for rec in records.iter().filter(|r| !r.fix_kind.is_empty()) {
        *by_kind.entry(rec.fix_kind.clone()).or_default() += 1;
    }
    assert!(
        by_kind.len() >= 2,
        "the corpus exercises only {:?}. SCE_ERROR_CONTRACT.md §3.1 \
         defines six variants; a gate that sees one of them speaks for \
         one of them.",
        by_kind,
    );
}
