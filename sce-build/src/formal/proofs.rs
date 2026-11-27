// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The judge of a proofs manifest (docs/adr/0115, D10 and D13; docs/adr/0146,
//! D1): every Rust file with a contract verified by the pinned Verus as the
//! manifest declares.
//!
//! - A file is PROVED when Verus verifies exactly the number of functions the
//!   manifest records, with no error and no warning. A contract that was taken
//!   out leaves fewer.
//! - A proof that cannot fail shows nothing, so each MUTANT is a change of the
//!   code that breaks something a user relies on: it must compile, and Verus must
//!   refuse it.
//! - What is proved is what runs: the file is compiled by the module the
//!   manifest names, and nothing in it is taken on trust (an assumption, an
//!   admitted proof, a body Verus does not read).
//!
//! The manifest is complete over its repository: every file that carries a
//! contract is declared. Its paths are read from the root of the Git work tree
//! that holds the manifest.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use super::toolchain::{self, VerusPin};
use super::{FormalError, WorkDir};

/// The surface's stability (docs/adr/0146, D5; `SCE_WIRE_CONTRACTS.md`), which the
/// header of `schemas/sce-formal-proofs.v1.schema.json` repeats.
pub const PROOFS_SCHEMA_STATUS: &str = "pre-release";

const PROOF_KEYS: &[&str] = &["file", "compiled_by", "decision", "verified", "mutants"];
const MUTANT_KEYS: &[&str] = &["breaks", "from", "to"];

/// A proofs manifest, as `schemas/sce-formal-proofs.v1.schema.json` describes it.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub v: Option<Value>,
    #[serde(default)]
    pub proofs: Vec<BTreeMap<String, Value>>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// One declared proof, read from its entry once the declaration holds.
#[derive(Debug, Clone)]
pub struct Proof {
    pub file: String,
    pub verified: u64,
    pub mutants: Vec<Mutant>,
}

/// One change of a proved file that Verus must refuse.
#[derive(Debug, Clone)]
pub struct Mutant {
    pub breaks: String,
    pub from: String,
    pub to: String,
}

/// What makes Verus take something on trust.
static TRUSTED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(assume|admit|assume_specification|external_body|external_fn_specification|external_type_specification|external_trait_specification|axiom)\b|verifier::external|exec_allows_no_decreases_clause",
    )
    .expect("a valid expression")
});
static RESULTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"verification results:: (\d+) verified, (\d+) errors").expect("a valid expression")
});
static WARNING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(warning|WARNING)").expect("a valid expression"));
static JOBSERVER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"--jobserver-(?:auth|fds)=(\d+),(\d+)").expect("a valid expression")
});
/// The variables rustc looks for a jobserver in, in the order it reads them.
const JOBSERVER_VARIABLES: &[&str] = &["CARGO_MAKEFLAGS", "MAKEFLAGS", "MFLAGS"];

/// Read a manifest's text.
pub fn parse(text: &str) -> Result<Manifest, String> {
    serde_json::from_str(text).map_err(|e| format!("not a proofs manifest: {e}"))
}

fn text_of(entry: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    entry.get(key).and_then(Value::as_str).map(str::to_string)
}

/// What is wrong with a run of a proof that must hold, or None.
pub fn judged(output: &str, status: i32, verified: u64) -> Option<String> {
    let Some(found) = RESULTS.captures(output) else {
        return Some("Verus judged nothing: the file does not compile".to_string());
    };
    let errors: u64 = found[2].parse().unwrap_or(u64::MAX);
    if errors != 0 || status != 0 {
        return Some(format!("{} error(s)", &found[2]));
    }
    if found[1].parse::<u64>().ok() != Some(verified) {
        return Some(format!(
            "{} verified where {verified} are recorded",
            &found[1]
        ));
    }
    if WARNING.is_match(output) {
        return Some("a warning".to_string());
    }
    None
}

/// What is wrong with a run of a mutant, which must be refused, or None.
pub fn mutant_judged(output: &str) -> Option<String> {
    let Some(found) = RESULTS.captures(output) else {
        return Some("Verus judged nothing: the mutant does not compile".to_string());
    };
    (&found[2] == "0")
        .then(|| "the proof holds of it, so it says nothing of what the mutant breaks".to_string())
}

