// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which backend runtimes the spec ledger can actually judge.
//
// `validate-code-refs` enforces two different things about a `§scxml-` cite.
// EXISTENCE — the id names a real ledger section — is checked over the whole
// tree. BINDING — the cite is tied to what the ledger says implements that
// section — is checked only inside `[code_refs] paths` in
// `docs/spec/scxml/mnemosyne.toml`. Inside that scan, a SYMBOL-level binding
// needs a resolver for the file's language; without one the binding is
// file-level.
//
// Five backend runtimes implement the same clauses. Two of them were inside
// that scan and three were not, so their cites were spell-checked and nothing
// more. Enrolling the three earlier would have been a false claim rather than
// a fix: a language with no resolver used to publish a symbol_mismatch count
// of zero, which reads as "checked and clean". Mnemosyne R1142 made the axis
// say it had no instrument instead, and R1144 refuses a run whose recorded
// symbol no resolver covers — so at that pin all five are enrolled, all cites
// carry bindings, and what is missing is named out loud by the tool.
//
// That asymmetry was recorded as prose in the ledger config — which claimed,
// in the present tense, that "no hand-authored §scxml- citation lives in an
// excluded tree" while 62 of them did. Prose cannot hold a claim like that;
// this can.
//
// What was left was upstream, not local: Mnemosyne shipped `tree-sitter-cpp`
// and `tree-sitter-rust` plugin crates and no others, its symbol-axis
// extension table mapped `.go` and `.py` to languages that had no backend, and
// `.kt` was not in that table at all. The pin bump to `ecee1fe0` closed all
// three at once — that build carries five in-process backends and answers
// `describe-symbol-axis-reach` with "every language a file can map to has a
// resolver".
//
// The pin alone did not close it, and the difference is the point. A build that
// CAN resolve a language is not a workspace that DOES: enrolment is per-ledger,
// and the axis only judges a binding that records a `symbol`. Measured after
// the bump and before the enrolment: the three languages had 151 bindings and
// none of them named a symbol, so declaring the resolvers on their own would
// have moved them out of `unresolved_languages` — the tool's honest "no
// instrument" signal — and into `covered`, while still comparing nothing.
//
// So the enrolment and the 324 symbols landed together, and `UNREACHED` is
// empty because the workspace says so, not because someone remembered to prune
// it. The list used to be retired "by hand in the commit that lands the
// resolver", and it duly survived that commit: every assertion here stayed
// green while all three stated reasons had become false, because nothing asked
// the tool. Now something does.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Backend runtimes, each of which lowers the same SCXML clauses for one
/// language. Test trees and generated output are deliberately absent: this is
/// about hand-authored implementation code.
const RUNTIMES: &[&str] = &[
    "backends/c/runtime",
    "backends/go/runtime",
    "backends/kotlin/runtime",
    "backends/python/runtime",
    "backends/rust/runtime/src",
];

/// Why a runtime the scan reaches is still outside the SYMBOL axis.
///
/// The distinction is the whole point of recording it. An `Upstream` gap is
/// waiting on someone else and nothing in this repository closes it; a
/// `LocalEnrolment` gap is work this repository can do and has not. Written as
/// one prose field, they read the same — which is how three entries went on
/// blaming upstream through the bump that closed their upstream half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cause {
    /// The pinned build carries no resolver for the language.
    Upstream,
    /// The pinned build resolves it; this ledger does not enrol it.
    LocalEnrolment,
}

/// Runtimes the scan reaches but the SYMBOL axis cannot, with the cause. An
/// entry here is a claim that gets checked below, not an exemption: the tree
/// must still be inside the scan set, it must still cite, the workspace must
/// still report the language unresolved, and the cause must still be the cause.
/// Empty, and empty for a measured reason rather than an unrecorded one: the
/// `ecee1fe0` pin carries the three resolvers, the ledgers declare them, and
/// the 151 file-level bindings those trees carried now name symbols. The
/// workspace reports no unresolved language, which is the condition the checks
/// below hold it to.
const UNREACHED: &[(&str, Cause, &str)] = &[];

