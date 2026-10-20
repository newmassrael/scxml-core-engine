// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every capability one backend has, every other backend has too, or the
// difference is written down here with the reason.
//
// The owner's requirement for the Swift backend (2026-10-20) was the same
// structure as the other six, no single-source-of-truth violation, and a
// judgement by structure rather than by results. A conformance suite judges
// results: it can be green for a backend that never learnt a capability,
// because nothing asked it to. This check asks. For each backend it derives a
// row of capabilities from the registries that already decide them — the kind
// matrix, the static data model roster, the CLI routes, the script-engine
// seam, the Mesh rosters, the conformance harness tree — and holds every row
// to the union of all of them.
//
// Nothing here restates a registry. A capability is read from the code that
// decides it; the only thing written by hand is `ACCEPTED`, the record of
// which backend lacks which capability and why, and that record is checked in
// both directions: a gap with no entry is refused, and an entry for a gap that
// has closed is refused as stale, so the record cannot drift from the tree.
//
// Two registries answer "not applicable" themselves, with the reason in the
// answer, and those answers are taken as they stand rather than copied into
// `ACCEPTED`: the kind matrix's MCU class (a link, a buffer pool or a worker
// exists only where the machine is a microcontroller program) and the W3C
// suite's identity (a backend whose emitted tests never name their suite has
// no name to set).
//
// A backend added to `Language::ALL` joins every row at once. Each capability
// it does not have yet is then a red until an entry says so, which makes its
// entries in `ACCEPTED` the list of what is left; the `NotYet` ones close as
// it gains the capability and the stale check removes them as they do.
//
// Outside the generator it reads the integration channel table
// (`common::integration_channels`, the one `integration_stem_registration`
// holds the drivers to). What it does not read yet: the gates per backend and
// the shared fixtures each runtime reads. Those join in their own commit.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::integration_channels::Channel;

use sce_build::cli_language::LanguageRoute;
use sce_build::conformance::harness_layout;
use sce_build::forge::codegen_matrix::{lookup, EmitOutcome};
use sce_build::forge::model::ForgeKind;
use sce_build::forge::{cbor_codec, content_line_codec};
use sce_build::generator::{lowers_static_datamodel, Language, ScriptEngineTarget};
use sce_build::mesh::codegen::has_mesh_emitter;
use sce_build::mesh::transport::{host_core_languages, known_names};
use sce_build::template_registry::{embedded_templates_for, SUPPORTED_LANGUAGES};
use sce_build::w3c_suite::{SuiteIdentity, SuiteIdentityError};

/// Lower bound on the capabilities derived: a derivation that stopped
/// reading a registry would shrink every row alike and leave the comparison
/// passing on less.
const MIN_FACTS: usize = 40;

/// How a backend stands to one capability.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Holding {
    Holds,
    Lacks,
    /// A registry says the capability does not apply to this backend, and why.
    NotApplicable(String),
}

/// One capability and how every backend stands to it.
struct Fact {
    name: String,
    holdings: Vec<(Language, Holding)>,
}

fn fact(name: impl Into<String>, holding: impl Fn(Language) -> Holding) -> Fact {
    Fact {
        name: name.into(),
        holdings: Language::ALL.iter().map(|&l| (l, holding(l))).collect(),
    }
}

fn holds_if(yes: bool) -> Holding {
    if yes {
        Holding::Holds
    } else {
        Holding::Lacks
    }
}

