// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A mutation round that is killed outright does not leave its mutation behind.
//
// `scripts/mutate` hangs every restore off one `trap ... EXIT`, and a trap does
// not run on SIGKILL. Measured twice: a round died with the tree still holding
// its mutation, the next round read that file as its baseline, and a successor
// session read it as leftovers and reverted it by hand — producing a false
// CAUGHT the second time, which is a verdict that reads as a test working.
//
// The repair is a record: `scripts/lib/mutation_inflight.py` writes what the
// round is about to mutate, and where the original bytes live, to a path with
// no session in it — then the NEXT invocation finishes the restore. Every
// failure of that record reads as success, which is why each one is a test:
//
//   - a record path that moves with whoever runs the round is a session id by
//     another name, and the next invocation finds nothing to repair;
//   - a LIVE round's edits look exactly like an abandoned round's. Reverting
//     them is the failure this whole record exists to stop, so "is it still
//     running" is asked of the kernel and every uncertain answer is "alive";
//   - a snapshot that did not survive must be NAMED, not guessed at. A
//     `git checkout` here would put back HEAD, which is not what a round
//     starting from a dirty tree began with;
//   - and a round that ended without putting the tree back must keep its
//     record, or the harness deletes the evidence of its own failure.
//
// The library is driven the way `scripts/mutate` drives it, so what is measured
// is the code that runs in a round and not a copy of it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn tool() -> String {
    repo_root()
        .join("scripts/lib/mutation_inflight.py")
        .display()
        .to_string()
}

fn library() -> String {
    repo_root()
        .join("scripts/lib/mutation_ledger.sh")
        .display()
        .to_string()
}

/// Run a snippet from inside `cwd`, and return its stdout and whether it
/// succeeded. Both halves matter here: a refusal is a non-zero status AND the
/// sentence that says what it refused to guess at.
fn sh(cwd: &Path, script: &str) -> (String, bool) {
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(cwd)
        .output()
        .expect("run bash");
    assert!(
        String::from_utf8_lossy(&out.stderr).trim().is_empty()
            || !String::from_utf8_lossy(&out.stderr).contains("Traceback"),
        "the tool died rather than answering:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.success(),
    )
}

/// A tree standing in for a checkout mid-round: one declared file, a snapshot
/// holding the bytes it started from, and somewhere for the record to live.
struct Round {
    home: TempDir,
    repo: PathBuf,
    work: PathBuf,
    records: PathBuf,
}

impl Round {
    fn new() -> Self {
        let home = tempdir().expect("temp home");
        let repo = home.path().join("repo");
        let work = home.path().join("work");
        let records = home.path().join("records");
        fs::create_dir_all(work.join("snapshot")).expect("snapshot dir");
        fs::create_dir_all(&records).expect("records dir");
        fs::create_dir_all(&repo).expect("repo dir");
        Command::new("git")
            .args(["init", "-q", "."])
            .current_dir(&repo)
            .status()
            .expect("git init");
        fs::write(repo.join("f.txt"), "original\n").expect("write target");
        fs::write(work.join("snapshot/f.txt"), "original\n").expect("write snapshot");
        Round {
            home,
            repo,
            work,
            records,
        }
    }

    /// Write the record the way a round does, and return where it landed. The
    /// tool mints the name, so a caller cannot leave a half-written one behind.
    ///
    /// `alive` decides whose pid the record carries: this test process, which
    /// is running, or a shell that exits the moment it has written — an
    /// abandoned round, exactly.
    fn open(&self, alive: bool) -> PathBuf {
        let pid = if alive {
            format!("{}", std::process::id())
        } else {
            "$$".to_string()
        };
        let script = format!(
            "printf 'f.txt\\n' | bash -c 'exec python3 {tool} open \
             --dir {dir} --repo {repo} --casefile demo.cases --mode run \
             --tree deadbeef --work {work} --snapshot {work}/snapshot --pid {pid}'",
            tool = tool(),
            dir = self.records.display(),
            repo = self.repo.display(),
            work = self.work.display(),
        );
        let (out, ok) = sh(&self.repo, &script);
        assert!(ok, "opening the record failed: {out}");
        let record = PathBuf::from(out.trim());
        assert!(
            record.is_file(),
            "the tool did not print where the record landed: {out}"
        );
        let (out, ok) = sh(
            &self.repo,
            &format!(
                "python3 {} case --record {} --label 'a demo case'",
                tool(),
                record.display()
            ),
        );
        assert!(ok, "naming the case failed: {out}");
        record
    }

    fn mutate(&self) {
        fs::write(self.repo.join("f.txt"), "MUTATED\n").expect("apply the mutation");
    }

    fn target(&self) -> String {
        fs::read_to_string(self.repo.join("f.txt")).expect("read the target")
    }

