// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A casefile header that counts its cases is counted back.
//
// Twelve casefiles make a promise about the cases below them, and the promises
// are prose: nothing in the harness reads them, and a round is green whatever
// the header says, because every case was CAUGHT. Hand-checking all twelve
// found one FALSE and one incomplete, and both came out of the same shape — a
// sentence that states a NUMBER. Headers that say "the cases below both attack
// it" without a count were all true. So the number is the part a machine can
// hold, and this is the machine that holds it.
//
// It cannot judge the rest, and that limit is the point rather than an excuse.
// `event_payload_is_read_not_run` was false because it pointed at cases by
// POSITION — "the last two cases below" — and two unrelated cases were appended
// after it the same day. No scan can tell which cases a sentence MEANS. What a
// scan can tell is that the arithmetic is wrong, and that is what went wrong
// here every time.
//
// Four traps this repository has already paid for, avoided by construction:
//
// COMMENTS ARE THE SUBJECT, so they cannot merely be stripped. The usual rule
// here is to remove comments before scanning; a casefile header IS a comment,
// so instead the header is delimited precisely — every comment line before the
// first `mutation_case` — and per-case comments are excluded. That boundary is
// not the first declaration: `c11_datamodel_reader` and
// `parallel_region_transition_domain` both count their cases in prose that sits
// BELOW `mutation_targets`.
//
// HEADER PROSE WRAPS, so the lines are joined before matching.
// `empty_finalize_is_not_an_absent_one` wraps "the" and "three cases" across
// two lines, and a per-line regex saw "three cases," with no article and
// matched nothing — a miss that reads exactly like a clean file.
//
// A HEADER MAY QUOTE ITS OWN RETIRED CLAIM. `event_payload_is_read_not_run`
// records the correction it received, quoting the false sentence verbatim, so a
// scan that read quoted text as a live claim would fail the one file that got
// this right. Double-quoted spans are blanked first.
//
// WHAT A SCAN DECLINES MUST BE COUNTED, or the count it prints reads as
// coverage. This gate shipped its first version reporting "1 out of scope" — the
// one number too large to be a case count — while silently ignoring every
// phrasing its patterns did not happen to spell. A wider net (any number word
// within six tokens of "case"/"below") found 66 such phrasings and showed 53
// being declined without a word, five of them checkable. Those five are patterns
// now, and the remainder is reported as a number rather than implied to be zero.
//
// And there is a FLOOR. A source scan whose population goes empty reports zero
// violations, which reads like a clean corpus.
//
// ⚠ One widening had to be walked back, which is the shape to expect: reading a
// bare "Both cases" as "this file has two" reported a defect that was not there,
// because `event_data_xml_readings_ctest` says "two cases here read ..." and
// then "Both cases still applied" ABOUT THOSE TWO, in a file of six. The
// deictic — "both cases BELOW", "both cases HERE" — is what makes it a claim
// about the file, and that was measured rather than reasoned.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Spelled-out counts. Prose in this corpus writes a case count as a word.
const WORDS: [(&str, usize); 16] = [
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("sixteen", 16),
];

/// The largest number this gate will accept as a claim about a casefile's own
/// cases. The biggest casefile in the corpus holds eighteen, so a bigger number
/// in a header is prose about something else — `kotlin_engine_selection` says a
/// misspelt engine "passed all 226 cases", meaning the W3C generated machines.
/// Above this the sentence is reported out of scope WITH that reason rather than
/// dropped, because a silent exclusion reads like coverage.
const PLAUSIBLE_MAX: usize = 30;

/// What a matched sentence claims.
#[derive(Debug, PartialEq)]
enum Claim {
    /// The header says how many cases the file has: must be exact.
    Total(usize),
    /// The header names a subset or a run of cases: must not exceed the count.
    Bound(usize),
}

struct Finding {
    file: String,
    fragment: String,
    claim: Claim,
    cases: usize,
}

/// Every comment line before the first `mutation_case`, joined, with
/// double-quoted spans blanked.
fn header(body: &str) -> String {
    let mut out = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("mutation_case ") {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix('#') {
            out.push(' ');
            out.push_str(rest.trim());
        }
    }
    // Blank the quoted spans. Done on the joined text so a quote that wraps
    // across two header lines is still one span.
    let mut cleaned = String::with_capacity(out.len());
    let mut inside = false;
    for ch in out.chars() {
        if ch == '"' {
            inside = !inside;
            cleaned.push('"');
        } else if inside {
            cleaned.push(' ');
        } else {
            cleaned.push(ch);
        }
    }
    cleaned
}

fn case_count(body: &str) -> usize {
    body.lines()
        .filter(|l| l.starts_with("mutation_case "))
        .count()
}

/// Positions, for the headers that point at a case by its place in the file.
const ORDINALS: [(&str, usize); 10] = [
    ("first", 1),
    ("second", 2),
    ("third", 3),
    ("fourth", 4),
    ("fifth", 5),
    ("sixth", 6),
    ("seventh", 7),
    ("eighth", 8),
    ("ninth", 9),
    ("tenth", 10),
];

