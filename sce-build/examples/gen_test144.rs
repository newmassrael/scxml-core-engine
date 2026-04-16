// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Quick test: generate test144_sm.rs and print to stdout
fn main() {
    let template_dir = sce_build::find_template_dir();
    match sce_build::compile_scxml_to_string("resources/144/test144.scxml", &template_dir) {
        Ok(code) => {
            print!("{code}");
        }
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    }
}
