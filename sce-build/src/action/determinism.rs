// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An action's result on this machine held against another machine's result
//! for the same key (ADR 0097, section 5): what the `generated-deterministic`
//! gate judges.
//!
//! An action key names everything an action may read and the generator that
//! runs it, so two machines that compute one key must write the same files
//! with the same bytes. A fleet entry of the same key that holds other files
//! is either an action that is not a function of its declared inputs or a
//! fleet that holds a wrong entry, and either would spread to every machine
//! that takes it. Two results agree when they name the same files, each with
//! the same digest and executable bit, and end with the same exit code.

use std::collections::BTreeMap;

use super::reapi::{ActionResult, Digest};

/// How one result differs from the other's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Divergence {
    /// A file only this machine's result names.
    OnlyHere(String),
    /// A file only the other machine's result names.
    OnlyThere(String),
    /// A file both name, with different bytes.
    Bytes(String),
    /// A file both name with the same bytes, executable on one side only.
    Executable(String),
    /// The two runs ended with different exit codes.
    ExitCode { here: i32, there: i32 },
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Divergence::OnlyHere(p) => write!(f, "written here only:       {p}"),
            Divergence::OnlyThere(p) => write!(f, "written there only:      {p}"),
            Divergence::Bytes(p) => write!(f, "different bytes:         {p}"),
            Divergence::Executable(p) => write!(f, "different mode:          {p}"),
            Divergence::ExitCode { here, there } => {
                write!(f, "different exit code:     {here} here, {there} there")
            }
        }
    }
}

/// What the other machine holds for a key this machine ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Agreement {
    /// The same files with the same bytes.
    Agrees,
    /// No entry: nothing to compare, which is no verdict either way.
    Absent,
    /// An entry that differs, and how.
    Differs(Vec<Divergence>),
}

type Outputs<'a> = BTreeMap<&'a str, (Option<&'a Digest>, bool)>;

fn outputs(result: &ActionResult) -> Outputs<'_> {
    result
        .output_files
        .iter()
        .map(|f| (f.path.as_str(), (f.digest.as_ref(), f.is_executable)))
        .collect()
}

/// Hold `here`, the result this machine's run recorded, against `there`, the
/// other machine's entry for the same key.
pub fn compare(here: &ActionResult, there: Option<&ActionResult>) -> Agreement {
    let Some(there) = there else {
        return Agreement::Absent;
    };
    let (mine, theirs) = (outputs(here), outputs(there));
    let mut divergences = Vec::new();
    for (path, (digest, executable)) in &mine {
        match theirs.get(path) {
            None => divergences.push(Divergence::OnlyHere((*path).to_string())),
            Some((other, _)) if other != digest => {
                divergences.push(Divergence::Bytes((*path).to_string()))
            }
            Some((_, other)) if other != executable => {
                divergences.push(Divergence::Executable((*path).to_string()))
            }
            Some(_) => {}
        }
    }
    for path in theirs.keys().filter(|p| !mine.contains_key(*p)) {
        divergences.push(Divergence::OnlyThere((*path).to_string()));
    }
    if here.exit_code != there.exit_code {
        divergences.push(Divergence::ExitCode {
            here: here.exit_code,
            there: there.exit_code,
        });
    }
    if divergences.is_empty() {
        Agreement::Agrees
    } else {
        Agreement::Differs(divergences)
    }
}

/// The verdict over every action of a run: how many agreed, how many the
/// other machine held no entry for, and each that differed, by its key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Verdict {
    pub agreed: usize,
    pub absent: usize,
    pub differed: Vec<(Digest, Vec<Divergence>)>,
}

impl Verdict {
    /// Count `agreement` for the action `key`.
    pub fn count(&mut self, key: &Digest, agreement: Agreement) {
        match agreement {
            Agreement::Agrees => self.agreed += 1,
            Agreement::Absent => self.absent += 1,
            Agreement::Differs(divergences) => self.differed.push((key.clone(), divergences)),
        }
    }

    /// Whether every entry the other machine held agreed. A run that found no
    /// entry at all passes too, and [`Verdict::compared`] says so.
    pub fn passed(&self) -> bool {
        self.differed.is_empty()
    }

    /// How many actions had an entry to compare against.
    pub fn compared(&self) -> usize {
        self.agreed + self.differed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::super::reapi::OutputFile;
    use super::*;

    fn result(files: &[(&str, &[u8], bool)]) -> ActionResult {
        ActionResult {
            output_files: files
                .iter()
                .map(|(path, bytes, is_executable)| OutputFile {
                    path: (*path).to_string(),
                    digest: Some(Digest::of(bytes)),
                    is_executable: *is_executable,
                })
                .collect(),
            ..ActionResult::default()
        }
    }

    #[test]
    fn the_same_files_with_the_same_bytes_agree_whatever_their_order() {
        let here = result(&[("sce-out/a.rs", b"a", false), ("sce-out/b.rs", b"b", false)]);
        let there = result(&[("sce-out/b.rs", b"b", false), ("sce-out/a.rs", b"a", false)]);
        assert_eq!(compare(&here, Some(&there)), Agreement::Agrees);
    }

    #[test]
    fn no_entry_there_is_no_verdict() {
        let here = result(&[("sce-out/a.rs", b"a", false)]);
        assert_eq!(compare(&here, None), Agreement::Absent);
    }

    /// Every way two results of one key can differ is named, each once.
    #[test]
    fn every_divergence_is_named() {
        let here = result(&[
            ("sce-out/same.rs", b"s", false),
            ("sce-out/bytes.rs", b"1", false),
            ("sce-out/mode.sh", b"m", true),
            ("sce-out/mine.rs", b"x", false),
        ]);
        let mut there = result(&[
            ("sce-out/same.rs", b"s", false),
            ("sce-out/bytes.rs", b"2", false),
            ("sce-out/mode.sh", b"m", false),
            ("sce-out/theirs.rs", b"y", false),
        ]);
        there.exit_code = 1;
        let Agreement::Differs(found) = compare(&here, Some(&there)) else {
            panic!("two different results agreed");
        };
        assert_eq!(
            found,
            vec![
                Divergence::Bytes("sce-out/bytes.rs".into()),
                Divergence::OnlyHere("sce-out/mine.rs".into()),
                Divergence::Executable("sce-out/mode.sh".into()),
                Divergence::OnlyThere("sce-out/theirs.rs".into()),
                Divergence::ExitCode { here: 0, there: 1 },
            ]
        );
    }

    /// A verdict passes only when no entry differed, and says how many it
    /// compared, so a run that found nothing to compare reads as such.
    #[test]
    fn a_verdict_fails_on_one_difference_and_counts_what_it_compared() {
        let here = result(&[("sce-out/a.rs", b"a", false)]);
        let other = result(&[("sce-out/a.rs", b"b", false)]);
        let mut verdict = Verdict::default();
        verdict.count(&Digest::of(b"k1"), compare(&here, Some(&here)));
        verdict.count(&Digest::of(b"k2"), compare(&here, None));
        assert!(verdict.passed());
        assert_eq!((verdict.compared(), verdict.absent), (1, 1));
        verdict.count(&Digest::of(b"k3"), compare(&here, Some(&other)));
        assert!(!verdict.passed());
        assert_eq!(verdict.differed[0].0, Digest::of(b"k3"));
        assert_eq!(verdict.compared(), 2);
    }
}