fn ordinal_at(words: &[&str], at: usize) -> Option<usize> {
    let token = words.get(at)?.trim_matches(|c: char| !c.is_alphanumeric());
    let lower = token.to_ascii_lowercase();
    ORDINALS.iter().find(|(w, _)| *w == lower).map(|(_, n)| *n)
}

/// Read a count immediately following `at` in `words`, as a word or as digits.
fn count_at(words: &[&str], at: usize) -> Option<usize> {
    let token = words.get(at)?.trim_matches(|c: char| !c.is_alphanumeric());
    let lower = token.to_ascii_lowercase();
    if let Some((_, n)) = WORDS.iter().find(|(w, _)| *w == lower) {
        return Some(*n);
    }
    lower.parse::<usize>().ok()
}

/// The phrasings this gate understands, matched over the joined header's words.
///
/// Deliberately a short, literal list rather than one clever pattern: every
/// entry here was read out of a real header, and a phrasing nobody writes is a
/// rule nobody can violate.
fn claims(
    header: &str,
    cases: usize,
    file: &str,
    out: &mut Vec<Finding>,
    skipped: &mut Vec<String>,
) -> usize {
    let words: Vec<&str> = header.split_whitespace().collect();
    let lower: Vec<String> = words.iter().map(|w| w.to_ascii_lowercase()).collect();
    let bare = |i: usize| -> String {
        lower
            .get(i)
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .unwrap_or_default()
    };
    // Which tokens a claim consumed, so the residue below is what the patterns
    // genuinely declined rather than everything they did not happen to start on.
    let mut claimed = vec![false; words.len()];
    let take = |from: usize, to: usize, claimed: &mut Vec<bool>| {
        for slot in claimed
            .iter_mut()
            .take((to + 1).min(words.len()))
            .skip(from)
        {
            *slot = true;
        }
    };

    for i in 0..words.len() {
        // "<n> of the <m> cases"  — the second number is the total.
        if bare(i) == "of" && bare(i + 1) == "the" && bare(i + 3).starts_with("case") {
            if let Some(n) = count_at(&words, i + 2) {
                push(n, Claim::Total(n), i, &words, cases, file, out, skipped);
                take(i, i + 3, &mut claimed);
                continue;
            }
        }
        // "<n> of <m> cases", no article — the same claim, and the phrasing a
        // header reaches for when it reports a past round: "SURVIVED two of
        // three cases".
        if bare(i + 1) == "of" && bare(i + 3).starts_with("case") && bare(i + 2) != "the" {
            if let (Some(_), Some(m)) = (count_at(&words, i), count_at(&words, i + 2)) {
                push(m, Claim::Total(m), i, &words, cases, file, out, skipped);
                take(i, i + 3, &mut claimed);
                continue;
            }
        }
        // "all <n> cases" / "the <n> cases" / "across the <n> cases"
        if (bare(i) == "all" || bare(i) == "the") && bare(i + 2).starts_with("case") {
            if let Some(n) = count_at(&words, i + 1) {
                push(n, Claim::Total(n), i, &words, cases, file, out, skipped);
                take(i, i + 2, &mut claimed);
                continue;
            }
        }
        // "the <n> below" — a count of what follows, so also a total.
        if bare(i) == "the" && bare(i + 2) == "below" {
            if let Some(n) = count_at(&words, i + 1) {
                push(n, Claim::Total(n), i, &words, cases, file, out, skipped);
                take(i, i + 2, &mut claimed);
                continue;
            }
        }
        // "both cases below" / "both cases here" — a total of two.
        //
        // ⚠ The deictic is load-bearing and was measured, not guessed. A BARE
        // "Both cases" refers to a subset the sentence before it just named:
        // `event_data_xml_readings_ctest` says "two cases here read ..." and
        // then "Both cases still applied", in a file of SIX. Reading that as a
        // total made this gate report a defect that was not there.
        if bare(i) == "both"
            && bare(i + 1).starts_with("case")
            && (bare(i + 2) == "below" || bare(i + 2) == "here")
        {
            push(2, Claim::Total(2), i, &words, cases, file, out, skipped);
            take(i, i + 2, &mut claimed);
            continue;
        }
        // "the last <n> cases" — a run, so only bounded.
        if bare(i) == "last" && bare(i + 2).starts_with("case") {
            if let Some(n) = count_at(&words, i + 1) {
                push(n, Claim::Bound(n), i, &words, cases, file, out, skipped);
                take(i, i + 2, &mut claimed);
                continue;
            }
        }
        // "the <ordinal> one below" — a position, so bounded by the count.
        if bare(i) == "the" && bare(i + 2) == "one" && bare(i + 3) == "below" {
            if let Some(n) = ordinal_at(&words, i + 1) {
                push(n, Claim::Bound(n), i, &words, cases, file, out, skipped);
                take(i, i + 3, &mut claimed);
                continue;
            }
        }
        // "<n> cases here" — a subset of this file's cases, so bounded.
        if bare(i + 1).starts_with("case") && bare(i + 2) == "here" {
            if let Some(n) = count_at(&words, i) {
                push(n, Claim::Bound(n), i, &words, cases, file, out, skipped);
                take(i, i + 2, &mut claimed);
                continue;
            }
        }
        // "<n> of the cases" — a subset, so only bounded.
        if bare(i + 1) == "of" && bare(i + 2) == "the" && bare(i + 3).starts_with("case") {
            if let Some(n) = count_at(&words, i) {
                push(n, Claim::Bound(n), i, &words, cases, file, out, skipped);
                take(i, i + 3, &mut claimed);
            }
        }
    }

    // The residue: a number-ish word with `case`/`cases`/`below` within six
    // tokens that no pattern above consumed.
    //
    // Reported because the alternative is the hole this gate nearly shipped
    // with. It printed "1 out of scope" — the one number too large to be a case
    // count — which reads as though everything else had been accounted for. A
    // wider net found 66 number-near-case phrasings and showed the patterns were
    // then declining 53 of them in silence, five of which were checkable. A
    // count that is honest about what it declines is what lets the next reader
    // decide whether a new phrasing is worth a pattern.
    let mut residue = 0;
    for (i, taken) in claimed.iter().enumerate() {
        if *taken {
            continue;
        }
        let numberish =
            count_at(&words, i).is_some() || ordinal_at(&words, i).is_some() || bare(i) == "both";
        if !numberish {
            continue;
        }
        let near = (i + 1..(i + 7).min(words.len()))
            .any(|j| bare(j).starts_with("case") || bare(j) == "below");
        if near {
            residue += 1;
        }
    }
    residue
}