/// What the manifest and the files it declares get wrong, with no Verus.
pub fn declared(manifest: &Manifest, root: &Path, contracted: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    if manifest.v != Some(Value::from(1)) {
        let given = manifest
            .v
            .as_ref()
            .map_or("none".to_string(), Value::to_string);
        problems.push(format!(
            "the manifest is of version {given}, and this judge reads version 1 (\"v\": 1)"
        ));
    }
    for key in manifest.other.keys() {
        problems.push(format!(
            "the manifest names {key:?}, which it does not declare: a tool is pinned by the judge (docs/adr/0146, D3)"
        ));
    }
    for entry in &manifest.proofs {
        let name = text_of(entry, "file").unwrap_or_else(|| "?".to_string());
        for key in PROOF_KEYS {
            if !entry.contains_key(*key) {
                problems.push(format!("{name}: missing key {key}"));
            }
        }
        for key in entry.keys() {
            if !PROOF_KEYS.contains(&key.as_str()) {
                problems.push(format!("{name}: unknown key {key}"));
            }
        }
        if entry.get("verified").is_some_and(|v| v.as_u64().is_none()) {
            problems.push(format!("{name}: verified is a count"));
        }
        let path = root.join(&name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            problems.push(format!("{name}: no such file"));
            continue;
        };
        for (at, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if TRUSTED.is_match(code) {
                problems.push(format!(
                    "{name}:{}: taken on trust: {}",
                    at + 1,
                    line.trim()
                ));
            }
        }
        let compiled_by = text_of(entry, "compiled_by").unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let declares = Regex::new(&format!(
            r"(?m)^\s*(pub(\([a-z]+\))? )?mod {};",
            regex::escape(&stem)
        ))
        .expect("a valid expression");
        let module = std::fs::read_to_string(root.join(&compiled_by)).unwrap_or_default();
        if compiled_by.is_empty() || !declares.is_match(&module) {
            problems.push(format!(
                "{name}: {compiled_by} does not compile it, so the proof is of nothing that runs"
            ));
        }
        let mutants = entry
            .get("mutants")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if mutants.is_empty() {
            problems.push(format!(
                "{name}: no mutant, so nothing shows that the proof can fail"
            ));
        }
        for mutant in &mutants {
            let keys: Vec<String> = mutant
                .as_object()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            let mut sorted_keys = keys.clone();
            sorted_keys.sort();
            let mut wanted: Vec<String> = MUTANT_KEYS.iter().map(|k| k.to_string()).collect();
            wanted.sort();
            if sorted_keys != wanted {
                problems.push(format!("{name}: a mutant has the keys {sorted_keys:?}"));
                continue;
            }
            let field = |key: &str| mutant[key].as_str().unwrap_or("").to_string();
            let (breaks, from, to) = (field("breaks"), field("from"), field("to"));
            let times = text.matches(from.as_str()).count();
            if from.is_empty() || times != 1 {
                problems.push(format!(
                    "{name}: mutant \"{breaks}\": its text is there {times} times"
                ));
            }
            if from == to {
                problems.push(format!("{name}: mutant \"{breaks}\": changes nothing"));
            }
        }
    }
    let names: Vec<String> = manifest
        .proofs
        .iter()
        .filter_map(|entry| text_of(entry, "file"))
        .collect();
    for name in contracted {
        if !names.contains(name) {
            problems.push(format!(
                "{name}: carries a contract, and the manifest does not declare it"
            ));
        }
    }
    problems
}

/// The root of the Git work tree that holds `manifest`, which a proofs manifest's
/// paths are read from.
pub fn root_of(manifest: &Path) -> Result<PathBuf, FormalError> {
    let dir = manifest
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let ran = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| FormalError::ToolUnavailable {
            tool: "git".to_string(),
            detail: format!(
                "cannot run git to find the work tree of {}: {e}",
                manifest.display()
            ),
        })?;
    if !ran.status.success() {
        return Err(FormalError::Declaration {
            manifest: manifest.display().to_string(),
            problems: vec![format!(
                "is in no Git work tree, and a proofs manifest is read from the root of the one that holds it: {}",
                String::from_utf8_lossy(&ran.stderr).trim()
            )],
        });
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&ran.stdout).trim()))
}

