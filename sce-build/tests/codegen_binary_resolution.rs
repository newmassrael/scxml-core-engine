// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every harness must reach the sce-codegen binary without naming a
//! cargo build profile.
//!
//! The profile used to be spelled out independently at ~100 sites
//! across five languages. Moving the generator's build from release to
//! debug moved the workflow's build and upload steps, the 46 regen
//! scripts and the CMake search path — and left the conformance
//! harnesses (Python, Go, Kotlin, C, C++) looking for a release binary
//! nothing produced any more, plus a download step that put the
//! artifact somewhere other than where the next step chmod'd it. Every
//! `SCE Forge Numerical Conformance` job died before running a single
//! assertion.
//!
//! Resolution now lives in four ecosystem locators —
//! `scripts/lib/sce_codegen.sh`, `cmake/SCEFindCodegen.cmake`,
//! `gradle/sce-codegen.gradle.kts` and
//! `backends/python/forge-runtime/tests/_sce_codegen.py` — which are
//! the only files allowed to know that `target/release` can hold a
//! binary at all. These tests pin that, the workflow's artifact path
//! agreement, and the fact that the fallback actually works.

use std::path::{Path, PathBuf};

/// The four files allowed to name the release profile: they are what
/// keeps an existing release build usable now that nothing produces one.
const LOCATORS: &[&str] = &[
    "scripts/lib/sce_codegen.sh",
    "cmake/SCEFindCodegen.cmake",
    "gradle/sce-codegen.gradle.kts",
    "backends/python/forge-runtime/tests/_sce_codegen.py",
    // This gate names the path it forbids, in prose and in the needle.
    "sce-build/tests/codegen_binary_resolution.rs",
];

/// Enough files must reach the scan for a clean result to mean
/// anything; a filter that accidentally matched nothing would
/// otherwise pass as loudly as a clean tree. The tree tracked ~2600
/// files when this bound was measured.
const MIN_SCANNED_FILES: usize = 1500;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Tracked files only — `git ls-files` is the enumeration source so
/// gitignored artifacts (build trees, generated probe sources) never
/// enter the gate and an untracked scratch file cannot red CI.
fn tracked_files(root: &Path) -> Vec<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files must succeed");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

/// `(line number, text)` for the lines of `text` that are not wholly a
/// comment, in `#` and `//` dialects — which is every file type that
/// mentions the generator here (YAML, CMake, shell, Rust, Kotlin, Python).
///
/// Both scans below are about where the binary is *resolved*, and a
/// sentence describing a resolution does not perform one. Without this the
/// gates fire on the comments explaining why a site was consolidated,
/// which pushes the next author to describe the fix vaguely or not at all
/// — a scanner that punishes documentation gets less documentation.
///
/// A trailing comment after code stays in, deliberately: that line still
/// carries the code.
fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines().enumerate().filter_map(|(i, line)| {
        let trimmed = line.trim_start();
        (!trimmed.starts_with('#') && !trimmed.starts_with("//")).then_some((i + 1, line))
    })
}

/// Nothing outside the locators may name the release profile.
///
/// Stated as a ban on `target/release/sce-codegen` rather than on
/// "naming any profile" because the two profiles are not symmetric:
/// debug is what every build path in this repository produces, so a
/// file naming it is at worst redundant, while a file naming release
/// is naming a binary that will not exist on a tree built today. That
/// asymmetry is exactly what broke the conformance jobs.
#[test]
fn nothing_outside_the_locators_names_the_release_profile() {
    let root = repo_root();
    let needle = "target/release/sce-codegen";
    let mut scanned = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for rel in tracked_files(&root) {
        if LOCATORS.contains(&rel.as_str()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue; // binary or non-UTF-8 file
        };
        scanned += 1;
        for (lineno, line) in code_lines(&text) {
            if line.contains(needle) {
                violations.push(format!("{rel}:{lineno}: {}", line.trim()));
            }
        }
    }

    assert!(
        scanned >= MIN_SCANNED_FILES,
        "only {scanned} tracked text files reached the scan (expected at \
         least {MIN_SCANNED_FILES}); the enumeration broke, so a clean \
         result would prove nothing",
    );
    assert!(
        violations.is_empty(),
        "{} site(s) name the release profile of the generator, which no \
         build path in this repository produces. Resolve it through the \
         ecosystem locator instead ({}):\n{}",
        violations.len(),
        LOCATORS[..4].join(", "),
        violations.join("\n"),
    );
}

