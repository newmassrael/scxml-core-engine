// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-ttcn3-validate <suite.scxml> <export dir>`: validate the TTCN-3
//! export the directory holds of the suite (docs/SCE_TEST_PROFILE.md §9.4).
//!
//! Each violation is printed on a line of its own, its code first. The exit
//! status is 0 when there is none, 1 when there is one or more, and 2 when an
//! input cannot be read or is not one the validator reads.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [suite, export] = arguments.as_slice() else {
        eprintln!("usage: sce-ttcn3-validate <suite.scxml> <export dir>");
        return ExitCode::from(2);
    };
    match sce_ttcn3_validator::validate_directory(Path::new(suite), Path::new(export)) {
        Ok(violations) if violations.is_empty() => ExitCode::SUCCESS,
        Ok(violations) => {
            for violation in &violations {
                println!("{violation}");
            }
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("sce-ttcn3-validate: {error}");
            ExitCode::from(2)
        }
    }
}
