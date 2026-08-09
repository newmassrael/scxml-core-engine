// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML 4.9 `<log>` on the C11 backend, without a script engine.
//
// `<log>` lowers to `fprintf(stderr, …)`. The `<stdio.h>` include was
// gated on `model.needs_script_engine`, so a document that logs but
// needs no script engine emitted a call with no declaration in scope.
// C99 removed implicit declarations, and the repo builds C11 with
// `-Werror`, so the generated translation unit did not compile —
// while `sce-codegen` reported success and every generator test
// stayed green, because nothing compiled that combination.
//
// The combination is narrow enough that no existing fixture hit it:
// the C11 fixtures that log also use a script engine, and the ones
// that do not log compile fine. This test pins the intersection.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A document that logs and needs no script engine: `<log label=…>`
/// is a literal label, not an expression, so nothing here forces the
/// scripting tier on.
const LOGGING_FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <onentry>
      <log label="entering s1"/>
    </onentry>
    <transition event="go" target="s2">
      <log label="taking go"/>
    </transition>
  </state>
  <final id="s2"/>
</scxml>
"#;

#[test]
fn c11_log_without_script_engine_compiles_under_werror() {
    let Some(cc) = sce_build::toolchain::locate_any(&["gcc", "cc"]) else {
        eprintln!("no C compiler located; skipping");
        return;
    };

    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("c11-log-no-engine-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let scxml = out_dir.join("logging.scxml");
    std::fs::write(&scxml, LOGGING_FIXTURE).expect("write fixture");

    let gen = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&scxml)
        .arg("-l")
        .arg("c11")
        .arg("-o")
        .arg(&out_dir)
        .output()
        .expect("invoke sce-codegen");
    assert!(
        gen.status.success(),
        "generation failed: {}",
        String::from_utf8_lossy(&gen.stderr),
    );

    // The manifest must agree that no script engine is involved —
    // otherwise this test would be exercising the already-covered
    // scripting path and would say nothing about the gap.
    let manifest: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&gen.stdout).trim())
            .expect("manifest line is JSON");
    assert_eq!(
        manifest["needs_script_engine"], false,
        "fixture must not need a script engine, or it misses the gap",
    );

    let output = Command::new(&cc)
        .args(["-std=c11", "-c", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(&out_dir)
        .arg("-o")
        .arg(out_dir.join("logging_sm.o"))
        .arg(out_dir.join("logging_sm.c"))
        .output()
        .expect("invoke C compiler");

    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&out_dir);
    assert!(
        output.status.success(),
        "generated C11 with <log> and no script engine must compile \
         under -Werror:\n{stderr}",
    );
}
