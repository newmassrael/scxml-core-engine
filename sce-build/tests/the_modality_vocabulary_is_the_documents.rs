// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Requirement-closure RFC ① — Atomic K ⑶: which words make a sentence
//! normative is a property of the document, not a constant.
//!
//! # ⭐ What this actually protects, which is not what it sounds like
//!
//! SCE never derives a requirement's modality — a manifest declares it.
//! So the verbal-form table has exactly ONE consumer in this tree: the
//! copyright guard, which refuses a manifest string that reads as
//! specification prose. A manifest is a checked-in file in a public
//! repository and specification sentences are usually somebody else's
//! copyright, so that guard is the thing standing between a paste and a
//! permanent publication.
//!
//! Before this, the guard knew four English words. A Korean-authored
//! manifest could therefore carry a copyrighted Korean requirement
//! sentence in a section title and load in silence — the guard was not
//! wrong about it, it could not see it at all. That is the hole these
//! tests close, and it is why the convention is worth having rather
//! than an abstraction for its own sake.
//!
//! # ⚠ A declared convention ADDS a vocabulary, never replaces one
//!
//! Measured over all 275 pages of the OEM standard this convention was
//! added for — read in place, never copied here — `shall` occurs **651**
//! times. The document is MIXED: Korean-normative for its first twenty
//! pages, where `shall` occurs zero times, and English-normative from
//! page 21 on. A guard that consulted only the declared table would have
//! gone blind to two thirds of the very document that motivated it.
//!
//! So the rule is directional: declaring a convention may only make the
//! guard see more. Its failure mode is publishing someone else's
//! sentence, which is permanent; its false-positive mode is a refused
//! heading, which is an inconvenience.
//!
//! # ⚠⚠ The table is measured, not intuited
//!
//! Korean carries obligation in the auxiliary ending rather than in a
//! verb, so the entries are endings. Counted in the same document:
//! `야 한다` 55, `야 함` 4, `야 하며` 1, `서는 안` 2, `지 않아야` 1,
//! `수 있다` 29, `권장` 1. A table of VERBS would have listed
//! `하여야 한다` (13) and missed `해야 한다` (20), the commoner form.

use sce_build::requirement_manifest::{prose_reason, ModalityConvention, RequirementManifest};

/// Strings that are normative prose in Korean, with what makes each so.
///
/// ⚠ These are constructed here, not lifted from the source document —
/// the guard under test exists to stop specification sentences being
/// committed, and a test that pasted one in to prove the point would
/// commit the thing it guards against.
const KOREAN_PROSE: &[(&str, &str)] = &[
    ("제어기는 요청을 처리하여야 한다", "obligation, -여야 한다"),
    ("제어기는 상태를 저장해야 한다", "obligation, -해야 한다"),
    (
        "응답을 전송하지 않아야 하는 경우",
        "prohibition, -지 않아야",
    ),
    ("진단 세션을 종료할 수 있다", "permission, -수 있다"),
    ("기본값 사용을 권장 하는 항목", "recommendation, 권장"),
];

/// Strings that are coordinates or labels, in Korean, which the guard
/// must NOT refuse.
///
/// The floor under a guard is that it still accepts what it should: a
/// rule that refused every Korean string would pass every test above
/// and make the format unusable for the documents it was added for.
const KOREAN_LABELS: &[&str] = &["진단 통신 사양", "네트워크 계층 개요", "부록 A 참조 문서"];

fn manifest_with(convention: &str, title: &str) -> String {
    format!(
        r#"{{ "doc_id": "d", "rev": "A",
              "extraction": {{ "ids": "native", "trace": "none",
                               "modality_convention": "{convention}",
                               "method": "hand" }},
              "sections": [{{ "id": "1", "title": "{title}" }}],
              "requirements": [{{ "id": "REQ-1", "section": "1" }}] }}"#
    )
}

