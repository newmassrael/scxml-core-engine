// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-codegen codec-roles` prints the table `codec_roles` computes
//! (docs/adr/0058 §1): the same text for the same document, one table per
//! document, and a refusal of a document that is not a codec.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn resource(stem: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/forge/resources")
        .join(format!("{stem}.scxml"))
}

fn codec_roles(documents: &[PathBuf]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("codec-roles")
        .args(documents)
        .output()
        .expect("spawn sce-codegen")
}

#[test]
fn it_prints_the_table_of_each_document_in_the_order_given() {
    let run = codec_roles(&[resource("codec_length_ref"), resource("codec_repeat_basic")]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let table = String::from_utf8(run.stdout).expect("text");
    assert_eq!(
        table
            .lines()
            .filter(|line| line.starts_with("codec "))
            .collect::<Vec<_>>(),
        ["codec codec_length_ref", "codec codec_repeat_basic"]
    );
    assert!(
        table.contains("len      derived  length-of(payload)"),
        "{table}"
    );
    assert!(table.contains("count-of(frags)"), "{table}");
}

#[test]
fn a_document_that_is_not_a_codec_is_refused() {
    let run = codec_roles(&[resource("inline_codec")]);
    assert!(!run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("is not one"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
