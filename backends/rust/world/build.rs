// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the world simulation: the machines under `machines/` are
// generated in-process by sce_build into OUT_DIR, and a tests file mounts them
// through `machines.rs`, which this script writes. Nothing generated is
// committed, so there is no tree to regenerate or to drift.
//
// A generated machine opens with inner attributes and doc comments, which
// `include!` cannot take, so each is declared as a module by `#[path]` instead.

use std::path::Path;

fn main() {
    let machines = ["echo_client", "echo_server"];
    println!("cargo:rerun-if-changed=build.rs");
    let sources: Vec<String> = machines
        .iter()
        .map(|name| format!("machines/{name}.scxml"))
        .collect();
    for source in &sources {
        println!("cargo:rerun-if-changed={source}");
    }
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    sce_build::compile_scxml(&sources);

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let mut mounted = String::new();
    for name in machines {
        let generated = Path::new(&out_dir).join(format!("{name}_sm.rs"));
        mounted.push_str(&format!(
            "#[path = {:?}]\n#[allow(dead_code, unused)]\npub mod {name};\n",
            generated.display().to_string()
        ));
    }
    std::fs::write(Path::new(&out_dir).join("machines.rs"), mounted).expect("write machines.rs");
}
