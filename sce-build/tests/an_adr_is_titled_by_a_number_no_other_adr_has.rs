// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A decision record is named by its number, and the number names one record.
//
// Every record under `docs/adr/` is a file `NNNN-<subject>.md` whose first line
// is its title, `# ADR NNNN — <decision>`. Code, diagnostics (a record's `spec`
// field reads `ADR 0146 D1`), other records and other repositories cite a
// record by that number alone, so the number is the record's identity: records
// are numbered in sequence and a number is never used again (Michael Nygard,
// "Documenting Architecture Decisions", 2011).
//
// Measured 2026-11-28: two numbers each named two records. 0146 named a formal
// judge and a bounded FIFO's full policy, 0148 a reactive turn and a message as
// a case of a union; each pair was written on two branches that took the next
// free number of the same main, and nothing refused the second record of either
// number when it landed. Two of the four were renumbered ADR 0157 and 0158.
//
// What this holds, over every record the repository holds (one written and not
// yet staged included, so the second record of a number is refused before it
// is committed):
//   - a Markdown file directly under `docs/adr/` is named `NNNN-<subject>.md`:
//     four digits, a hyphen and a subject;
//   - no two such files carry one number;
//   - the first line reads `# ADR NNNN` with the number of the file's name, so
//     a record renumbered in its name alone, or copied from another and not
//     retitled, is refused too.

mod common;

use std::collections::BTreeMap;
use std::fs;

/// The directory the records are kept in, relative to the repository root.
const RECORDS: &str = "docs/adr";

/// The fewest records the sweep must find: a sweep that finds none certifies
/// nothing. The directory held 158 records when this was written.
const MIN_RECORDS: usize = 150;

/// One record as the rule reads it: the name of its file and its first line.
struct Record {
    file_name: String,
    title: String,
}

/// The number the name of a record's file gives it: the four digits it starts
/// with, when the name is `NNNN-<subject>.md` and the subject is not empty.
fn number_of(file_name: &str) -> Option<&str> {
    let stem = file_name.strip_suffix(".md")?;
    let number = stem.get(..4)?;
    let subject = stem.get(4..)?.strip_prefix('-')?;
    (number.bytes().all(|b| b.is_ascii_digit()) && !subject.is_empty()).then_some(number)
}