/// Every capability, read from the registry that decides it.
fn derived_facts() -> Vec<Fact> {
    let mut facts = Vec::new();

    // The kind matrix: which Forge kinds each backend emits.
    for name in ForgeKind::ALL_ATTR_NAMES {
        let kind = ForgeKind::from_attr(name).expect("every attribute name parses");
        facts.push(fact(format!("forge kind `{name}`"), |l| {
            match lookup(kind, l) {
                EmitOutcome::Emit => Holding::Holds,
                EmitOutcome::TemplateMissing => Holding::Lacks,
                EmitOutcome::McuClassOnNonMcuLanguage => Holding::NotApplicable(
                    "an MCU-class kind on a backend that is not an MCU target (codegen_matrix)"
                        .to_string(),
                ),
            }
        }));
    }

    facts.push(fact("sce-static data model", |l| {
        holds_if(lowers_static_datamodel(l))
    }));
    facts.push(fact("may-fail algorithm bodies", |l| {
        holds_if(sce_build::forge::generator::lowers_may_fail(l))
    }));
    facts.push(fact("embedded template registry", |l| {
        holds_if(SUPPORTED_LANGUAGES.contains(&l))
    }));
    facts.push(fact("CBOR codec", |l| holds_if(cbor_codec::lowers(l))));
    facts.push(fact("content-line codec", |l| {
        holds_if(content_line_codec::lowers(l))
    }));
    facts.push(fact("save and restore", |l| {
        holds_if(l.writes_saved_state())
    }));

    // The numerical conformance harness: its scaffold, and the kind
    // fragments under it, from the tree the generator renders.
    let tree: BTreeSet<&str> = embedded_templates_for(Language::Cpp)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    facts.push(fact("numerical conformance harness", |l| {
        let layout = harness_layout(l);
        holds_if(
            tree.contains(
                format!("{}/{}", layout.template_subdir, layout.template_filename).as_str(),
            ),
        )
    }));
    let fragments_of = |l: Language| -> BTreeSet<String> {
        let prefix = format!("{}/kinds/", harness_layout(l).template_subdir);
        tree.iter()
            .filter_map(|name| name.strip_prefix(prefix.as_str()))
            .map(|rest| rest.split('.').next().unwrap_or(rest).to_string())
            .collect()
    };
    let every_fragment: BTreeSet<String> = Language::ALL
        .iter()
        .flat_map(|&l| fragments_of(l))
        .collect();
    for fragment in every_fragment {
        facts.push(fact(format!("conformance fragment `{fragment}`"), |l| {
            holds_if(fragments_of(l).contains(&fragment))
        }));
    }

    // The CLI: which subcommands serve each backend.
    for &route in LanguageRoute::ALL {
        facts.push(fact(format!("`{}` route", route.subcommand()), |l| {
            holds_if(route.serves(l))
        }));
    }

    // The script-engine seam: which engines a backend's artifact can run on.
    for &target in ScriptEngineTarget::ALL {
        facts.push(fact(
            format!("artifact for the {} engine", target.wire_name()),
            |l| holds_if(l.supports_script_engine_target(target)),
        ));
    }

    // Mesh: an emitter for a deploy build, and a host core per transport.
    facts.push(fact("Mesh emitter", |l| holds_if(has_mesh_emitter(l))));
    for transport in known_names() {
        facts.push(fact(format!("host Mesh core over `{transport}`"), |l| {
            holds_if(host_core_languages(transport).contains(&l))
        }));
    }

    // The standalone W3C suite: whether the emitted tests name a suite.
    facts.push(fact(
        "named W3C suite",
        |l| match SuiteIdentity::for_language(l) {
            Ok(_) => Holding::Holds,
            Err(SuiteIdentityError::NotApplicable { reason, .. }) => {
                Holding::NotApplicable(reason.to_string())
            }
            Err(other) => panic!(
                "{l:?}: the suite identity answered with neither a name nor a reason: {other:?}"
            ),
        },
    ));

    // The integration channels: whether a backend has one. That a channel
    // drives every stem it does not declare it cannot host is
    // `integration_stem_registration`'s check over the same table; what it
    // cannot see is a backend with no channel at all, which is what this asks.
    facts.push(fact("integration channel", |l| {
        holds_if(Channel::ALL.iter().any(|c| c.language() == Some(l)))
    }));

    // A capability no backend holds is not one any backend is behind on.
    facts.retain(|f| f.holdings.iter().any(|(_, h)| *h == Holding::Holds));
    facts
}

