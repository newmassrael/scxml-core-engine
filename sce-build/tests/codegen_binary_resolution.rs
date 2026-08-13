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