    fn recover(&self) -> (String, bool) {
        sh(
            &self.repo,
            &format!(
                "python3 {} recover --dir {} --repo {}",
                tool(),
                self.records.display(),
                self.repo.display()
            ),
        )
    }

    /// Only the records themselves: the tool stages content beside them under
    /// a name it does not consider, and counting that would be counting a
    /// halfway state as a round.
    fn records_left(&self) -> usize {
        fs::read_dir(&self.records)
            .expect("read records dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".json"))
            .count()
    }
}

#[test]
fn the_in_flight_record_does_not_move_with_the_harness_that_runs_the_round() {
    let home = tempdir().expect("temp home");
    // `XDG_DATA_HOME` points somewhere that is not the answer, because the loop
    // harness driving these rounds exports its own — and the verdict ledger's
    // first shape consulted it, which put the very first round's record under a
    // directory named after the run rather than after the corpus.
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -euo pipefail; source {}; mutation_inflight_dir",
            library()
        ))
        .current_dir(repo_root())
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("decoy"))
        .env_remove("SCE_MUTATION_INFLIGHT_DIR")
        .output()
        .expect("ask the library where the record goes");
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();

    assert_eq!(
        dir,
        home.path()
            .join(".local/share/sce-mutation-corpus/in-flight")
            .display()
            .to_string(),
        "the record's path is derived from something other than HOME, so it \
         moves with whoever runs the round — and the next invocation, which is \
         the one that can repair the tree, looks somewhere else"
    );
    assert!(
        !dir.contains("decoy"),
        "the record followed XDG_DATA_HOME: {dir}"
    );
}

#[test]
fn a_killed_rounds_mutation_is_put_back_from_its_own_snapshot() {
    let round = Round::new();
    round.open(false);
    round.mutate();
    assert_eq!(round.target(), "MUTATED\n", "the setup did not mutate");

    let (report, ok) = round.recover();

    assert!(ok, "the recovery refused a repair it could make:\n{report}");
    assert_eq!(
        round.target(),
        "original\n",
        "the mutation a killed round left behind is still in the tree:\n{report}"
    );
    assert!(
        report.contains("f.txt"),
        "the repair did not say what it put back:\n{report}"
    );
    // Which round, and which case. A killed round's case is the one case that
    // reached no verdict, and a report that cannot name it sends the reader
    // looking through the whole casefile.
    assert!(
        report.contains("demo.cases") && report.contains("a demo case"),
        "the repair did not say whose mutation it was:\n{report}"
    );
    assert_eq!(
        round.records_left(),
        0,
        "the record outlived the repair, so the next invocation repeats it"
    );
    assert!(
        !round.work.exists(),
        "the round's scratch survived, which is the `rm -rf` its trap missed"
    );
}

#[test]
fn a_round_that_is_still_running_is_left_alone() {
    let round = Round::new();
    // The record carries this test process's pid, so the round it names is
    // running by construction.
    round.open(true);
    round.mutate();

    let (report, ok) = round.recover();

    assert!(ok, "a live round was treated as a failure:\n{report}");
    assert_eq!(
        round.target(),
        "MUTATED\n",
        "a LIVE round's mutation was reverted. That is the measured failure \
         this record exists to stop: reverted before the build it reads as \
         SURVIVED, reverted during it as CAUGHT, and the second reads as a \
         test doing its job:\n{report}"
    );
    assert_eq!(
        round.records_left(),
        1,
        "the live round's record was removed, so nothing will repair its tree \
         if it is killed next"
    );
    // And silent, deliberately. A round in flight is not a repair, and saying
    // so from a harness that is trying to START a round makes that harness's
    // output depend on what else is running — measured 2026-08-26, five
    // concurrent `--check` runs failing on each other's notes.
    assert!(
        report.trim().is_empty(),
        "a harness starting its own round reported on someone else's:\n{report}"
    );

    // The question itself is not dropped, it is moved: this is what a session
    // asks before it decides a dirty tree is leftovers.
    let (listing, ok) = sh(
        &round.repo,
        &format!("python3 {} list --dir {}", tool(), round.records.display()),
    );
    assert!(ok, "the listing failed:\n{listing}");
    assert!(
        listing.contains("running") && listing.contains("a demo case"),
        "nothing can tell a session that this tree's edits belong to a live \
         round, which is how two of them came to be reverted:\n{listing}"
    );
}

