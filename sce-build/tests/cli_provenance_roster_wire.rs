// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What `sce-codegen provenance-roster` PUBLISHES says what the roster
//! holds — checked against the published bytes, not against the table.
//!
//! # The failure this exists for, which was live and green
//!
//! The roster distinguished three states at the time: a code
//! demonstrated carrying the enclosing anchor, a code that can never
//! carry, and a code no scenario has run. The command emitted two
//! verdicts, so all 312 of the third kind published as `never` —
//! telling every consumer they *can never carry*, which is the
//! stronger sentence and one nobody had evidence for.
//!
//! ⚠ It has since grown a fourth, `pending`, and the same collapse was
//! found live in it (2026-09-15). The count above is deliberately left
//! in the past tense rather than updated: it describes the day of the
//! defect. [`VERDICTS`] is the only place the current set is stated.
//!
//! It survived the full `sce-build` suite, clippy, `tree-hygiene` and a
//! push. Not because those gates are weak, but because **every one of
//! them asserts on the roster in-crate**: they ask `anchor_carriage`
//! what it answers and agree with themselves. A gate that reads the
//! table twice cannot see the wire drop a distinction between them.
//!
//! ⚠ A gate over ALL printed surfaces was considered and is NOT
//! buildable today, which is worth saying so nobody spends a round
//! finding out again. It would need to derive which subcommands print
//! something a consumer parses, and that is not a scan: of the 21
//! `cmd_*` handlers only TWO contain a `print!` in their own body,
//! while the binary holds 36 print sites — the rest emit through
//! helpers. A predicate keyed on print macros would call ten real
//! surfaces non-surfaces, and a hand-listed set checked against
//! nothing independent is the exclusion list this repository has been
//! burned by. The twelve are gated one at a time until something in
//! the tree can name them.
//!
//! So this target spawns the binary and compares what came out of it
//! against what the table says — the only arrangement in which
//! flattening is visible. `SCE_ERROR_CONTRACT.md` §2.1.2 is the clause
//! that depends on it: a consumer is told to read an absent
//! `spec_provenance` by looking the code up here, and a lookup that
//! reports `never` for an unmeasured code sends them to the wrong
//! conclusion with the contract's own blessing.

use std::collections::BTreeSet;
use std::process::Command;

use sce_build::forge::diagnostic::{
    anchor_carriage, AnchorCarriage, NoAnchor, Pipeline, ALL_DIAGNOSTIC_CODES,
    PROVENANCE_ROSTER_STATUS,
};

/// The generator binary. `env!` here rather than in a helper: it
/// expands at compile time, and `cli_feature_gating` derives which
/// targets reach the binary by finding this expansion in the target's
/// own source.
const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

/// Every verdict the command may print. Four, because the roster
/// distinguishes four states; a fifth spelling is a wire the table did
/// not ask for.
///
/// ⚠ `pending` is the youngest and the one that had been missing:
/// until 2026-09-15 a code whose coordinate is merely unthreaded was
/// published as `never`, which is a claim about the code rather than
/// about SCE's progress on it.
const VERDICTS: &[&str] = &["carries", "never", "unknown", "pending"];

/// The roster is a registered wire surface, and the registry says the
/// same thing about it that the producer does.
///
/// # Why this one needs its own guard
///
/// `wire_surface_stability.rs` closes the registry's loop by walking
/// `schemas/` and `apis/` back to the declared lists, so a schema file
/// cannot land, be consumed, and never acquire a row. The roster has no
/// schema file — it is a TSV a subcommand prints — so that walk cannot
/// reach it, and it was published, wired into `SCE_ERROR_CONTRACT.md`
/// §2.1.2 as the lookup a consumer is told to run, and gated here,
/// while remaining absent from the registry entirely. Nothing was
/// broken; nothing could have noticed.
///
/// The module docs above record why the general version — a gate over
/// every printed surface — is not buildable. That leaves per-surface
/// anchoring as the available shape, which is what this is.
#[test]
fn the_registry_declares_the_rosters_status() {
    let registry = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("sce-build has a parent dir")
            .join("SCE_WIRE_CONTRACTS.md"),
    )
    .expect("SCE_WIRE_CONTRACTS.md is readable");

    let row = registry
        .lines()
        .find(|line| line.starts_with("| Provenance roster (`sce-codegen provenance-roster`)"))
        .unwrap_or_else(|| {
            panic!(
                "SCE_WIRE_CONTRACTS.md has no row for the provenance \
                 roster. It is a surface a consumer is directed to by \
                 SCE_ERROR_CONTRACT.md §2.1.2, so it needs a row saying \
                 how stable it is and how a reader learns it changed — \
                 and no reverse walk can add one for it, because it has \
                 no schema file to be walked."
            )
        });

    assert!(
        row.contains(&format!("`{PROVENANCE_ROSTER_STATUS}`")),
        "the registry's roster row does not name the status the \
         producer declares ({PROVENANCE_ROSTER_STATUS}). \
         SCE_WIRE_CONTRACTS.md requires one commit to move both.\nrow: \
         {row}",
    );
    assert!(
        row.contains("PROVENANCE_ROSTER_STATUS"),
        "the registry's roster row must cite the producer const that \
         settles its status, so a reader is sent to the one place that \
         decides it rather than trusting the table.\nrow: {row}",
    );
}

