// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The programs an installed application carries, found where an installer puts them.
//!
//! An installer cannot rely on anything being installed beside the application: not the
//! generator, not `sce-work`, not the authoring server. It carries them in a folder of its own
//! (`sce-author/bin`, the layout `scripts/package_sce_author.sh` makes), and the application has
//! to find them there without the environment naming them and without a source tree. What is
//! found is what the shell uses; what is missing is said, program by program, so that a broken
//! install is told as one and not as an AI that will not start.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use sce_app_core::installed::{in_bundle, Installed};

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

#[test]
fn a_bundle_that_carries_everything_is_found_program_by_program() {
    let root = common::scratch("installed-all");
    for name in ["sce-author-mcp", "sce-work", "sce-codegen"] {
        touch(&root.join("bin").join(exe(name)));
    }

    let found = in_bundle(&root);

    assert_eq!(
        found,
        Installed {
            author: Some(root.join("bin").join(exe("sce-author-mcp"))),
            work: Some(root.join("bin").join(exe("sce-work"))),
            codegen: Some(root.join("bin").join(exe("sce-codegen"))),
        }
    );
    assert!(found.is_complete());
    assert!(found.missing().is_empty());
}

#[test]
fn what_the_bundle_lacks_is_named() {
    let root = common::scratch("installed-partial");
    touch(&root.join("bin").join(exe("sce-codegen")));

    let found = in_bundle(&root);

    assert_eq!(
        found.codegen,
        Some(root.join("bin").join(exe("sce-codegen")))
    );
    assert_eq!(found.author, None);
    assert_eq!(found.work, None);
    assert!(!found.is_complete());
    assert_eq!(found.missing(), vec!["sce-author-mcp", "sce-work"]);
}

#[test]
fn a_folder_with_no_bundle_in_it_has_nothing() {
    let root = common::scratch("installed-none");

    let found = in_bundle(&root);

    assert_eq!(found, Installed::default());
    assert_eq!(
        found.missing(),
        vec!["sce-author-mcp", "sce-work", "sce-codegen"]
    );
    assert_eq!(
        in_bundle(&PathBuf::from("/nowhere/at/all")),
        Installed::default()
    );
}

#[test]
fn a_directory_of_the_same_name_is_not_a_program() {
    let root = common::scratch("installed-dir");
    fs::create_dir_all(root.join("bin").join(exe("sce-work"))).unwrap();

    assert_eq!(in_bundle(&root).work, None);
}
