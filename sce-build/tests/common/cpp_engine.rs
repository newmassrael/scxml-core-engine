// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Where a translation unit finds the headers the generated C++ engine
//! includes.
//!
//! A CMake consumer never names these: it links `SCE::sce_core`, whose
//! interface carries them (`sce/CMakeLists.txt`: its own include root, the
//! JSON and XML readers, and `sce_forge_runtime` linked INTERFACE). The suites
//! that compile generated C++ with `g++` directly have no target to link, so
//! they ask here, and this list is that interface written once.
//!
//! Each suite used to write the list itself. When the engine's own header
//! started including the forge runtime (`core/EngineInbox.h` holds the
//! inbox on the forge LSCQ, docs/adr/0083), six of them stopped compiling
//! every generated machine at once, while the CMake build — which reads the
//! interface — was green. A dependency of the engine is added here and in
//! `sce_core`'s interface, nowhere else.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// The include directories of `SCE::sce_core`'s interface, in the build tree:
/// what a unit including a generated `<machine>_sm.h` compiles against.
pub fn engine_include_dirs() -> Vec<PathBuf> {
    let root = repo_root();
    vec![
        root.join("sce/include"),
        root.join("third_party/nlohmann_json/include"),
        root.join("third_party/pugixml/src"),
        root.join("backends/cpp/forge-runtime/include"),
    ]
}
