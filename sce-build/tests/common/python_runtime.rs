// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Whether a Python interpreter can run the SCE Python runtime.
//!
//! The runtime imports `lupa.lua54` (`sce_runtime/scripting/lua_engine.py`),
//! which lupa ships from 2.0 on. A distribution's `python3-lupa` is older:
//! Ubuntu's 1.14 answers `import lupa` and has no `lua54`, so a check that
//! asks only for `import lupa` passes and the machine then fails to import.
//! That is a red that blames the code for a property of the machine, the
//! defect `scripts/install_sce_python.sh` and the gates' precondition
//! (`sce_gate_python_requires_lupa` in scripts/gates/lib.sh) were written
//! against. The question asked here is the runtime's own import.

// Every suite that declares `mod common` compiles this module; only the ones
// that run the Python runtime call it.
#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

/// Whether `python` imports the Lua binding the SCE Python runtime imports.
pub fn runs_the_runtime(python: &Path) -> bool {
    Command::new(python)
        .args(["-I", "-c", "import lupa.lua54"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