/// What a file that carries a contract Verus reads holds: the attribute its contracts stand
/// under. Spelled in two halves, so this judge's own source, which names it, is not one.
const CONTRACT_MARK: &str = concat!("cfg_attr(verus", "_only");

/// Every Rust file of the work tree at `root` that Verus would read a contract of.
pub fn contracted_files(root: &Path) -> Result<Vec<String>, FormalError> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["grep", "-l", "--untracked", CONTRACT_MARK, "--", "*.rs"])
        .output()
        .map_err(|e| FormalError::ToolUnavailable {
            tool: "git".to_string(),
            detail: format!("cannot run git grep in {}: {e}", root.display()),
        })?;
    match ran.status.code() {
        Some(0) | Some(1) => Ok(String::from_utf8_lossy(&ran.stdout)
            .split_whitespace()
            .map(str::to_string)
            .collect()),
        _ => Err(FormalError::ToolUnavailable {
            tool: "git".to_string(),
            detail: format!("git grep: {}", String::from_utf8_lossy(&ran.stderr).trim()),
        }),
    }
}

fn descriptor_is_open(fd: u32) -> bool {
    Path::new(&format!("/proc/self/fd/{fd}")).exists()
}

/// The jobserver variables of `environ` as Verus is to see them: a pair of
/// descriptors that is open here is kept, since the child inherits it, and one
/// that is not is taken out, since no child can inherit what this process does
/// not hold; rustc inside Verus would warn that the build is misconfigured, and
/// Verus reports that warning as an internal error. A jobserver named by a fifo
/// needs no descriptor and is left as it is.
pub fn jobserver_handoff(
    environ: &[(String, String)],
    is_open: impl Fn(u32) -> bool,
) -> Vec<(String, String)> {
    environ
        .iter()
        .filter(|(name, _)| JOBSERVER_VARIABLES.contains(&name.as_str()))
        .map(|(name, value)| {
            let kept = JOBSERVER.replace_all(value, |found: &regex::Captures<'_>| {
                let open = [&found[1], &found[2]]
                    .iter()
                    .all(|fd| fd.parse().is_ok_and(&is_open));
                if open {
                    found[0].to_string()
                } else {
                    String::new()
                }
            });
            (name.clone(), kept.into_owned())
        })
        .collect()
}

fn run_verus(tool: &Path, file: &Path) -> Result<(i32, String), FormalError> {
    let environ: Vec<(String, String)> = std::env::vars().collect();
    let mut command = Command::new(tool);
    command
        .arg("--crate-type=lib")
        .arg(file)
        .current_dir(file.parent().unwrap_or(Path::new(".")));
    for (name, value) in jobserver_handoff(&environ, descriptor_is_open) {
        command.env(name, value);
    }
    let ran = command.output().map_err(|e| FormalError::ToolUnavailable {
        tool: "Verus".to_string(),
        detail: format!("cannot run {}: {e}", tool.display()),
    })?;
    Ok((
        ran.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        ),
    ))
}

fn proofs_of(manifest: &Manifest) -> Vec<Proof> {
    manifest
        .proofs
        .iter()
        .map(|entry| Proof {
            file: text_of(entry, "file").unwrap_or_default(),
            verified: entry.get("verified").and_then(Value::as_u64).unwrap_or(0),
            mutants: entry
                .get("mutants")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|m| Mutant {
                    breaks: m["breaks"].as_str().unwrap_or("").to_string(),
                    from: m["from"].as_str().unwrap_or("").to_string(),
                    to: m["to"].as_str().unwrap_or("").to_string(),
                })
                .collect(),
        })
        .collect()
}