/// `[code_refs] paths` from the scxml ledger config — the scan set itself,
/// read rather than restated, so this test cannot disagree with the gate.
fn scanned_paths(root: &Path) -> Vec<String> {
    let toml = fs::read_to_string(root.join("docs/spec/scxml/mnemosyne.toml"))
        .expect("read docs/spec/scxml/mnemosyne.toml");
    let (_, after) = toml
        .split_once("\npaths = [")
        .expect("mnemosyne.toml declares [code_refs] paths");
    let (block, _) = after.split_once("\n]").expect("paths list is closed");
    let out: Vec<String> = block
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter_map(|l| l.strip_prefix('"'))
        .filter_map(|l| l.split('"').next())
        .map(str::to_string)
        .collect();
    assert!(
        out.len() > 10,
        "parsed {} scan path(s) from mnemosyne.toml; the parse broke, not the config",
        out.len()
    );
    out
}

fn is_under(rel: &str, dir: &str) -> bool {
    rel == dir || rel.starts_with(&format!("{}/", dir.trim_end_matches('/')))
}

/// Tracked files under `dir` that carry a hand-authored `§scxml-` cite.
/// Generated files are skipped: their cite came from a template, so the tree
/// that owns it is the template tree.
fn citing_files(root: &Path, dir: &str) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("ls-files")
        .arg(dir)
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files {dir} failed");

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|rel| {
            let Ok(text) = fs::read_to_string(root.join(rel)) else {
                return false;
            };
            let head: String = text.chars().take(2000).collect();
            if head.contains("SCE-GENERATED")
                || head.contains("DO NOT EDIT")
                || head.contains("Code generated by")
            {
                return false;
            }
            text.contains("§scxml-")
        })
        .map(str::to_string)
        .collect()
}

/// Every backend runtime that cites the spec is inside the ledger's scan.
///
/// Before the R1142 pin this could only ask for "reached OR named", because
/// enrolling a resolver-less language published a zero that read as a clean
/// check. It can ask for the stronger thing now, and the stronger thing is
/// what closes the gap: a cite inside the scan must be BOUND to something the
/// ledger records, whether or not a resolver can pin it to a symbol.
#[test]
fn every_backend_runtime_citing_the_spec_is_inside_the_scan() {
    let root = repo_root();
    let paths = scanned_paths(&root);

    let mut outside: Vec<String> = Vec::new();
    let mut examined = 0usize;

    for runtime in RUNTIMES {
        let cites = citing_files(&root, runtime);
        if cites.is_empty() {
            continue;
        }
        examined += 1;
        // A runtime counts as reached when the scan covers it, whether the
        // configured path is the tree root or a source directory inside it.
        let reached = paths
            .iter()
            .any(|p| is_under(runtime, p) || is_under(p, runtime));
        if !reached {
            outside.push(format!(
                "{runtime} ({} file(s) cite the spec), e.g. {}",
                cites.len(),
                cites.first().expect("non-empty")
            ));
        }
    }

    assert!(
        examined >= 4,
        "only {examined} of {} backend runtime(s) were found to cite the spec; \
         the scan broke, not the tree",
        RUNTIMES.len(),
    );

    assert!(
        outside.is_empty(),
        "backend runtime(s) cite `§scxml-` from outside the ledger's scan:\n  {}\n\
         Their cites are checked for existence only — nothing ties them to the \
         clause the ledger says they implement. Add the tree's SOURCE directory \
         to `[code_refs] paths`; a language with no symbol resolver is reported \
         as unreached rather than silently counted clean since Mnemosyne R1142.",
        outside.join("\n  "),
    );
}

