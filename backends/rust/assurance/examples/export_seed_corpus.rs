// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Write what every target reads when it is true, one file per item under `<dir>/<target>/`, so that
//! the coverage-guided fuzzer starts from inputs that reach the checks behind the parsers instead of
//! from nothing. `scripts/fuzz/run.sh` calls it; run by hand: `cargo run -p sce-rust-assurance
//! --example export_seed_corpus -- <dir>`.

use std::path::PathBuf;

use sce_rust_assurance::targets::{self, NAMES};

fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: export_seed_corpus <dir>"),
    );
    for name in NAMES {
        let target = targets::build(name);
        let folder = dir.join(name);
        std::fs::create_dir_all(&folder).unwrap();
        for (index, item) in target.corpus.iter().enumerate() {
            std::fs::write(folder.join(format!("seed-{index}")), item).unwrap();
        }
        println!("{name}: {} seeds", target.corpus.len());
    }
}