#[allow(clippy::too_many_arguments)]
fn push(
    n: usize,
    claim: Claim,
    at: usize,
    words: &[&str],
    cases: usize,
    file: &str,
    out: &mut Vec<Finding>,
    skipped: &mut Vec<String>,
) {
    let lo = at.saturating_sub(1);
    let hi = (at + 5).min(words.len());
    let fragment = words[lo..hi].join(" ");
    if n > PLAUSIBLE_MAX {
        skipped.push(format!(
            "{file}: {n} is larger than any casefile's case count, so \
             {fragment:?} counts something else"
        ));
        return;
    }
    out.push(Finding {
        file: file.to_string(),
        fragment,
        claim,
        cases,
    });
}

fn scan() -> (Vec<Finding>, Vec<String>, usize, usize) {
    let dir = repo_root().join("sce-build/tests/mutations");
    let mut findings = Vec::new();
    let mut skipped = Vec::new();
    let mut scanned = 0;
    let mut residue = 0;
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "cases"))
        .collect();
    paths.sort();
    for path in paths {
        let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let name = path
            .file_name()
            .expect("a file name")
            .to_string_lossy()
            .into_owned();
        scanned += 1;
        let cases = case_count(&body);
        residue += claims(&header(&body), cases, &name, &mut findings, &mut skipped);
    }
    (findings, skipped, scanned, residue)
}

#[test]
fn every_counting_header_agrees_with_its_case_count() {
    let (findings, skipped, scanned, residue) = scan();

    // The floor, before any verdict. A scan of nothing violates nothing.
    assert!(
        scanned > 50,
        "only {scanned} casefiles were scanned — this gate has no population \
         and would pass on anything"
    );
    let totals = findings
        .iter()
        .filter(|f| matches!(f.claim, Claim::Total(_)))
        .count();
    assert!(
        totals >= 3,
        "only {totals} header(s) were found to state a total case count, out of \
         {scanned} casefiles. The phrasings this gate understands have gone out \
         of use, so it is now checking almost nothing — widen them rather than \
         leaving it green"
    );

    let mut wrong = Vec::new();
    for f in &findings {
        match f.claim {
            Claim::Total(n) if n != f.cases => wrong.push(format!(
                "{}: header says {} but the file holds {} case(s) — {:?}",
                f.file, n, f.cases, f.fragment
            )),
            Claim::Bound(n) if n > f.cases => wrong.push(format!(
                "{}: header names {} case(s) out of only {} — {:?}",
                f.file, n, f.cases, f.fragment
            )),
            _ => {}
        }
    }

    // Printed on success too: the out-of-scope set is what a reader has to
    // weigh against the checked set, and a gate that hides it reports coverage
    // it does not have.
    println!(
        "{scanned} casefile(s): {} counting header claim(s) checked ({totals} total, \
         {} bounded); {} refused by the plausibility ceiling; {residue} further \
         number-near-case phrasing(s) the patterns decline",
        findings.len(),
        findings.len() - totals,
        skipped.len()
    );
    for reason in &skipped {
        println!("  refused — {reason}");
    }
    println!(
        "  the declined set was read once, on 2026-08-26, and was incidental \
         proximity (\"one out from under a case\", \"six such cases took forty-five \
         minutes\"). A NEW phrasing here is a candidate for a pattern, not proof \
         of coverage."
    );

    assert!(
        wrong.is_empty(),
        "a casefile header counts its cases wrongly. Nothing in the harness \
         reads prose, so a round stays green while the header sends the next \
         reader to cases that are not there:\n  {}",
        wrong.join("\n  ")
    );
}