/// Each named gap still has the cause it names.
///
/// This is the third way a gap entry stops describing the tree, and it is the
/// one that went unmeasured: a resolver lands upstream, the pin moves, and
/// every other assertion here stays green while the stated reasons are false.
/// It happened at the bump to `ecee1fe0`, whose build gained tree-sitter-go,
/// -kotlin and -python — the three entries went on saying Mnemosyne ships no
/// resolver for them.
///
/// Two questions decide it, and they are not the same question. What the BUILD
/// can resolve comes from `describe-symbol-axis-reach`; what this WORKSPACE
/// actually resolves comes from `validate-code-refs --json`, whose
/// `symbol_axis.unresolved_languages` names the languages whose cites bind at
/// file level here. At this pin they disagree — the build resolves five
/// languages and the ledgers enrol two — so a check that asked only the first
/// would have retired three gaps that are still real.
///
/// A missing binary is reported as unmeasured, not as a failure. The pin is
/// raised before the new revision is installed as a matter of course, and a
/// gate that spells "I have no tool" as "the author is wrong" sends the author
/// looking for a defect in their own tree — measured on this repository when a
/// build machine lacked the binary and two contract tests reported that the
/// gate "rejected a real citation".
///
/// That courtesy is for a developer's machine and stops at a lane that claims
/// its checks ran. `SCE_REQUIRE_TOOLS` is this repository's word for that
/// claim, and under it the absence is a hard failure: a skip is an unrun
/// check, not a passing one, and this suite is invisible from the outside
/// either way — it prints its note to stderr and returns `ok`.
#[test]
fn each_named_gap_still_has_the_cause_it_names() {
    let root = repo_root();
    let Some(bin) = pinned_mnemosyne_cli(&root) else {
        assert!(
            !sce_build::toolchain::tools_are_required(),
            "SCE_REQUIRE_TOOLS is set, so this lane claims its checks ran — but \
             no rev-pinned mnemosyne-cli is installed for the revision \
             `.github/workflows/spec-citations.yml` names. Without it neither \
             half runs: nothing reads which languages this build resolves and \
             nothing reads which ones these ledgers bind, so the UNREACHED \
             list ({} entr(ies)) is compared against nothing — including when \
             it is empty, which is the claim that every runtime is reached. \
             Run scripts/install_mnemosyne_cli.sh in this job, or point \
             MNEMOSYNE_BIN at the binary.",
            UNREACHED.len()
        );
        eprintln!(
            "unmeasured: no rev-pinned mnemosyne-cli for the revision \
             `.github/workflows/spec-citations.yml` names. Install it, or point \
             MNEMOSYNE_BIN at it, to have this build's symbol-axis reach checked \
             against the {} gap entr(ies) below.",
            UNREACHED.len()
        );
        return;
    };

    let out = Command::new(&bin)
        .arg("describe-symbol-axis-reach")
        .current_dir(root.join("docs/spec/scxml"))
        .output()
        .unwrap_or_else(|e| panic!("run {}: {e}", bin.display()));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "`describe-symbol-axis-reach` exited {:?}:\n{}{}",
        out.status.code(),
        text,
        String::from_utf8_lossy(&out.stderr),
    );

    let (resolved, extensions) = parse_reach(&text);
    assert!(
        resolved.len() >= 2 && extensions.len() >= 4,
        "parsed {} backend(s) and {} extension row(s) from the tool's report; \
         the parse broke, not the build. An empty parse would let every check \
         below pass without asking anything:\n{text}",
        resolved.len(),
        extensions.len(),
    );

    // The question each runtime asks is decided by the extensions it is
    // actually written in — not by a directory-name-to-language table, which
    // would be one more list to keep true. `backends/c/runtime` is the case
    // that makes the difference: its `.c` files map to `cpp`, not to a
    // language named after the directory.
    let mut unreached_now: Vec<String> = Vec::new();
    let mut asked: BTreeSet<&String> = BTreeSet::new();
    for dir in RUNTIMES {
        for ext in source_extensions(&root, dir) {
            let Some(lang) = extensions.get(&ext) else {
                continue; // the axis never claimed to route this extension
            };
            asked.insert(lang);
            if !resolved.contains(lang) {
                unreached_now.push(format!(
                    "{dir}: `{ext}` maps to `{lang}`, which has no resolver"
                ));
            }
        }
    }
    unreached_now.sort();
    unreached_now.dedup();

    // A lower bound on what was asked, not on what was found. The runtimes are
    // walked on disk, and a walk that returns nothing asks nothing — both
    // directions below then pass without the tool's answer mattering, and a
    // vacuous run prints the same green as a clean one. Four: the five trees
    // are written in cpp, go, kotlin, python and rust, and `backends/c/runtime`
    // routes through `cpp` like the C++ tree does.
    assert!(
        asked.len() >= 4,
        "only {} language(s) were asked about across {} runtime tree(s) — {:?}. \
         The trees are walked on disk; a walk that yields nothing makes every \
         check below vacuous.",
        asked.len(),
        RUNTIMES.len(),
        asked,
    );

    // What this workspace actually reaches, which is not what the build can
    // reach: the ledgers enrol a resolver per language, so a build that
    // carries five and a ledger that declares two leave three unresolved here.
    let unresolved_here = workspace_unresolved_languages(&bin, &root);
    assert!(
        !unresolved_here.is_empty() || UNREACHED.is_empty(),
        "the workspace reports no unresolved language, yet {} gap entr(ies) \
         claim one. Either the enrolment landed and the entries should go, or \
         the report was not read.",
        UNREACHED.len(),
    );

    let mut wrong: Vec<String> = Vec::new();
    for (dir, cause, reason) in UNREACHED {
        // The languages this tree is written in, through the tool's table.
        let langs: BTreeSet<&String> = source_extensions(&root, dir)
            .iter()
            .filter_map(|ext| extensions.get(ext))
            .collect();

        if !langs.iter().any(|l| unresolved_here.contains(*l)) {
            wrong.push(format!(
                "{dir}: the workspace resolves every language it is written in \
                 ({langs:?}), so this gap is closed — {reason}"
            ));
            continue;
        }
        // The cause each entry names, checked against the build rather than
        // taken on trust. This is the half that was silently false.
        let build_resolves_all = langs.iter().all(|l| resolved.contains(*l));
        let actual = if build_resolves_all {
            Cause::LocalEnrolment
        } else {
            Cause::Upstream
        };
        if actual != *cause {
            wrong.push(format!(
                "{dir}: recorded as {cause:?} but the pinned build says \
                 {actual:?} — {reason}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "UNREACHED entr(ies) no longer state their cause correctly:\n  {}\n\
         An Upstream gap waits on someone else; a LocalEnrolment gap is work \
         this repository has not done. Recording the first when it is the \
         second reads as coverage that was never won.",
        wrong.join("\n  "),
    );

    // The other direction: a runtime the workspace cannot reach with nothing
    // saying so publishes a symbol_mismatch count of zero that reads as
    // checked-and-clean.
    let named: BTreeSet<&str> = UNREACHED.iter().map(|(d, _, _)| *d).collect();
    let mut unnamed: Vec<String> = Vec::new();
    for dir in RUNTIMES {
        if named.contains(dir) {
            continue;
        }
        for ext in source_extensions(&root, dir) {
            if let Some(lang) = extensions.get(&ext) {
                if unresolved_here.contains(lang) {
                    unnamed.push(format!("{dir}: `{ext}` is `{lang}`, unresolved here"));
                }
            }
        }
    }
    unnamed.sort();
    unnamed.dedup();
    assert!(
        unnamed.is_empty(),
        "runtime(s) the symbol axis cannot reach, with no UNREACHED entry \
         saying so:\n  {}",
        unnamed.join("\n  "),
    );

    // `unreached_now` is the build-side view, kept for the message it gives
    // when a pin regresses: a language the build itself dropped.
    let regressed: Vec<&String> = unreached_now
        .iter()
        .filter(|g| !named.iter().any(|d| g.starts_with(&format!("{d}:"))))
        .collect();
    assert!(
        regressed.is_empty(),
        "the pinned build carries no resolver for these, and no entry says \
         so:\n  {}\n\
         Either the pin regressed or the entry is missing.",
        regressed
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
    );
}

/// Languages whose cites bind at file level in the scxml workspace, from
/// `validate-code-refs --json`.
///
/// The tool's own field, not a count derived from the config: enrolment is
/// per-ledger and per-language, and re-deriving it here would make this test
/// agree with a copy of the rule instead of with the run.
fn workspace_unresolved_languages(bin: &Path, root: &Path) -> BTreeSet<String> {
    let out = Command::new(bin)
        .arg("validate-code-refs")
        .arg("--json")
        .current_dir(root.join("docs/spec/scxml"))
        .output()
        .unwrap_or_else(|e| panic!("run {}: {e}", bin.display()));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "`validate-code-refs --json` did not emit JSON ({e}):\n{stdout}{}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    let axis = report
        .get("symbol_axis")
        .unwrap_or_else(|| panic!("no `symbol_axis` in the report:\n{stdout}"));
    assert!(
        axis.get("covered")
            .is_some_and(|c| !c.as_object().is_none_or(serde_json::Map::is_empty)),
        "`symbol_axis.covered` is empty — the run resolved nothing at all, which \
         is not a state this repository can be in with cpp and rust enrolled:\n{axis}"
    );
    // Absence and emptiness are different answers and must not read the same.
    // The field is `{}` in a clean run, so a reader that reached for the wrong
    // name would also come back with nothing to report — measured: a mutation
    // renaming this key survived every other assertion here, because "no
    // unresolved language" is exactly what the healthy tree says.
    axis.get("unresolved_languages")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| {
            panic!(
                "no `symbol_axis.unresolved_languages` object in the report. \
                 Its absence is not the same answer as its being empty, and \
                 this test cannot tell the two apart from the value alone:\n{axis}"
            )
        })
        .keys()
        .cloned()
        .collect()
}

/// The backends this build carries and the extensions it routes, read out of
/// `describe-symbol-axis-reach`.
fn parse_reach(text: &str) -> (BTreeSet<String>, BTreeMap<String, String>) {
    let mut resolved = BTreeSet::new();
    let mut extensions = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some((_, lang)) = line.split_once(" resolves `") {
            if let Some((lang, _)) = lang.split_once('`') {
                resolved.insert(lang.to_string());
            }
        } else if let Some((ext, lang)) = line.split_once(" -> ") {
            if ext.starts_with('.') && !ext.contains(' ') {
                extensions.insert(ext.to_string(), lang.trim().to_string());
            }
        }
    }
    (resolved, extensions)
}