/// The same ban, in the spelling the check above cannot see.
///
/// `nothing_outside_the_locators_names_the_release_profile` looks for
/// `target/release/sce-codegen` — the directory and the binary adjacent on
/// one line. That is how a shell script writes it and how a workflow
/// writes it, and it is *not* how CMake writes it: `find_program` takes
/// the name in one argument and the directories in another, so the two
/// halves land on different lines and the needle never matches.
///
/// The gap was not theoretical. Four sites lived inside it, and two of
/// them were the ones that mattered most — the W3C static-test module and
/// the root list's install rule — each carrying its own `find_program`
/// with the profiles in the opposite order to the locator's, so a stale
/// release binary outranked a fresh debug one on exactly the lane where a
/// months-old generator was found generating in the first place.
///
/// So this scans for the directory alone, in any file that also mentions
/// the generator. Restricting it to those files is what keeps it from
/// firing on the many unrelated uses of a release profile in a Rust
/// workspace; a file that never names `sce-codegen` is not resolving one.
#[test]
fn nothing_outside_the_locators_reaches_into_a_profile_directory() {
    let root = repo_root();
    let profile_dir = "target/release";
    let generator = "sce-codegen";
    let mut scanned = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for rel in tracked_files(&root) {
        if LOCATORS.contains(&rel.as_str()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue; // binary or non-UTF-8 file
        };
        scanned += 1;
        if !text.contains(generator) {
            continue;
        }
        for (lineno, line) in code_lines(&text) {
            if line.contains(profile_dir) {
                violations.push(format!("{rel}:{lineno}: {}", line.trim()));
            }
        }
    }

    assert!(
        scanned >= MIN_SCANNED_FILES,
        "only {scanned} tracked text files reached the scan (expected at \
         least {MIN_SCANNED_FILES}); the enumeration broke, so a clean \
         result would prove nothing",
    );
    assert!(
        violations.is_empty(),
        "{} site(s) reach into a build-profile directory to find the \
         generator. The profile is a build-layout detail, and a second \
         copy of the search is also a second copy of the profile ORDER — \
         which is what decides whether a stale binary outranks a fresh \
         one. Resolve through the ecosystem locator instead ({}):\n{}",
        violations.len(),
        LOCATORS[..4].join(", "),
        violations.join("\n"),
    );
}

