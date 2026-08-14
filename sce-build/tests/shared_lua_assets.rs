// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! The Lua sources every engine loads have exactly one definition.
//!
//! `sce/include/scripting/*.lua` is that definition. Five of the six
//! backends reach it directly — C++ through a CMake-generated header, Rust
//! through `include_str!`, Kotlin through a Gradle copy, Python through the
//! repository path, C11 through a codegen-time embed — and Go cannot,
//! because `go:embed` refuses paths outside the module. Go therefore keeps
//! a byte copy, and a byte copy with nothing checking it is a second
//! definition waiting to disagree.
//!
//! That matters more for `ecma_semantics.lua` than for anything else here:
//! it defines what `==` and `+` *mean*, so a drifted copy would not crash,
//! it would answer differently on one backend.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every `(canonical source, copy)` pair a backend maintains.
const COPIES: &[(&str, &str)] = &[
    (
        "sce/include/scripting/json_builtins.lua",
        "backends/go/lua/json_builtins.lua",
    ),
    (
        "sce/include/scripting/ecma_semantics.lua",
        "backends/go/lua/ecma_semantics.lua",
    ),
];

#[test]
fn every_copied_lua_asset_is_byte_identical_to_its_source() {
    for (source, copy) in COPIES {
        let source_path = repo_root().join(source);
        let copy_path = repo_root().join(copy);
        let source_bytes = std::fs::read(&source_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", source_path.display()));
        let copy_bytes = std::fs::read(&copy_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", copy_path.display()));
        assert_eq!(
            source_bytes, copy_bytes,
            "{copy} has drifted from {source}. The copy exists because `go:embed` \
             cannot reach outside the module, not because the two are allowed to \
             differ — copy the source over it rather than editing the copy."
        );
    }
}

/// Every engine that runs generated Lua loads the semantics file.
///
/// The emitted code calls `_scxml_add`, `_scxml_eq` and the bit helpers by
/// name. An engine that skips the file does not fail to build — it fails at
/// the first `+` in a document, at runtime, in whatever language embeds it.
/// So the load is asserted from the outside, once, rather than trusted to
/// six separate reviews.
#[test]
fn every_lua_engine_loads_the_shared_semantics() {
    // Each entry: the file that must reference the asset, and the token that
    // shows it is loaded rather than merely mentioned.
    const LOADERS: &[(&str, &str)] = &[
        ("backends/rust/lua/src/lib.rs", "ecma_semantics.lua"),
        ("backends/go/lua/lua_engine.go", "ecmaSemanticsLua"),
        (
            "backends/kotlin/lua/src/main/kotlin/com/sce/scripting/lua/LuaScriptEngine.kt",
            "loadEcmaSemantics()",
        ),
        (
            "backends/python/runtime/sce_runtime/scripting/lua_engine.py",
            "ecma_semantics.lua",
        ),
        ("sce/src/scripting/LuaEngine.cpp", "ECMA_SEMANTICS_LUA"),
        (
            "tools/codegen/templates/c/scriptengine.jinja2",
            "ecma_semantics_lua_c",
        ),
    ];
    let mut missing = Vec::new();
    for (file, token) in LOADERS {
        let path = repo_root().join(file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if !text.contains(token) {
            missing.push(format!(
                "{file} does not load the shared semantics ({token})"
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "an engine runs generated Lua without the definitions that Lua calls:\n{}",
        missing.join("\n")
    );
}
