// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A fixed test port must not be one the kernel hands out.
//
// `tests/CMakeLists.txt` pins a loopback port for each mesh fixture
// that needs a stable listen address. Six of them were picked from the
// IANA *dynamic* range (49152-65535) on the reasoning that nothing is
// registered there — true, and the wrong property to select on. Linux
// allocates ephemeral ports from `ip_local_port_range`, 32768-60999 by
// default, so a fixed port inside it is a port the kernel may hand to
// any process that binds port 0, including the sibling test running
// beside this one.
//
// Measured 2026-08-12: `mesh_session_f_crossdev_donedata` failed a
// full-suite run in `TransportRouter::init` while its own workers held
// 33837, 36605 and 45821 — all freshly allocated from that range. The
// test carried a `RESOURCE_LOCK` naming its port, which reads as
// protection and is not: a lock serialises tests that declare the same
// lock, and an ephemeral bind declares nothing. All six fixed ports
// were in the range, so the other five were unlucky-not-yet.
//
// This is the check that keeps the next "free-looking" number out of
// the range, since picking one is a decision no build error follows.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// The range the running kernel allocates ephemeral ports from.
///
/// Read from the kernel rather than assumed, because the property
/// under test is about *this* machine's allocator — a CI runner that
/// widened the range would make a port that is safe here unsafe there,
/// and a hardcoded pair would not notice. The documented Linux default
/// is the fallback when the file cannot be read, so the check still
/// asserts something rather than passing quietly.
fn ephemeral_range() -> (u32, u32, &'static str) {
    match std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range") {
        Ok(text) => {
            let mut it = text
                .split_whitespace()
                .filter_map(|t| t.parse::<u32>().ok());
            match (it.next(), it.next()) {
                (Some(lo), Some(hi)) if lo < hi => (lo, hi, "this kernel"),
                _ => (32768, 60999, "the Linux default (kernel file unparseable)"),
            }
        }
        Err(_) => (32768, 60999, "the Linux default (kernel file unreadable)"),
    }
}

/// Every `set(SCE_TEST_..._PORT "<n>" CACHE ...)` declaration, read
/// from the file that declares them so a port added tomorrow is
/// checked without a second edit here.
fn declared_ports(cmake: &str) -> Vec<(String, u32)> {
    let mut found = Vec::new();
    for line in cmake.lines() {
        // Anchored at the start of the trimmed line, which is also what
        // keeps prose out: this file's rationale quotes the numbers that
        // used to be wrong, and a search that looked anywhere in the line
        // would read the explanation as a declaration. An explicit
        // comment skip was written here first and was dead — a commented
        // `# set(...)` fails this prefix already — so it is gone rather
        // than kept as a second mechanism nobody exercises.
        let Some(rest) = line.trim_start().strip_prefix("set(SCE_TEST_") else {
            continue;
        };
        let Some((name, tail)) = rest.split_once(char::is_whitespace) else {
            continue;
        };
        if !name.contains("PORT") {
            continue;
        }
        let value: String = tail
            .trim_start()
            .trim_start_matches('"')
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(port) = value.parse::<u32>() {
            found.push((format!("SCE_TEST_{name}"), port));
        }
    }
    found
}

#[test]
fn no_fixed_test_port_sits_in_the_kernels_ephemeral_range() {
    let cmake = std::fs::read_to_string(repo_root().join("tests/CMakeLists.txt"))
        .expect("read tests/CMakeLists.txt");
    let ports = declared_ports(&cmake);

    // Floor: a parse that stopped matching would find nothing and
    // report every port safe. Six were declared when this was written.
    assert!(
        ports.len() >= 6,
        "parsed only {} fixed port declaration(s) from tests/CMakeLists.txt — \
         the scan is broken, not the file",
        ports.len()
    );

    let (lo, hi, source) = ephemeral_range();
    let colliding: Vec<String> = ports
        .iter()
        .filter(|(_, p)| (lo..=hi).contains(p))
        .map(|(name, p)| format!("{name}={p}"))
        .collect();
    assert!(
        colliding.is_empty(),
        "fixed test port(s) inside the ephemeral range {lo}-{hi} reported by {source}: {colliding:?}\n\
         The kernel may hand any of these to a sibling test's `bind(port 0)` at any moment, and a \
         RESOURCE_LOCK does not prevent it — the competitor is an ephemeral bind that declares no \
         lock. Pick a port below {lo}."
    );
}

#[test]
fn no_configured_build_tree_still_holds_a_port_from_the_range() {
    // The declaration is a `CACHE STRING` default, which an already
    // configured tree ignores. Measured 2026-08-12: after the six ports
    // moved below the floor, `build/CMakeCache.txt` still held all six
    // old values, so the build that actually runs the tests kept the
    // colliding ones — and the check above, reading only the source,
    // would have called that clean. CI restores this cache between
    // runs, so the stale value is not a local-only accident.
    //
    // Absent tree: nothing to check. This is a property of a tree that
    // exists, and `the_build_dir_resolver_survives_a_tree_that_does_not
    // _exist_yet` is the sibling case for gates that must start anyway.
    let cache = repo_root().join("build/CMakeCache.txt");
    let Ok(text) = std::fs::read_to_string(&cache) else {
        return;
    };

    let (lo, hi, source) = ephemeral_range();
    let mut colliding: Vec<String> = Vec::new();
    let mut seen = 0usize;
    for line in text.lines() {
        // `NAME:STRING=value`, the cache's own shape.
        let Some((decl, value)) = line.split_once('=') else {
            continue;
        };
        let Some((name, _ty)) = decl.split_once(':') else {
            continue;
        };
        if !name.starts_with("SCE_TEST_") || !name.contains("PORT") {
            continue;
        }
        seen += 1;
        if let Ok(port) = value.trim().parse::<u32>() {
            if (lo..=hi).contains(&port) {
                colliding.push(format!("{name}={port}"));
            }
        }
    }

    assert!(
        colliding.is_empty(),
        "the configured build tree holds fixed port(s) inside the ephemeral range {lo}-{hi} \
         reported by {source}: {colliding:?}\n\
         Editing the `set(... CACHE STRING)` default does not move an existing tree. Re-point them \
         with:\n  cmake -S . -B build {}",
        colliding
            .iter()
            .filter_map(|c| c.split('=').next())
            .map(|n| format!("-D{n}=<port below {lo}>"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    // Not a floor on `seen`: a tree configured before these ports
    // existed legitimately has none, and demanding some would fail a
    // fresh checkout for being fresh.
    let _ = seen;
}

#[test]
fn the_port_scan_reads_declarations_rather_than_prose() {
    // The anchor is the mechanism. `tests/CMakeLists.txt` explains the
    // move in prose that quotes the old numbers, so a scan that looked
    // for the pattern anywhere in a line would report its own rationale
    // as a violation — and the fix for that red would be to delete the
    // explanation. Both shapes that carry an old number are here.
    let sample = "\
        # These were 55821 before the move; see the note above.\n\
        # set(SCE_TEST_HISTORICAL_PORT \"55821\" CACHE STRING \"what it used to be\")\n\
        set(SCE_TEST_REAL_PORT \"20821\" CACHE STRING \"doc\")\n";
    let found = declared_ports(sample);
    assert_eq!(
        found,
        vec![("SCE_TEST_REAL_PORT".to_string(), 20821)],
        "the scan read a commented-out declaration as a live one"
    );
}
