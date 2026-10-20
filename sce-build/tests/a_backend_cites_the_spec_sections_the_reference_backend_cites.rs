// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every W3C SCXML section the reference backend's code cites, every other
// backend's code cites too, or the difference is written down here with the
// reason.
//
// The Go runtime says of itself that it is a port of the Rust one and that
// "Rust wins" where they differ (backends/go/runtime/doc.go), and the Kotlin
// and Python runtimes were written against the same reference. A conformance
// suite shows that a port behaves; it does not show that the port has a part
// for each part of the algorithm, because a fixture nobody wrote passes
// everywhere. The citations do show it. Each part of the algorithm cites the
// section it implements, the ledger validates every citation in the
// directories it enrols, and so the set of sections a backend cites is the
// outline of what it implemented. A section the reference cites and a port
// does not is a part the port is missing, or a part it has and does not say
// it has; either way it is not yet the same structure.
//
// A backend's code is its runtime and its templates: the C11 backend writes
// most of its algorithm into the code it generates, and a comparison of
// runtimes alone measured it at 19 sections against the reference's 65. Its
// runtime directories are the ones the ledger enrols inside the backend's
// tree (`Language::runtime_tree`); its templates are the ones it writes
// (`Language::authors_template`). The citations are recognised by the tool
// the citation gates run (`migrate_citations.py --list-cited`), in both the
// token form the runtimes use and the prose form the templates keep, so this
// cannot disagree with the gates about what a citation is.
//
// The one hand-written thing is `ACCEPTED`. It is checked both ways: a
// section a backend lacks with no entry is refused, and an entry for a section
// the backend now cites is refused as stale. A citation the code should carry
// is not an entry: where a backend implements a section and does not say so,
// the fix is the citation.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use sce_build::generator::Language;

/// The backend the others port.
const REFERENCE: Language = Language::Rust;

/// Lower bound on the sections the reference cites: a recogniser or an
/// attribution that stopped working would shrink every set alike and leave
/// the comparison passing on less.
const MIN_REFERENCE_SECTIONS: usize = 60;