/// Check every proof `manifest_path` declares. Each run's line goes to `report`
/// as it is judged.
pub fn check(
    manifest_path: &Path,
    report: &mut super::models::Report<'_>,
) -> Result<(), FormalError> {
    let shown = manifest_path.display().to_string();
    let text = std::fs::read_to_string(manifest_path).map_err(|e| FormalError::Declaration {
        manifest: shown.clone(),
        problems: vec![format!("cannot be read: {e}")],
    })?;
    let manifest = parse(&text).map_err(|problem| FormalError::Declaration {
        manifest: shown.clone(),
        problems: vec![problem],
    })?;
    let root = root_of(manifest_path)?;
    let problems = declared(&manifest, &root, &contracted_files(&root)?);
    if !problems.is_empty() {
        for problem in &problems {
            (report.out)(&format!("FAIL {problem}"));
        }
        return Err(FormalError::Declaration {
            manifest: shown,
            problems,
        });
    }
    let pin: VerusPin = toolchain::pinned().verus;
    let tool = toolchain::admitted_verus(&pin)?;
    let mut failures = Vec::new();
    let mut runs = 0usize;
    for proof in proofs_of(&manifest) {
        let source = root.join(&proof.file);
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let body = std::fs::read_to_string(&source).map_err(|e| FormalError::Declaration {
            manifest: shown.clone(),
            problems: vec![format!("{}: cannot be read: {e}", proof.file)],
        })?;
        let scratch = WorkDir::new("verus-proof")?;
        let copy = scratch.path().join(&name);
        let mut attempts: Vec<(Option<String>, String)> = vec![(None, body.clone())];
        attempts.extend(
            proof
                .mutants
                .iter()
                .map(|m| (Some(m.breaks.clone()), body.replacen(&m.from, &m.to, 1))),
        );
        for (breaks, text) in attempts {
            runs += 1;
            std::fs::write(&copy, &text).map_err(|e| FormalError::ToolUnavailable {
                tool: "a working directory".to_string(),
                detail: format!("cannot write {}: {e}", copy.display()),
            })?;
            let (status, output) = run_verus(&tool, &copy)?;
            let (what, wrong) = match &breaks {
                None => (
                    format!("{name}: {} verified", proof.verified),
                    judged(&output, status, proof.verified),
                ),
                Some(breaks) => (format!("{name}: refused: {breaks}"), mutant_judged(&output)),
            };
            match &wrong {
                None => (report.out)(&format!("ok   {what}")),
                Some(why) => {
                    (report.out)(&format!("FAIL {what}: {why}"));
                    let lines: Vec<&str> = output.lines().collect();
                    (report.detail)(&lines[lines.len().saturating_sub(30)..].join("\n"));
                    failures.push(format!("{what}: {why}"));
                }
            }
        }
    }
    if !failures.is_empty() {
        return Err(FormalError::Verdict {
            manifest: shown,
            runs,
            failures,
        });
    }
    (report.out)("every proof is as declared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const BODY: &str = "pub fn f(x: u8) -> u8 {\n    x + 1\n}\n";

    struct Fixture {
        dir: WorkDir,
    }

    impl Fixture {
        fn new() -> Self {
            let fixture = Self {
                dir: WorkDir::new("test-proofs").expect("a working directory"),
            };
            std::fs::create_dir_all(fixture.root().join("src")).expect("src");
            fixture.put("src/lib.rs", "pub mod core;\n");
            fixture.put("src/core.rs", BODY);
            fixture
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn put(&self, name: &str, text: &str) {
            std::fs::write(self.root().join(name), text).expect("written");
        }

        fn manifest(&self) -> Value {
            json!({"v": 1, "proofs": [{
                "file": "src/core.rs", "compiled_by": "src/lib.rs", "decision": "docs/adr/0115", "verified": 1,
                "mutants": [{"breaks": "it adds nothing", "from": "x + 1", "to": "x"}],
            }]})
        }

        fn declared(&self, manifest: &Value, contracted: &[&str]) -> Vec<String> {
            let contracted: Vec<String> = contracted.iter().map(|s| s.to_string()).collect();
            declared(
                &serde_json::from_value(manifest.clone()).expect("a manifest"),
                self.root(),
                &contracted,
            )
        }

        fn finds(&self, manifest: &Value, needle: &str) -> bool {
            self.declared(manifest, &["src/core.rs"])
                .iter()
                .any(|p| p.contains(needle))
        }
    }

    #[test]
    fn a_whole_declaration_is_clean() {
        let f = Fixture::new();
        assert_eq!(
            f.declared(&f.manifest(), &["src/core.rs"]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_file_with_a_contract_nobody_declared_is_found() {
        let f = Fixture::new();
        assert!(f
            .declared(&f.manifest(), &["src/core.rs", "src/other.rs"])
            .iter()
            .any(|p| p.contains("src/other.rs")));
    }

    #[test]
    fn a_tool_named_in_the_manifest_is_refused() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["verus"] = json!({"version": "0"});
        assert!(f.finds(&manifest, "pinned by the judge"));
    }

    #[test]
    fn what_is_taken_on_trust_is_found() {
        let f = Fixture::new();
        for line in [
            "    assume(x < 255);",
            "#[verifier::external_body]",
            "    admit();",
        ] {
            f.put(
                "src/core.rs",
                &format!("{BODY}// assume nothing here\n{line}\n"),
            );
            let found: Vec<String> = f
                .declared(&f.manifest(), &["src/core.rs"])
                .into_iter()
                .filter(|p| p.contains("taken on trust"))
                .collect();
            assert_eq!(found.len(), 1, "{line}: {found:?}");
        }
    }

    #[test]
    fn a_proof_of_a_file_nothing_compiles_is_found() {
        let f = Fixture::new();
        f.put("src/lib.rs", "pub mod other;\n");
        assert!(f.finds(&f.manifest(), "does not compile it"));
    }

    #[test]
    fn a_proof_with_no_mutant_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["proofs"][0]["mutants"] = json!([]);
        assert!(f.finds(&manifest, "no mutant"));
    }

    #[test]
    fn a_mutant_whose_text_is_not_there_once_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["proofs"][0]["mutants"][0]["from"] = json!("x * 2");
        assert!(f.finds(&manifest, "its text is there 0 times"));
        manifest["proofs"][0]["mutants"][0]["from"] = json!("x");
        assert!(f.finds(&manifest, "times"));
    }

    #[test]
    fn a_mutant_that_changes_nothing_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["proofs"][0]["mutants"][0]["to"] = json!("x + 1");
        assert!(f.finds(&manifest, "changes nothing"));
    }

    #[test]
    fn a_missing_or_unknown_key_is_found() {
        let f = Fixture::new();
        let mut manifest = f.manifest();
        manifest["proofs"][0]
            .as_object_mut()
            .expect("an entry")
            .remove("decision");
        manifest["proofs"][0]["owner"] = json!("x");
        assert!(f.finds(&manifest, "missing key decision"));
        assert!(f.finds(&manifest, "unknown key owner"));
    }

    #[test]
    fn a_proof_judged_by_its_results() {
        assert_eq!(
            judged("verification results:: 11 verified, 0 errors", 0, 11),
            None
        );
        assert!(judged("verification results:: 10 verified, 0 errors", 0, 11).is_some());
        assert!(judged("verification results:: 11 verified, 1 errors", 1, 11).is_some());
        assert!(judged(
            "warning: unused\nverification results:: 11 verified, 0 errors",
            0,
            11
        )
        .is_some());
        assert!(judged("error[E0425]", 1, 11).is_some());
    }

    #[test]
    fn a_mutant_must_be_refused() {
        assert_eq!(
            mutant_judged("verification results:: 10 verified, 1 errors"),
            None
        );
        assert!(mutant_judged("verification results:: 11 verified, 0 errors").is_some());
        assert!(mutant_judged("error: expected expression").is_some());
    }

    #[test]
    fn a_jobserver_reaches_verus_whole_or_not_at_all() {
        let environ = vec![
            (
                "MAKEFLAGS".to_string(),
                "-j4 --jobserver-auth=3,4".to_string(),
            ),
            (
                "CARGO_MAKEFLAGS".to_string(),
                "--jobserver-fds=8,9 -j".to_string(),
            ),
            (
                "MFLAGS".to_string(),
                "--jobserver-auth=fifo:/tmp/js".to_string(),
            ),
            ("PATH".to_string(), "/bin".to_string()),
        ];
        let handed: BTreeMap<String, String> = jobserver_handoff(&environ, |fd| fd == 3 || fd == 4)
            .into_iter()
            .collect();
        assert_eq!(handed["MAKEFLAGS"], "-j4 --jobserver-auth=3,4");
        assert_eq!(handed["CARGO_MAKEFLAGS"], " -j");
        assert_eq!(handed["MFLAGS"], "--jobserver-auth=fifo:/tmp/js");
        assert!(!handed.contains_key("PATH"));
    }

    #[test]
    fn the_status_of_the_surface_is_the_schemas() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/sce-formal-proofs.v1.schema.json"
        ))
        .expect("the schema is JSON");
        assert_eq!(
            schema["x-sce-schema-status"],
            Value::from(PROOFS_SCHEMA_STATUS)
        );
    }
}