/// Why a backend lacks a capability.
#[derive(Debug, Clone, Copy)]
enum Why {
    /// The design gives this backend another way, or none, on purpose.
    Design(&'static str),
    /// Not built for this backend; an entry that closes when it is.
    NotYet(&'static str),
}

/// Every capability a backend lacks, with the reason. The one hand-written
/// table in this file, and checked both ways against the derived rows.
const ACCEPTED: &[(Language, &str, Why)] = &[
    // The CLI routes (cli_language.rs, `LanguageRoute::exclusion_reason`).
    (
        Language::C11,
        "`generate-w3c` route",
        Why::NotYet("no C11 W3C statechart emitter exists yet; the C11 W3C lane runs hand-authored drivers through CMake"),
    ),
    (
        Language::Cpp,
        "`generate-integration` route",
        Why::Design("its integration fixtures emit at CMake build time, so there is no committed tree to regenerate"),
    ),
    (
        Language::C11,
        "`generate-integration` route",
        Why::Design("its integration fixtures emit at CMake build time, so there is no committed tree to regenerate"),
    ),
    // The script-engine seam (docs/SCE_LUA_TRANSLATION_SEAM.md).
    (
        Language::Rust,
        "artifact for the ecmascript engine",
        Why::Design("it lowers ECMAScript to Lua at build time, and the Lua is the only artifact"),
    ),
    (
        Language::Go,
        "artifact for the ecmascript engine",
        Why::Design("it lowers ECMAScript to Lua at build time, and the Lua is the only artifact"),
    ),
    (
        Language::Python,
        "artifact for the ecmascript engine",
        Why::Design("it lowers ECMAScript to Lua at build time, and the Lua is the only artifact"),
    ),
    (
        Language::C11,
        "artifact for the ecmascript engine",
        Why::Design("it lowers ECMAScript to Lua at build time, and the Lua is the only artifact"),
    ),
    // Mesh (SCE_MESH.md §mesh-19, §mesh-20).
    (
        Language::Go,
        "Mesh emitter",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::Python,
        "Mesh emitter",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::C11,
        "Mesh emitter",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::Cpp,
        "host Mesh core over `wss`",
        Why::Design("C++ routes Mesh through its generated TransportRouter, not a host core"),
    ),
    (
        Language::Go,
        "host Mesh core over `wss`",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::Python,
        "host Mesh core over `wss`",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::C11,
        "host Mesh core over `wss`",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::Cpp,
        "host Mesh core over `zenoh_inbox`",
        Why::Design("C++ routes Mesh through its generated TransportRouter, not a host core"),
    ),
    (
        Language::Kotlin,
        "host Mesh core over `zenoh_inbox`",
        Why::NotYet("the Kotlin core has no Zenoh client adapter (SCE_MESH.md §mesh-20.4)"),
    ),
    (
        Language::Go,
        "host Mesh core over `zenoh_inbox`",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::Python,
        "host Mesh core over `zenoh_inbox`",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    (
        Language::C11,
        "host Mesh core over `zenoh_inbox`",
        Why::NotYet("no host Mesh core for this backend; the E5 extension opened Rust and Kotlin"),
    ),
    // Saving and restoring a machine (SCE Accepted Subset §2.15).
    (
        Language::Cpp,
        "save and restore",
        Why::NotYet("the E17 extension built it for Rust and Kotlin"),
    ),
    (
        Language::Go,
        "save and restore",
        Why::NotYet("the E17 extension built it for Rust and Kotlin"),
    ),
    (
        Language::Python,
        "save and restore",
        Why::NotYet("the E17 extension built it for Rust and Kotlin"),
    ),
    (
        Language::C11,
        "save and restore",
        Why::NotYet("the E17 extension built it for Rust and Kotlin"),
    ),
];

/// What the comparison found: gaps with no entry, and entries whose gap is
/// not there.
#[derive(Debug, Default, PartialEq, Eq)]
struct Verdict {
    undeclared: Vec<String>,
    stale: Vec<String>,
}

fn judge(facts: &[Fact], accepted: &[(Language, &str, Why)]) -> Verdict {
    let mut verdict = Verdict::default();
    let standing: BTreeMap<(String, &str), &Holding> = facts
        .iter()
        .flat_map(|f| {
            f.holdings
                .iter()
                .map(move |(l, h)| ((f.name.clone(), l.canonical_name()), h))
        })
        .collect();
    let declared: BTreeSet<(String, &str)> = accepted
        .iter()
        .map(|(l, name, _)| (name.to_string(), l.canonical_name()))
        .collect();
    for ((name, backend), holding) in &standing {
        if **holding == Holding::Lacks && !declared.contains(&(name.clone(), *backend)) {
            verdict.undeclared.push(format!("{backend} lacks {name}"));
        }
    }
    for (l, name, why) in accepted {
        let key = (name.to_string(), l.canonical_name());
        match standing.get(&key) {
            Some(Holding::Lacks) => {}
            Some(other) => verdict.stale.push(format!(
                "{} is recorded as lacking {name} ({why:?}) but it is {other:?}",
                l.canonical_name()
            )),
            None => verdict.stale.push(format!(
                "{} is recorded as lacking {name} ({why:?}), which is not a capability any \
                 registry derives",
                l.canonical_name()
            )),
        }
    }
    verdict
}

/// The comparison refuses a gap with no entry and an entry with no gap, and
/// passes an entry that matches: without this the two checks below could pass
/// by comparing nothing.
#[test]
fn the_comparison_refuses_an_undeclared_gap_and_a_stale_entry() {
    let facts = vec![
        fact("a", |l| holds_if(l != Language::Go)),
        fact("b", |l| holds_if(l != Language::Python)),
        fact("c", |_| Holding::Holds),
    ];
    let accepted = [
        (Language::Go, "a", Why::NotYet("x")),
        (Language::Rust, "c", Why::NotYet("y")),
        (Language::Kotlin, "d", Why::Design("z")),
    ];
    let verdict = judge(&facts, &accepted);
    assert_eq!(verdict.undeclared, ["python lacks b"]);
    assert_eq!(verdict.stale.len(), 2, "{verdict:#?}");
    assert!(verdict.stale[0].starts_with("rust is recorded as lacking c"));
    assert!(verdict.stale[1].contains("not a capability any registry derives"));
}

/// Every capability a backend lacks is one `ACCEPTED` records, with a reason.
#[test]
fn every_capability_a_backend_lacks_is_recorded_with_its_reason() {
    let facts = derived_facts();
    assert!(
        facts.len() >= MIN_FACTS,
        "only {} capabilities were derived, so a registry has stopped being read",
        facts.len()
    );
    let verdict = judge(&facts, ACCEPTED);
    assert!(
        verdict.undeclared.is_empty(),
        "{} capability gap(s) are not recorded. Either the backend should have it (build it), or \
         the difference is accepted: add an entry to ACCEPTED saying why, as Design when the \
         design gives the backend another way on purpose and NotYet when it is simply not built:\n  {}",
        verdict.undeclared.len(),
        verdict.undeclared.join("\n  ")
    );
}

/// Every entry says why: an accepted difference with no reason is one nobody
/// can judge when the backend is revisited.
#[test]
fn every_recorded_gap_says_why() {
    for (language, name, why) in ACCEPTED {
        let (Why::Design(reason) | Why::NotYet(reason)) = why;
        assert!(
            reason.split_whitespace().count() >= 4,
            "{} lacking {name} carries no reason a reader can act on: {reason:?}",
            language.canonical_name()
        );
    }
}

/// No entry in `ACCEPTED` outlives its gap.
#[test]
fn no_recorded_gap_has_closed() {
    let verdict = judge(&derived_facts(), ACCEPTED);
    assert!(
        verdict.stale.is_empty(),
        "{} entr(y/ies) in ACCEPTED no longer describe the tree. A NotYet entry whose backend \
         gained the capability is removed, which is the point of it; any other is corrected:\n  {}",
        verdict.stale.len(),
        verdict.stale.join("\n  ")
    );
}