/// Each convention has a non-empty set of prose its guard catches.
#[test]
fn each_convention_catches_prose_the_other_does_not_name() {
    const FLOOR_PER_CONVENTION: usize = 3;

    let english = "The controller shall discard the request";
    let mut caught_by_korean = 0usize;
    let mut missed: Vec<String> = Vec::new();
    for (prose, why) in KOREAN_PROSE {
        if prose_reason(prose, ModalityConvention::KoreanFormalEndings).is_some() {
            caught_by_korean += 1;
        } else {
            missed.push(format!("  {why}: {prose}"));
        }
    }
    assert!(
        missed.is_empty(),
        "the Korean convention did not see prose it names:\n{}\n\nA guard \
         that cannot see a document's own normative sentences is not \
         protecting that document's author from publishing them.",
        missed.join("\n"),
    );

    // ...and the English convention alone does NOT see them. This is
    // the half that shows the declaration changes the judgement rather
    // than decorating it.
    let seen_by_english_alone = KOREAN_PROSE
        .iter()
        .filter(|(prose, _)| prose_reason(prose, ModalityConvention::EnglishModalVerbs).is_some())
        .count();
    assert_eq!(
        seen_by_english_alone, 0,
        "the English convention reported Korean prose, so the two tables \
         are not actually distinct and the declaration decides nothing",
    );

    println!(
        "korean convention caught {caught_by_korean} prose string(s); english caught {seen_by_english_alone} of the same"
    );
    assert!(
        caught_by_korean >= FLOOR_PER_CONVENTION,
        "only {caught_by_korean} Korean prose string(s) examined, floor \
         {FLOOR_PER_CONVENTION}",
    );

    // The English convention's own non-empty case, so neither side of
    // the pair rests on the other's evidence.
    assert!(
        prose_reason(english, ModalityConvention::EnglishModalVerbs).is_some(),
        "the English convention must still catch English prose",
    );
}

/// ⭐ Declaring a convention only ever makes the guard see MORE.
///
/// The measurement behind this: the OEM standard that motivated the
/// Korean table carries 651 `shall`. Had the declaration replaced the
/// English table instead of adding to it, that document would have lost
/// its protection over the two thirds of its pages written in English.
#[test]
fn a_declared_convention_never_blinds_the_guard_to_the_other() {
    let english_prose = "Each request shall be answered within the timeout";
    let both: [ModalityConvention; 2] = [
        ModalityConvention::EnglishModalVerbs,
        ModalityConvention::KoreanFormalEndings,
    ];
    let mut checked = 0usize;
    for convention in both {
        assert!(
            prose_reason(english_prose, convention).is_some(),
            "declaring {convention:?} lost sight of English prose; a guard \
             against publishing somebody else's sentence may not get \
             weaker when a document says what language it is written in",
        );
        checked += 1;
    }
    println!("checked {checked} convention(s) against the never-weaken rule");
    assert_eq!(checked, both.len(), "every convention must be checked");
    assert!(checked >= 2, "only {checked} convention(s); floor 2");
}

/// The guard still accepts coordinates and labels under either
/// convention.
#[test]
fn a_label_is_not_prose_under_either_convention() {
    const FLOOR: usize = 3;
    let mut accepted = 0usize;
    let mut wrong: Vec<String> = Vec::new();
    for label in KOREAN_LABELS {
        for convention in [
            ModalityConvention::EnglishModalVerbs,
            ModalityConvention::KoreanFormalEndings,
        ] {
            match prose_reason(label, convention) {
                None => accepted += 1,
                Some(reason) => wrong.push(format!("  {convention:?} refused `{label}`: {reason}")),
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "a heading was refused as prose:\n{}\n\nA guard that refuses every \
         string in a language is not a guard, it is a ban on the language.",
        wrong.join("\n"),
    );
    println!("accepted {accepted} label(s) across both conventions");
    assert!(
        accepted >= FLOOR * 2,
        "only {accepted} acceptance(s) checked"
    );
}

/// End to end: a manifest declaring Korean refuses a Korean sentence in
/// a title, and the same manifest declaring English does not see it.
///
/// ⚠ This is the one that matters. The tests above exercise the
/// predicate; this drives the whole load path, which is where the
/// protection either exists or does not.
#[test]
fn the_declaration_reaches_the_load_path() {
    let sentence = "제어기는 요청을 처리하여야 한다";
    let refused =
        RequirementManifest::from_json(&manifest_with("korean-formal-endings", sentence), "korean");
    let err = refused
        .expect_err("a Korean manifest must refuse a Korean requirement sentence in a title")
        .to_string();
    assert!(
        err.contains("normative marker"),
        "the refusal must say what it saw; it said: {err}",
    );

    let unseen =
        RequirementManifest::from_json(&manifest_with("english-modal-verbs", sentence), "english");
    assert!(
        unseen.is_ok(),
        "declaring English must not catch Korean prose, or the two \
         conventions are not distinct at the load path either",
    );

    // And a label loads under Korean, so the refusal above is about the
    // sentence rather than about the language.
    assert!(
        RequirementManifest::from_json(
            &manifest_with("korean-formal-endings", "진단 통신 사양"),
            "korean_label",
        )
        .is_ok(),
        "a Korean heading must still load",
    );
    println!("drove 3 manifest(s) through the real load path");
}