/// Extensions of the hand-authored sources under one runtime tree.
fn source_extensions(root: &Path, dir: &str) -> BTreeSet<String> {
    fn walk(at: &Path, out: &mut BTreeSet<String>) {
        let Ok(entries) = fs::read_dir(at) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                out.insert(format!(".{ext}"));
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(&root.join(dir), &mut out);
    out
}

/// The rev-pinned binary, or `None` when this machine has no build of it.
///
/// Resolved the way `scripts/gates/ledger-citations.sh` resolves it — from the
/// pin in the workflow, under the revision-keyed install root — so this test
/// and the gate cannot end up asking two different binaries.
fn pinned_mnemosyne_cli(root: &Path) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("MNEMOSYNE_BIN") {
        let p = PathBuf::from(explicit);
        return p.is_file().then_some(p);
    }
    let workflow = fs::read_to_string(root.join(".github/workflows/spec-citations.yml")).ok()?;
    let rev = workflow
        .lines()
        .find_map(|l| l.trim().strip_prefix("MNEMOSYNE_REV:"))
        .map(str::trim)?;
    let home = std::env::var("HOME").ok()?;
    let bin = Path::new(&home)
        .join(".local/share/mnemosyne-rev")
        .join(&rev[..8.min(rev.len())])
        .join("bin/mnemosyne-cli");
    bin.is_file().then_some(bin)
}