#[test]
fn a_snapshot_that_did_not_survive_is_named_rather_than_guessed_at() {
    let round = Round::new();
    round.open(false);
    round.mutate();
    fs::remove_dir_all(&round.work).expect("lose the snapshot");

    let (report, ok) = round.recover();

    assert!(
        !ok,
        "a repair it could not make was reported as done:\n{report}"
    );
    assert_eq!(
        round.target(),
        "MUTATED\n",
        "the file was changed without the bytes to change it back — a guess, \
         which is what `git checkout` would have been:\n{report}"
    );
    assert!(
        report.contains("f.txt") && report.contains("demo.cases"),
        "the refusal did not name what is still mutated:\n{report}"
    );
    assert_eq!(
        round.records_left(),
        1,
        "the record was removed along with the refusal, leaving no trace of a \
         tree that is still mutated"
    );

    // And it clears itself once the bytes are back, so the refusal is a state
    // the tree can leave rather than a flag someone has to remember to reset.
    fs::write(round.repo.join("f.txt"), "original\n").expect("put it back by hand");
    let (report, ok) = round.recover();
    assert!(
        ok,
        "the refusal outlived the condition that caused it:\n{report}"
    );
    assert_eq!(round.records_left(), 0, "the record survived its repair");
}

#[test]
fn a_record_from_another_checkout_is_not_acted_on() {
    let round = Round::new();
    // Same relative path, a different tree. `scripts/mutate --check` runs over
    // this corpus on every push, and a record naming another checkout must not
    // make one tree restore a file on behalf of another.
    fs::write(
        round.records.join("foreign.json"),
        format!(
            "{{\"schema\":1,\"pid\":1,\"starttime\":\"1\",\"boot_id\":\"x\",\
              \"repo\":\"/somewhere/else\",\"casefile\":\"other.cases\",\"mode\":\"run\",\
              \"work\":\"/nope\",\"snapshot\":\"/nope/snapshot\",\
              \"targets\":[{{\"path\":\"f.txt\",\"sha256\":\"{}\"}}]}}",
            "0".repeat(64)
        ),
    )
    .expect("write a foreign record");
    round.mutate();

    let (report, ok) = round.recover();

    assert!(ok, "another checkout's record failed this tree:\n{report}");
    assert_eq!(
        round.target(),
        "MUTATED\n",
        "a record belonging to another checkout was acted on here:\n{report}"
    );
    assert_eq!(
        round.records_left(),
        1,
        "another checkout's record was deleted by this tree's harness"
    );
}

#[test]
fn a_round_that_did_not_put_the_tree_back_keeps_its_record() {
    let round = Round::new();

    // Through the shell wrappers this time, because it is the EXIT trap's
    // contract that is under test: `mutation_restore_quiet` swallows its own
    // errors, so a restore that did not happen must be caught by the close —
    // and its non-zero status is what stops the trap deleting the snapshot the
    // repair will need.
    let open = format!(
        "set -euo pipefail; source {lib}; \
         mutation_inflight_open demo.cases run {work} {work}/snapshot 'f.txt'; \
         printf 'MUTATED\\n' > f.txt; \
         mutation_inflight_close && echo CLOSED_CLEAN || echo CLOSED_REFUSED",
        lib = library(),
        work = round.work.display(),
    );
    let (report, ok) = Command::new("bash")
        .arg("-c")
        .arg(&open)
        .current_dir(&round.repo)
        .env("HOME", round.home.path())
        .env("SCE_MUTATION_INFLIGHT_DIR", &round.records)
        .output()
        .map(|out| {
            (
                String::from_utf8_lossy(&out.stdout).into_owned(),
                out.status.success(),
            )
        })
        .expect("drive the shell wrappers");

    assert!(ok, "the wrappers failed outright:\n{report}");
    assert!(
        report.contains("CLOSED_REFUSED"),
        "a round whose tree is still mutated reported a clean close, which is \
         what lets the trap delete the snapshot:\n{report}"
    );
    assert!(
        report.contains("f.txt"),
        "the close did not name what was left in the tree:\n{report}"
    );
    assert_eq!(
        round.records_left(),
        1,
        "the record was removed by a close that had nothing to close"
    );

    // And the same wrappers on a tree that IS back leave nothing behind.
    fs::write(round.repo.join("f.txt"), "original\n").expect("put it back");
    let clean = format!(
        "set -euo pipefail; source {lib}; \
         mutation_inflight_open demo.cases run {work} {work}/snapshot 'f.txt'; \
         mutation_inflight_close && echo CLOSED_CLEAN",
        lib = library(),
        work = round.work.display(),
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(&clean)
        .current_dir(&round.repo)
        .env("HOME", round.home.path())
        .env("SCE_MUTATION_INFLIGHT_DIR", &round.records)
        .output()
        .expect("drive the shell wrappers");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("CLOSED_CLEAN"),
        "a round that put the tree back could not close its record:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        round.records_left(),
        1,
        "the clean close left its own record behind (the one still here is the \
         refused one from the first half)"
    );
}
