// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Claude Code programs this computer has, as the application finds them.
//!
//! A person does not type where a program is: a command that took a path would be a way to make
//! the application run whatever file it names, for anyone who can send the command. The
//! application looks in the places a client is installed (the search path first, then the
//! folders the official installer uses, which a window started from a menu may not have on its
//! path), asks each program that answers to `claude` what it is, and offers the ones that say
//! they are Claude Code. What it offers is what it may be told to run, and nothing else.
//!
//! Unix only: the stand-ins are shell scripts.

#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sce_app_core::claude_code::{candidates, known_directories, Candidate, Found, Search};

/// A program named `claude` in a folder of its own, saying `says` to `--version`.
fn claude_in(label: &str, says: &str) -> PathBuf {
    let dir = common::scratch(label);
    let program = dir.join("claude");
    common::write_program(
        &program,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '{says}'; exit 0; fi\nexit 1\n"
        ),
    );
    program
}

fn dir_of(program: &Path) -> PathBuf {
    program.parent().unwrap().to_path_buf()
}

fn search(path: Vec<PathBuf>, known: Vec<PathBuf>) -> Search {
    Search {
        path,
        known,
        timeout: Duration::from_secs(5),
    }
}

#[test]
fn a_program_on_the_search_path_that_says_it_is_claude_code_is_offered() {
    let program = claude_in("clients-path", "2.1.291 (Claude Code)");

    let found = candidates(&search(vec![dir_of(&program)], vec![]));

    assert_eq!(
        found,
        vec![Candidate {
            path: program,
            version: "2.1.291".to_string(),
            found: Found::SearchPath,
        }]
    );
}

#[test]
fn a_program_called_claude_that_says_it_is_something_else_is_not_offered() {
    let impostor = claude_in("clients-impostor", "1.0 (a program that is not it)");
    let says_nothing = claude_in("clients-silent", "");

    let found = candidates(&search(
        vec![dir_of(&impostor), dir_of(&says_nothing)],
        vec![],
    ));

    assert_eq!(found, vec![]);
}

#[test]
fn a_file_that_cannot_be_run_is_not_offered_and_does_not_stop_the_search() {
    let dir = common::scratch("clients-not-runnable");
    fs::write(dir.join("claude"), "#!/bin/sh\necho '2.0 (Claude Code)'\n").unwrap();
    fs::set_permissions(dir.join("claude"), fs::Permissions::from_mode(0o644)).unwrap();
    let real = claude_in("clients-after", "2.1.291 (Claude Code)");

    let found = candidates(&search(vec![dir, dir_of(&real)], vec![]));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, real);
}

#[test]
fn a_folder_that_is_not_there_is_not_an_error() {
    let real = claude_in("clients-missing-dir", "2.1.291 (Claude Code)");

    let found = candidates(&search(
        vec![PathBuf::from("/nowhere/at/all"), dir_of(&real)],
        vec![],
    ));

    assert_eq!(found.len(), 1);
}

#[test]
fn the_same_program_reached_by_two_names_is_offered_once_by_the_first() {
    let real = claude_in("clients-same", "2.1.291 (Claude Code)");
    let other = common::scratch("clients-link");
    symlink(&real, other.join("claude")).unwrap();

    let found = candidates(&search(vec![dir_of(&real), other], vec![]));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, real);
}

#[test]
fn the_search_path_comes_first_and_a_known_location_is_said_to_be_one() {
    let on_path = claude_in("clients-order-path", "2.1.291 (Claude Code)");
    let installed = claude_in("clients-order-known", "2.1.280 (Claude Code)");

    let found = candidates(&search(vec![dir_of(&on_path)], vec![dir_of(&installed)]));

    assert_eq!(
        found
            .iter()
            .map(|c| (&c.version[..], c.found))
            .collect::<Vec<_>>(),
        vec![
            ("2.1.291", Found::SearchPath),
            ("2.1.280", Found::KnownLocation)
        ]
    );
}

#[test]
fn a_program_that_does_not_answer_does_not_hold_the_search() {
    let dir = common::scratch("clients-hangs");
    let hung = dir.join("claude");
    common::write_program(
        &hung,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exec sleep 30; fi\nexit 1\n",
    );
    let real = claude_in("clients-after-hang", "2.1.291 (Claude Code)");
    let mut asked = search(vec![dir, dir_of(&real)], vec![]);
    asked.timeout = Duration::from_millis(400);

    let started = Instant::now();
    let found = candidates(&asked);

    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, real);
}

#[test]
fn the_known_locations_are_where_the_official_installer_puts_it_under_the_home_folder() {
    let known = known_directories(Some(Path::new("/home/person")));

    assert!(
        known.contains(&PathBuf::from("/home/person/.local/bin")),
        "{known:?}"
    );
    assert!(
        known.contains(&PathBuf::from("/home/person/.claude/local")),
        "{known:?}"
    );
    // With no home folder there are the places that need none, and no guess at one.
    let without = known_directories(None);
    assert!(
        without.iter().all(|dir| !dir.starts_with("/home")),
        "{without:?}"
    );
}