/// A named symbol-axis gap still describes the tree.
///
/// Two ways it can stop doing so, and both must fail: the tree leaves the scan
/// (then it is not a symbol-axis gap, it is a coverage hole), or it stops
/// citing the spec (then the entry outlives its subject). The third — a
/// resolver landing upstream, or an enrolment landing here — is measured by
/// [`each_named_gap_still_has_the_cause_it_names`].
#[test]
fn a_named_gap_still_describes_the_tree() {
    let root = repo_root();
    let paths = scanned_paths(&root);

    let mut stale: Vec<String> = Vec::new();
    for (dir, _cause, reason) in UNREACHED {
        let reached = paths.iter().any(|p| is_under(dir, p) || is_under(p, dir));
        if !reached {
            stale.push(format!(
                "{dir}: outside the scan set entirely, so this is not a \
                 symbol-axis gap — {reason}"
            ));
            continue;
        }
        if citing_files(&root, dir).is_empty() {
            stale.push(format!("{dir}: no longer cites the spec at all — {reason}"));
        }
    }

    assert!(
        stale.is_empty(),
        "UNREACHED entr(ies) no longer describe the tree:\n  {}\n\
         A gap list that outlives its gap reads as coverage that was never \
         won; drop the entry in the commit that closes it.",
        stale.join("\n  "),
    );
}