/// The published roster, one `(code, verdict, reason)` per line.
fn published() -> Vec<(String, String, String)> {
    let out = Command::new(CODEGEN)
        .arg("provenance-roster")
        .output()
        .expect("spawn sce-codegen");
    assert!(
        out.status.success(),
        "`provenance-roster` exited {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8(out.stdout).expect("roster is utf-8");
    stdout
        .lines()
        .map(|line| {
            let mut parts = line.split('\t');
            let code = parts.next().unwrap_or_default().to_string();
            let verdict = parts.next().unwrap_or_default().to_string();
            let reason = parts.next().unwrap_or_default().to_string();
            assert!(
                parts.next().is_none(),
                "a roster line carries more than three tab-separated \
                 fields, so a consumer splitting on the tab reads a \
                 reason as a fourth column: {line:?}",
            );
            (code, verdict, reason)
        })
        .collect()
}

/// One line per code, and nothing but the declared verdicts.
///
/// ⚠ The count lives in [`VERDICTS`] and is not restated here. It read
/// "the three declared verdicts" until a fourth landed, which is the
/// smallest possible instance of the thing this file is about.
#[test]
fn the_wire_prints_one_declared_verdict_for_every_code() {
    let rows = published();
    assert_eq!(
        rows.len(),
        ALL_DIAGNOSTIC_CODES.len(),
        "the roster published {} line(s) for {} code(s); a consumer \
         enumerating the contract from this output would be short",
        rows.len(),
        ALL_DIAGNOSTIC_CODES.len(),
    );

    let published_codes: BTreeSet<&str> = rows.iter().map(|(c, _, _)| c.as_str()).collect();
    let known: BTreeSet<&str> = ALL_DIAGNOSTIC_CODES.iter().map(|c| c.as_str()).collect();
    assert_eq!(
        published_codes, known,
        "the codes on the wire are not the codes the crate holds",
    );

    for (code, verdict, _) in &rows {
        assert!(
            VERDICTS.contains(&verdict.as_str()),
            "`{code}` published the verdict {verdict:?}, which is not one \
             of {VERDICTS:?}. A consumer branches on this word.",
        );
    }
}

/// ⭐ The anti-flattening assertion: `unknown` is exactly the set of
/// codes the table calls `NotYetMeasured`.
///
/// This is the one that would have failed on the defect. Equality in
/// BOTH directions, because each catches a different collapse: an
/// unmeasured code published as `never` claims impossibility on no
/// evidence, and a code that genuinely cannot carry published as
/// `unknown` invites a reader to expect it might later.
#[test]
fn unknown_on_the_wire_is_exactly_what_the_table_calls_unmeasured() {
    let wire: BTreeSet<String> = published()
        .into_iter()
        .filter(|(_, verdict, _)| verdict == "unknown")
        .map(|(code, _, _)| code)
        .collect();

    let table: BTreeSet<String> = ALL_DIAGNOSTIC_CODES
        .iter()
        .filter(|code| {
            matches!(
                anchor_carriage(**code, Pipeline::Statechart),
                AnchorCarriage::Registered(NoAnchor::NotYetMeasured)
            )
        })
        .map(|code| code.as_str().to_string())
        .collect();

    assert_eq!(
        wire, table,
        "the wire's `unknown` set and the table's `NotYetMeasured` set \
         disagree. Publishing an unmeasured code as `never` tells a \
         consumer it can NEVER carry, which is a claim about the code \
         made on evidence nobody has — and it is invisible to every \
         gate that asks the table what the table says.",
    );
}

/// The command's own `--help` names exactly the verdicts it prints.
///
/// # A fourth copy nothing held
///
/// The verdict vocabulary lives in four places: the producer's match,
/// [`VERDICTS`] here, `SCE_ERROR_CONTRACT.md` §2.1.2, and the clap doc
/// comment that becomes `--help`. The first two are pinned to each
/// other by the gates in this file through the published bytes. The
/// help text was pinned by nothing, and it is not an internal comment
/// — it is the first thing a consumer reads, ahead of the contract.
///
/// Measured 2026-09-15: adding `pending` to the wire left the help
/// still saying *"Three verdicts because the roster holds three
/// states"* and listing `<carries|never|unknown>`. Every gate on this
/// surface stayed green, because all of them read the roster's output
/// and none of them read what the tool says about itself.
///
/// Compared as a SET in both directions, so a verdict missing from the
/// help and a verdict the help invents both fail.
#[test]
fn the_help_text_names_exactly_the_verdicts_the_wire_prints() {
    let out = Command::new(CODEGEN)
        .args(["help", "provenance-roster"])
        .output()
        .expect("spawn sce-codegen help");
    let raw =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    // clap wraps long help to the terminal width, so the alternation
    // can arrive split across lines. Collapse whitespace before
    // parsing rather than let a line break decide whether this gate
    // sees a verdict.
    let help = raw.split_whitespace().collect::<Vec<_>>().join(" ");

    let open = help.find("<carries").unwrap_or_else(|| {
        panic!(
            "`provenance-roster --help` does not show the verdict \
             alternation a consumer reads before anything else. It is \
             the first description of this surface they meet; a \
             surface that describes itself differently from what it \
             prints has two contracts.\nhelp:\n{help}"
        )
    });
    let close = help[open..]
        .find('>')
        .expect("the verdict alternation closes")
        + open;
    let listed: BTreeSet<&str> = help[open + 1..close].split('|').map(str::trim).collect();
    let declared: BTreeSet<&str> = VERDICTS.iter().copied().collect();

    assert_eq!(
        listed, declared,
        "`provenance-roster --help` advertises {listed:?} but the \
         command prints {declared:?}. The help is the first account of \
         this surface a consumer reads, so a verdict missing here is a \
         verdict they will not branch on, and one invented here is a \
         branch that never fires.",
    );
}

/// ⭐ `pending` on the wire is exactly the set the table calls
/// `CoordinateNotThreaded`.
///
/// The sibling above pins `unknown` ↔ `NotYetMeasured`. That pinned
/// ONE reason and left the rest to a catch-all which published them
/// all as `never`, so the wire asserted impossibility for a code whose
/// coordinate is simply not threaded yet — the same collapse §6b.1
/// removed, one reason to the left, and invisible to every gate
/// because none of them pinned what the OTHER reasons may claim.
///
/// Equality in both directions, for the two different errors: a
/// pending code published as `never` tells a consumer to stop
/// expecting it, and a permanently-unanchorable code published as
/// `pending` promises work nobody can do.
#[test]
fn pending_on_the_wire_is_exactly_what_the_table_calls_unthreaded() {
    let wire: BTreeSet<String> = published()
        .into_iter()
        .filter(|(_, verdict, _)| verdict == "pending")
        .map(|(code, _, _)| code)
        .collect();

    let table: BTreeSet<String> = ALL_DIAGNOSTIC_CODES
        .iter()
        .filter(|code| {
            matches!(
                anchor_carriage(**code, Pipeline::Statechart),
                AnchorCarriage::Registered(NoAnchor::CoordinateNotThreaded)
            )
        })
        .map(|code| code.as_str().to_string())
        .collect();

    assert_eq!(
        wire, table,
        "the wire's `pending` set and the table's `CoordinateNotThreaded` \
         set disagree. A code whose producer simply never threaded a \
         position is owed work, not an impossibility, and publishing it \
         as `never` tells a consumer to stop expecting what SCE intends \
         to deliver.",
    );
}

/// No verdict is empty.
///
/// A floor rather than a count: the exact numbers move as scenarios
/// land, and pinning them here would make this gate a second place to
/// edit. What must never happen is a verdict disappearing — that is
/// what a flattening looks like from the outside, whichever direction
/// it collapses in.
#[test]
fn every_declared_verdict_actually_appears() {
    let rows = published();
    for verdict in VERDICTS {
        let n = rows.iter().filter(|(_, v, _)| v == verdict).count();
        assert!(
            n > 0,
            "no code published the verdict {verdict:?}. Either the \
             roster lost a state or the wire stopped distinguishing \
             one — the second is how 312 unmeasured codes came to \
             claim they could never carry.",
        );
    }
}

/// A code that is not `carries` says WHY, and one that carries does not
/// need to.
#[test]
fn a_refusal_carries_its_reason() {
    for (code, verdict, reason) in published() {
        if verdict == "carries" {
            continue;
        }
        assert!(
            reason.len() > 40,
            "`{code}` published {verdict:?} with a reason a reader \
             cannot weigh: {reason:?}. The roster exists so a consumer \
             need not read source; a bare verdict sends them back to it.",
        );
    }
}
