// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Generate code from any SCXML file:
//   cargo run --example gen_any -- path/to/file.scxml [rust|cpp|kotlin]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: gen_any <scxml_file> [rust|cpp|kotlin]");
        std::process::exit(2);
    }

    let lang_str = if args.len() == 3 { &args[2] } else { "rust" };
    let language: sce_build::generator::Language = lang_str.parse().unwrap_or_else(|_| {
        eprintln!("Unknown language: {lang_str}. Use: rust, cpp, kotlin");
        std::process::exit(2);
    });

    let template_dir = sce_build::find_template_dir_for(language);
    match sce_build::compile_scxml_lang(&args[1], &template_dir, language) {
        Ok(output) => {
            for (_filename, code) in &output.files {
                print!("{code}");
            }
        }
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    }
}