/// The conformance workflow must download the generator into the
/// directory the next step makes executable.
///
/// Upload names a file, download names a directory, and chmod names a
/// file again — three independent spellings of one path, none of which
/// the YAML checks against the others. When the build moved to debug,
/// upload and chmod moved and download did not: the artifact landed in
/// `target/release/` and `chmod +x target/debug/sce-codegen` failed
/// with "No such file or directory" in all four consumer jobs.
#[test]
fn the_conformance_workflow_downloads_the_generator_where_it_chmods_it() {
    let root = repo_root();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/forge-conformance.yml"))
        .expect("read forge-conformance.yml");

    let upload: Vec<&str> = workflow
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("path: target/") && l.ends_with("/sce-codegen"))
        .collect();
    assert_eq!(
        upload.len(),
        1,
        "expected exactly one upload path naming the binary; found {upload:?}. \
         This test reads the workflow's shape, so a shape change needs \
         re-aiming rather than a silent pass",
    );
    let uploaded = upload[0].trim_start_matches("path: ").to_string();

    let download_dirs: Vec<String> = workflow
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("path: target/") && !l.ends_with("/sce-codegen"))
        .map(|l| l.trim_start_matches("path: ").to_string())
        .collect();
    let chmod_paths: Vec<String> = workflow
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("run: chmod +x "))
        .map(str::to_string)
        .collect();

    assert!(
        !download_dirs.is_empty() && download_dirs.len() == chmod_paths.len(),
        "every job that downloads the generator must also make it \
         executable; found {} download path(s) and {} chmod step(s)",
        download_dirs.len(),
        chmod_paths.len(),
    );

    let mut violations: Vec<String> = Vec::new();
    for (dir, chmod) in download_dirs.iter().zip(chmod_paths.iter()) {
        let landed = format!("{dir}/sce-codegen");
        if &landed != chmod {
            violations.push(format!(
                "download `path: {dir}` lands the artifact at {landed}, but \
                 the job chmods {chmod}"
            ));
        }
        if landed != uploaded {
            violations.push(format!(
                "download `path: {dir}` lands the artifact at {landed}, but \
                 the build job uploaded {uploaded}"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "the generator artifact is downloaded somewhere other than where it \
         is used:\n{}",
        violations.join("\n"),
    );
}

// The conformance lanes' `| tee` steps used to be pinned here by a test
// scoped to `forge-conformance.yml` alone, which recorded in its own doc
// that `w3c-tests.yml` carried the same shape at sites it did not read.
// Counting the shape over every workflow subsumes it:
// `test_result_gating::every_pipeline_in_a_run_script_reports_its_own_failure`.

/// A cached generator path that no longer exists must not survive.
///
/// `find_program` writes `SCE_CODEGEN` into `CMakeCache.txt`; CI restores
/// that cache between runs; and the build step that precedes configure
/// deletes the release binary a debug-only build will not produce. The
/// three together left the cache naming a file that was gone, and because
/// `if(NOT SCE_CODEGEN)` was false nothing re-resolved — every generator
/// call then failed with an EMPTY message, surfacing as
/// `sce-codegen list-fixtures --harness simple failed for <path>:` and
/// taking the C++ W3C lane red on 2026-08-11.
///
/// Driven through real `cmake` runs rather than by reading the module: the
/// property is what the cache does on the second configure, which no amount
/// of reading the first one shows. A tiny project including only the
/// locator keeps it to about a second.
#[test]
fn the_cmake_locator_drops_a_cached_path_that_no_longer_exists() {
    if std::process::Command::new("cmake")
        .arg("--version")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("SKIP: cmake not on PATH");
        return;
    }
    let root = repo_root();
    let sandbox = std::env::temp_dir().join(format!("sce-codegen-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&sandbox);
    std::fs::create_dir_all(&sandbox).expect("create sandbox");

    // `project(... NONE)`: no compiler probe, so this stays fast and does
    // not depend on a toolchain being installed.
    std::fs::write(
        sandbox.join("CMakeLists.txt"),
        format!(
            "cmake_minimum_required(VERSION 3.16)\n\
             project(sce_codegen_cache_probe NONE)\n\
             include({}/cmake/SCEFindCodegen.cmake)\n\
             message(STATUS \"RESOLVED=${{SCE_CODEGEN}}\")\n",
            root.display()
        ),
    )
    .expect("write probe CMakeLists");

    let stub = sandbox.join("stub-codegen");
    std::fs::write(&stub, "#!/bin/sh\nexit 0\n").expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    let build = sandbox.join("build");
    let configure = |extra: Option<&str>| -> String {
        let mut cmd = std::process::Command::new("cmake");
        cmd.arg("-S").arg(&sandbox).arg("-B").arg(&build);
        if let Some(arg) = extra {
            cmd.arg(arg);
        }
        let out = cmd.output().expect("cmake runs");
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    };

    let first = configure(Some(&format!("-DSCE_CODEGEN={}", stub.display())));
    assert!(
        first.contains(&format!("RESOLVED={}", stub.display())),
        "the probe did not take the pinned path, so the second configure \
         would prove nothing:\n{first}"
    );

    std::fs::remove_file(&stub).expect("remove the binary the cache names");
    let second = configure(None);
    let _ = std::fs::remove_dir_all(&sandbox);

    assert!(
        !second.contains(&format!("RESOLVED={}", stub.display())),
        "the cache kept a generator path that no longer exists; every call \
         through it fails with an empty error:\n{second}"
    );
}

/// The release fallback must actually resolve, in both directions.
///
/// A search order is the kind of thing that reads correct and is
/// inert: the locators are the only reason a tree holding an older
/// release build still works, and nothing else in the suite would
/// notice if the second candidate were never consulted. This drives
/// the shell locator — the one the regen scripts and the Go harnesses
/// share — against a sandbox holding one profile at a time.
#[test]
fn the_shell_locator_finds_the_generator_in_either_profile() {
    let root = repo_root();
    let sandbox = std::env::temp_dir().join(format!("sce-codegen-locator-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&sandbox);

    let resolve = |profiles: &[&str]| -> String {
        let _ = std::fs::remove_dir_all(&sandbox);
        for profile in profiles {
            let dir = sandbox.join("target").join(profile);
            std::fs::create_dir_all(&dir).expect("create sandbox profile dir");
            let binary = dir.join("sce-codegen");
            std::fs::write(&binary, "#!/bin/sh\nexit 0\n").expect("write stub");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
                    .expect("chmod stub");
            }
        }
        let script = format!(
            "set -euo pipefail\nsource {}/scripts/lib/sce_codegen.sh\nsce_codegen_path {}\n",
            root.display(),
            sandbox.display(),
        );
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("bash runs the locator");
        assert!(
            out.status.success(),
            "the locator failed for profiles {profiles:?}:\n{}",
            String::from_utf8_lossy(&out.stderr),
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    let debug_only = resolve(&["debug"]);
    let release_only = resolve(&["release"]);
    let both = resolve(&["release", "debug"]);
    let _ = std::fs::remove_dir_all(&sandbox);

    assert!(
        debug_only.ends_with("target/debug/sce-codegen"),
        "a tree holding only a debug build must resolve to it; got {debug_only:?}",
    );
    assert!(
        release_only.ends_with("target/release/sce-codegen"),
        "a tree holding only a release build must still work — that is the \
         entire reason the second candidate exists; got {release_only:?}",
    );
    assert!(
        both.ends_with("target/debug/sce-codegen"),
        "with both profiles present the freshly built profile must win, or a \
         stale release binary silently outranks it; got {both:?}",
    );
}

/// A binary that exists is not the same as a binary built from these
/// sources, and the shell locator must ask.
///
/// `sce_codegen_require` handed back whatever `target/` held. That is
/// the locator the 46 regen scripts go through, so
/// `regen_all_committed_trees.sh` refreshed the W3C trees with a
/// generator predating the edit being regenerated for — and then
/// rebuilt mid-script, for a later phase that happened to go through
/// cargo, leaving one working tree holding artifacts from two different
/// generators. The other three locators already avoid this: CMake asks
/// `verify-generator`, Gradle and the Python harness rebuild whenever
/// cargo is present.
///
/// Driven against a sandbox rather than read: the property is which of
/// two commands the function runs, and a stale binary is exactly what
/// the real tree does not have when the suite runs.
#[test]
fn the_shell_locator_rebuilds_a_generator_that_disagrees_with_the_tree() {
    let root = repo_root();
    let sandbox = std::env::temp_dir().join(format!("sce-codegen-witness-{}", std::process::id()));

    // `verify_exit` is what the planted binary answers `verify-generator`
    // with: 20 is the CLI's "stale or unwitnessed", 0 is "current".
    let run = |verify_exit: i32| -> (String, bool) {
        let _ = std::fs::remove_dir_all(&sandbox);
        let bin_dir = sandbox.join("target/debug");
        let fake_path = sandbox.join("fakebin");
        std::fs::create_dir_all(&bin_dir).expect("create sandbox profile dir");
        std::fs::create_dir_all(&fake_path).expect("create fake PATH dir");

        let write_exec = |path: &std::path::Path, body: &str| {
            std::fs::write(path, body).expect("write stub");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
                    .expect("chmod stub");
            }
        };

        write_exec(
            &bin_dir.join("sce-codegen"),
            &format!(
                "#!/bin/sh\ncase \"$1\" in verify-generator) exit {verify_exit};; esac\nexit 0\n"
            ),
        );
        // Stands in for the rebuild: records that it ran, and leaves a
        // binary that agrees with the tree.
        write_exec(
            &fake_path.join("cargo"),
            &format!(
                "#!/bin/sh\ntouch {sandbox}/rebuilt\n\
                 printf '#!/bin/sh\\nexit 0\\n' > {sandbox}/target/debug/sce-codegen\n\
                 chmod 755 {sandbox}/target/debug/sce-codegen\nexit 0\n",
                sandbox = sandbox.display()
            ),
        );

        let script = format!(
            "set -euo pipefail\nexport PATH={fake}:$PATH\nsource {root}/scripts/lib/sce_codegen.sh\n\
             sce_codegen_require {sandbox}\n",
            fake = fake_path.display(),
            root = root.display(),
            sandbox = sandbox.display(),
        );
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("bash runs the locator");
        assert!(
            out.status.success(),
            "the locator failed with verify exit {verify_exit}:\n{}",
            String::from_utf8_lossy(&out.stderr),
        );
        (
            String::from_utf8_lossy(&out.stdout).trim().to_string(),
            sandbox.join("rebuilt").exists(),
        )
    };

    let (stale_path, stale_rebuilt) = run(20);
    let (current_path, current_rebuilt) = run(0);
    let _ = std::fs::remove_dir_all(&sandbox);

    assert!(
        stale_rebuilt,
        "a generator that disagrees with the tree was handed straight back — \
         every regen script would then re-stamp committed trees with it",
    );
    assert!(
        stale_path.ends_with("target/debug/sce-codegen"),
        "after the rebuild the locator must still name the binary; got {stale_path:?}",
    );
    assert!(
        !current_rebuilt,
        "a generator that already agrees with the tree was rebuilt anyway — the \
         witness exists so a prebuilt binary works where cargo is absent",
    );
    assert!(
        current_path.ends_with("target/debug/sce-codegen"),
        "a current generator must be handed back as found; got {current_path:?}",
    );
}

/// Enough regen scripts must reach the scan for a clean result to mean
/// anything. 115 per-stem scripts existed when this bound was measured
/// (2026-08-21); the floor sits well under that so adding or retiring a
/// fixture does not move it, and a glob that matched nothing still fails.
const MIN_REGEN_SCRIPTS: usize = 80;

/// Every regen script reaches the generator through the shell locator.
///
/// The locator is not only where the binary is found — it is also where
/// `SOURCE_DATE_EPOCH` is defaulted to 0, which is what makes the
/// `generated-at` header of a regenerated file reproducible.
/// `committed_trees_carry_a_pinned_generated_at` rejects any other stamp, so
/// a regen script that reaches the generator some other way writes a
/// wall-clock header into a committed tree and the drift gate rejects the
/// push. That is not hypothetical: only the master script exported the
/// variable and all 115 per-stem scripts did not, so regenerating a single
/// fixture — the normal shape of a round that changes one fixture — cost a
/// push cycle on 2026-08-20.
///
/// Stated over the scripts rather than over the exported variable, because
/// the variable is set in one place and a test that read it back there would
/// be asking the fix whether it applied itself. What can actually drift is a
/// script that stops going through the locator.
#[test]
fn every_regen_script_sources_the_shell_locator() {
    let root = repo_root();
    let mut scanned = 0usize;
    let mut orphans: Vec<String> = Vec::new();

    for rel in tracked_files(&root) {
        let Some(name) = rel.strip_prefix("scripts/") else {
            continue;
        };
        if !name.starts_with("regen_") || !name.ends_with(".sh") {
            continue;
        }
        scanned += 1;
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        if !code_lines(&text).any(|(_, l)| l.contains("lib/sce_codegen.sh")) {
            orphans.push(rel);
        }
    }

    assert!(
        scanned >= MIN_REGEN_SCRIPTS,
        "only {scanned} regen scripts reached the scan, under the floor of \
         {MIN_REGEN_SCRIPTS} — the naming convention this walks moved",
    );
    assert!(
        orphans.is_empty(),
        "{} regen script(s) do not source scripts/lib/sce_codegen.sh, so they \
         inherit neither the binary search nor the SOURCE_DATE_EPOCH pin and \
         will stamp wall-clock `generated-at` headers into committed trees: \
         {orphans:#?}",
        orphans.len(),
    );
}