/// Whether `title` is the title line of the record numbered `number`:
/// `# ADR <number>`, alone or followed by a space, so `# ADR 00150` is not the
/// title of record 0015.
fn titled(title: &str, number: &str) -> bool {
    title
        .strip_prefix("# ADR ")
        .and_then(|rest| rest.strip_prefix(number))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

/// Every way `records` break the rule, one line each: the names and titles in
/// the order of `records`, then each number more than one record carries.
fn problems(records: &[Record]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut by_number: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for record in records {
        let Some(number) = number_of(&record.file_name) else {
            problems.push(format!(
                "{RECORDS}/{}: a record is named NNNN-<subject>.md, four digits, a hyphen and \
                 its subject",
                record.file_name
            ));
            continue;
        };
        by_number
            .entry(number)
            .or_default()
            .push(record.file_name.as_str());
        if !titled(&record.title, number) {
            problems.push(format!(
                "{RECORDS}/{}: the first line is {:?}, not the title `# ADR {number}` its name \
                 gives it",
                record.file_name, record.title
            ));
        }
    }
    for (number, files) in &by_number {
        if files.len() > 1 {
            problems.push(format!(
                "ADR {number} names {} records: {}. A number names one record and is never used \
                 again: give every record of it but one the next free number, with a line saying \
                 which number it was first written under, and rewrite the references that mean it",
                files.len(),
                files.join(", ")
            ));
        }
    }
    problems
}

/// Every record the repository holds, tracked or written and not yet ignored,
/// with its first line, in the order of the names of their files.
fn records_in_the_tree() -> Vec<Record> {
    let root = common::repository::root();
    let prefix = format!("{RECORDS}/");
    let mut records: Vec<Record> = common::repository::paths_git_holds(&[RECORDS])
        .into_iter()
        .filter_map(|path| {
            let file_name = path.strip_prefix(prefix.as_str())?;
            if file_name.contains('/') || !file_name.ends_with(".md") {
                return None;
            }
            // A tracked record the working tree has deleted is listed until the
            // deletion is staged, and is no record of the tree being judged.
            let full = root.join(&path);
            if !full.is_file() {
                return None;
            }
            let text = fs::read_to_string(&full)
                .unwrap_or_else(|e| panic!("read {}: {e}", full.display()));
            Some(Record {
                file_name: file_name.to_string(),
                title: text.lines().next().unwrap_or("").to_string(),
            })
        })
        .collect();
    records.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    records.dedup_by(|a, b| a.file_name == b.file_name);
    records
}

#[test]
fn every_record_has_a_number_no_other_record_has_and_its_title_reads_it() {
    let records = records_in_the_tree();
    assert!(
        records.len() >= MIN_RECORDS,
        "found only {} record(s) under {RECORDS}: the enumeration is broken",
        records.len()
    );
    let found = problems(&records);
    assert!(
        found.is_empty(),
        "{} problem(s) with the numbers of the decision records:\n{}",
        found.len(),
        found.join("\n")
    );
}

fn record(file_name: &str, title: &str) -> Record {
    Record {
        file_name: file_name.to_string(),
        title: title.to_string(),
    }
}

#[test]
fn records_of_distinct_numbers_each_titled_by_its_own_pass() {
    let records = [
        record("0001-a-first-decision.md", "# ADR 0001 — A first decision"),
        record("0002-a-second-decision.md", "# ADR 0002"),
        record("0010-a-tenth-decision.md", "# ADR 0010 — A tenth decision"),
    ];
    assert_eq!(problems(&records), Vec::<String>::new());
}

#[test]
fn a_number_two_records_carry_is_refused_once_naming_both() {
    let records = [
        record("0146-a-bounded-fifo.md", "# ADR 0146 — A bounded FIFO"),
        record("0146-a-formal-judge.md", "# ADR 0146 — A formal judge"),
        record("0147-the-one-door.md", "# ADR 0147 — The one door"),
    ];
    let found = problems(&records);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with(
            "ADR 0146 names 2 records: 0146-a-bounded-fifo.md, 0146-a-formal-judge.md."
        ),
        "{found:?}"
    );
}

#[test]
fn a_title_that_reads_another_number_is_refused() {
    let records = [
        record("0157-a-bounded-fifo.md", "# ADR 0146 — A bounded FIFO"),
        record(
            "0015-a-short-number.md",
            "# ADR 00150 — A number that runs on",
        ),
        record(
            "0003-no-heading.md",
            "ADR 0003 — A title that is no heading",
        ),
        record("0004-empty.md", ""),
    ];
    let found = problems(&records);
    assert_eq!(found.len(), 4, "{found:?}");
    for (problem, name) in found.iter().zip([
        "0157-a-bounded-fifo.md",
        "0015-a-short-number.md",
        "0003-no-heading.md",
        "0004-empty.md",
    ]) {
        assert!(
            problem.starts_with(&format!("{RECORDS}/{name}: the first line is ")),
            "{problem}"
        );
    }
}

#[test]
fn a_file_not_named_by_a_number_and_a_subject_is_refused() {
    let records = [
        record("146-three-digits.md", "# ADR 146"),
        record("0146.md", "# ADR 0146"),
        record("0146-.md", "# ADR 0146"),
        record("01a6-not-a-number.md", "# ADR 01a6"),
        record("0146_an_underscore.md", "# ADR 0146"),
    ];
    let found = problems(&records);
    assert_eq!(found.len(), 5, "{found:?}");
    assert!(
        found
            .iter()
            .all(|p| p.contains(": a record is named NNNN-<subject>.md")),
        "{found:?}"
    );
}