/// Why a backend's code does not cite a section the reference's does.
#[derive(Debug, Clone, Copy)]
enum Why {
    /// The backend does the part another way on purpose.
    Design(&'static str),
    /// The part is not built for this backend.
    NotYet(&'static str),
}

/// Every section a backend lacks, with the reason.
const ACCEPTED: &[(Language, &str, Why)] = &[];

/// The sections each backend's code cites, keyed by the backend's canonical
/// name.
type Cited = BTreeMap<&'static str, BTreeSet<String>>;

/// The sections each backend's code cites, recognised by the citation tool.
fn cited_by_backend() -> Cited {
    let root = common::repository::root();
    let out = Command::new("python3")
        .arg(root.join("tools/mnemosyne-adoption/migrate_citations.py"))
        .arg("--list-cited")
        .arg("--from-toml")
        .arg(root.join("docs/spec/scxml/mnemosyne.toml"))
        .arg(root.join("tools/codegen/templates"))
        .output()
        .expect("run migrate_citations.py --list-cited");
    assert!(
        out.status.success(),
        "migrate_citations.py --list-cited failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("--list-cited prints JSON");
    let files = report["files"]
        .as_object()
        .expect("the report carries a `files` object");

    let mut cited: Cited = Language::ALL
        .iter()
        .map(|l| (l.canonical_name(), BTreeSet::new()))
        .collect();
    for (path, ids) in files {
        let ids = ids
            .as_array()
            .expect("each file lists its sections")
            .iter()
            .map(|id| id.as_str().expect("a section id is text").to_string());
        let owners: Vec<Language> = match path.strip_prefix("tools/codegen/templates/") {
            Some(name) => Language::ALL
                .iter()
                .copied()
                .filter(|l| l.authors_template(name))
                .collect(),
            None => Language::ALL
                .iter()
                .copied()
                .filter(|l| path.starts_with(&format!("{}/", l.runtime_tree())))
                .collect(),
        };
        let ids: Vec<String> = ids.collect();
        for owner in owners {
            cited
                .get_mut(owner.canonical_name())
                .expect("every backend has a set")
                .extend(ids.iter().cloned());
        }
    }
    cited
}

/// What the comparison found: sections lacked with no entry, and entries
/// whose section is cited after all.
#[derive(Debug, Default, PartialEq, Eq)]
struct Verdict {
    undeclared: Vec<String>,
    stale: Vec<String>,
}

fn judge(cited: &Cited, reference: Language, accepted: &[(Language, &str, Why)]) -> Verdict {
    let mut verdict = Verdict::default();
    let empty = BTreeSet::new();
    let wanted = cited.get(reference.canonical_name()).unwrap_or(&empty);
    for (&backend, sections) in cited {
        if backend == reference.canonical_name() {
            continue;
        }
        for section in wanted.difference(sections) {
            if !accepted
                .iter()
                .any(|(l, s, _)| l.canonical_name() == backend && s == section)
            {
                verdict
                    .undeclared
                    .push(format!("{backend} does not cite {section}"));
            }
        }
    }
    for (language, section, why) in accepted {
        let lacks = wanted.contains(*section)
            && !cited
                .get(language.canonical_name())
                .is_some_and(|sections| sections.contains(*section));
        if !lacks {
            verdict.stale.push(format!(
                "{} is recorded as not citing {section} ({why:?}), which is no longer a gap \
                 against {}",
                language.canonical_name(),
                reference.canonical_name()
            ));
        }
    }
    verdict
}

/// The comparison refuses a gap with no entry and an entry with no gap, and
/// passes an entry that matches: without this the checks below could pass by
/// comparing nothing.
#[test]
fn the_comparison_refuses_an_undeclared_gap_and_a_stale_entry() {
    let set = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    let cited: Cited = [
        ("rust", set(&["scxml-1", "scxml-2", "scxml-3"])),
        ("go", set(&["scxml-1", "scxml-2"])),
        ("kotlin", set(&["scxml-1", "scxml-3"])),
    ]
    .into_iter()
    .collect();
    let accepted = [
        (Language::Go, "scxml-3", Why::NotYet("x")),
        (Language::Go, "scxml-1", Why::Design("y")),
    ];
    let verdict = judge(&cited, Language::Rust, &accepted);
    assert_eq!(verdict.undeclared, ["kotlin does not cite scxml-2"]);
    assert_eq!(verdict.stale.len(), 1, "{verdict:#?}");
    assert!(verdict.stale[0].starts_with("go is recorded as not citing scxml-1"));
}

/// Every backend has runtime code the ledger validates: a backend whose
/// runtime is outside the enrolment would compare as citing only what its
/// templates cite, and nothing would check that its citations name real
/// sections.
#[test]
fn every_backend_has_an_enrolled_runtime() {
    let root = common::repository::root();
    let toml = std::fs::read_to_string(root.join("docs/spec/scxml/mnemosyne.toml"))
        .expect("read the ledger's mnemosyne.toml");
    let unenrolled: Vec<&str> = Language::ALL
        .iter()
        .filter(|l| !toml.contains(&format!("\"{}/", l.runtime_tree())))
        .map(|l| l.canonical_name())
        .collect();
    assert!(
        unenrolled.is_empty(),
        "these backends have no runtime directory in the W3C citation ledger's enrolment \
         (`paths` of docs/spec/scxml/mnemosyne.toml): {unenrolled:?}"
    );
}

/// Every section the reference cites is cited by every other backend, or
/// `ACCEPTED` says why not.
#[test]
fn every_section_the_reference_cites_is_cited_or_recorded() {
    let cited = cited_by_backend();
    let reference = &cited[REFERENCE.canonical_name()];
    assert!(
        reference.len() >= MIN_REFERENCE_SECTIONS,
        "the reference backend cites only {} sections, so the recogniser or the attribution \
         has stopped reading its code",
        reference.len()
    );
    let verdict = judge(&cited, REFERENCE, ACCEPTED);
    assert!(
        verdict.undeclared.is_empty(),
        "{} section(s) the reference backend's code cites are not cited by another backend's. \
         Where the backend implements the section, cite it at the code that does; where it \
         does not, implement it, or add an entry to ACCEPTED saying why:\n  {}",
        verdict.undeclared.len(),
        verdict.undeclared.join("\n  ")
    );
}

/// No entry in `ACCEPTED` outlives its gap, and every entry says why.
#[test]
fn every_recorded_gap_is_current_and_says_why() {
    for (language, section, why) in ACCEPTED {
        let (Why::Design(reason) | Why::NotYet(reason)) = why;
        assert!(
            reason.split_whitespace().count() >= 4,
            "{} not citing {section} carries no reason a reader can act on: {reason:?}",
            language.canonical_name()
        );
    }
    let verdict = judge(&cited_by_backend(), REFERENCE, ACCEPTED);
    assert!(
        verdict.stale.is_empty(),
        "{} entr(y/ies) in ACCEPTED no longer describe the tree:\n  {}",
        verdict.stale.len(),
        verdict.stale.join("\n  ")
    );
}
