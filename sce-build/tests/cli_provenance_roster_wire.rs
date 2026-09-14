// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What `sce-codegen provenance-roster` PUBLISHES says what the roster
//! holds — checked against the published bytes, not against the table.
//!
//! # The failure this exists for, which was live and green
//!
//! The roster distinguishes three states: a code demonstrated carrying
//! the enclosing anchor, a code that can never carry, and a code no
//! scenario has run. The command emitted two verdicts, so all 312 of
//! the third kind published as `never` — telling every consumer they
//! *can never carry*, which is the stronger sentence and one nobody
//! had evidence for.
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
};

/// The generator binary. `env!` here rather than in a helper: it
/// expands at compile time, and `cli_feature_gating` derives which
/// targets reach the binary by finding this expansion in the target's
/// own source.
const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

/// Every verdict the command may print. Three, because the roster has
/// three states; a fourth spelling is a wire the table did not ask for.
const VERDICTS: &[&str] = &["carries", "never", "unknown"];

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

/// One line per code, and nothing but the three declared verdicts.
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
