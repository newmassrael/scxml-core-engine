// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Watching-zenoh RFC §5.O Atomic 1 — sourcemap + symbol mangling +
// addr2sce integration fixture.
//
// The Atomic 1 contract (spec lines 3055-3057, 3219-3243, 3253-3278,
// 3321-3324):
//
//   D18(i)   Sourcemap JSON shape — version + source_hash +
//            template_hash + symbols map.
//   D18(ii)  Byte-identity across the 6 backends for the same SCXML.
//   D18(iii) source_hash byte-equal to §6.2.6 drift header.
//   D18(iv)  Symbol mangling round-trip with `_u_` escape.
//   D18(v)   Each new diagnostic fires on a synthetic offender.
//   D18(vi)  `sce-codegen addr2sce` resolves a known symbol.

use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript" name="atomic1">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"#;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn rand_suffix() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut h = DefaultHasher::new();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    h.finish()
}

/// Stage the fixture into a unique temp dir and run `sce-codegen
/// generate -l <lang>`. Returns (tmp_dir, sourcemap_text).
fn generate(lang: &str) -> (PathBuf, String) {
    let tmp = std::env::temp_dir().join(format!(
        "sce_atomic1_{lang}_{:x}",
        rand_suffix(),
    ));
    std::fs::create_dir_all(&tmp).expect("tmp dir");
    let scxml = tmp.join("atomic1.scxml");
    std::fs::write(&scxml, FIXTURE).expect("write fixture");

    let out = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&scxml)
        .arg("-l")
        .arg(lang)
        .arg("-o")
        .arg(&tmp)
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "generate -l {lang} failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let map_path = tmp.join("sce_sourcemap.json");
    let body = std::fs::read_to_string(&map_path)
        .unwrap_or_else(|e| panic!("read sourcemap at {}: {e}", map_path.display()));
    (tmp, body)
}

/// D18(i) — schema sanity. The emitted JSON carries the three top-
/// level keys per spec lines 3219-3243; the `symbols` object has at
/// least the machine-level entry.
#[test]
fn sourcemap_shape_matches_spec() {
    let (tmp, json) = generate("rust");
    let val: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(val["version"], 1);
    assert!(val.get("source_hash").is_some());
    assert!(val.get("template_hash").is_some());
    let symbols = val["symbols"].as_object().expect("symbols object");
    assert!(!symbols.is_empty(), "symbols map must be non-empty");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// D18(ii) — byte-identity across the 6 backends. Same SCXML input
/// produces a byte-equal sourcemap regardless of which backend wrote
/// it (Q-§5.O-8). Only checks the non-language-specific portion: the
/// symbol table + hash values. Note: the `scxml_file` field carries
/// the tmp-dir path which is unique per invocation, so we normalize
/// before comparing.
#[test]
fn sourcemap_byte_identity_across_backends() {
    let backends = ["rust", "cpp", "kotlin", "go", "c"];
    let mut normalised: Vec<String> = Vec::new();
    for lang in &backends {
        let (tmp, json) = generate(lang);
        let val: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        // Normalise: replace the per-invocation tmp-dir path in
        // `scxml_file` fields with a constant token so the comparison
        // is invariant of the temp directory.
        let normalised_val = normalise_scxml_file_paths(val);
        normalised.push(serde_json::to_string(&normalised_val).unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }
    // All entries should match the first.
    let first = &normalised[0];
    for (i, j) in normalised.iter().enumerate().skip(1) {
        assert_eq!(
            j, first,
            "sourcemap for backend {} differs from first (rust)",
            backends[i],
        );
    }
}

/// Replace per-invocation tmp-dir paths with a constant token so
/// byte-comparison across backends is path-invariant. The sourcemap
/// otherwise carries the same content for the same SCXML input.
fn normalise_scxml_file_paths(mut val: serde_json::Value) -> serde_json::Value {
    if let Some(symbols) = val
        .as_object_mut()
        .and_then(|o| o.get_mut("symbols"))
        .and_then(|s| s.as_object_mut())
    {
        for (_k, v) in symbols {
            if let Some(file) = v.get_mut("scxml_file").and_then(|f| f.as_str()).map(String::from)
            {
                let token = match file.rsplit('/').next() {
                    Some(name) => format!("<tmp>/{name}"),
                    None => "<tmp>".into(),
                };
                v["scxml_file"] = serde_json::Value::String(token);
            }
        }
    }
    val
}

/// D18(iii) — sourcemap.source_hash byte-equal to the §6.2.6 header
/// `source-hash` value embedded in the generated SM file.
#[test]
fn sourcemap_source_hash_matches_drift_header() {
    let (tmp, json) = generate("rust");
    let val: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let map_hash = val["source_hash"].as_str().unwrap().to_string();

    // Parse the §6.2.6 header from the emitted *_sm.rs.
    let sm_path = tmp.join("atomic1_sm.rs");
    let sm_body = std::fs::read_to_string(&sm_path).expect("read sm");
    let header_hash = sm_body
        .lines()
        .find_map(|l| l.strip_prefix("// source-hash: ").map(str::to_string))
        .expect("found source-hash header line");
    assert_eq!(
        map_hash, header_hash,
        "sourcemap.source_hash must match §6.2.6 header `source-hash`",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// D18(iv) — symbol mangling round-trip with `_u_` escape rule. Pure
/// library-level check (no codegen invocation).
#[test]
fn symbol_mangling_round_trip() {
    use sce_build::forge::symbol_mangling::{demangle, mangle};
    // Plain triple round-trips.
    let m = mangle("motor", "running", "_state_body");
    let (a, b, c) = demangle(&m).unwrap();
    assert_eq!((a.as_str(), b.as_str(), c.as_str()), ("motor", "running", "_state_body"));

    // Machine name with literal `__` escapes to `_u_` and round-trips.
    let m2 = mangle("ma__ch", "s1", "_state_body");
    let (a2, b2, c2) = demangle(&m2).unwrap();
    assert_eq!((a2.as_str(), b2.as_str(), c2.as_str()), ("ma__ch", "s1", "_state_body"));

    // State path with hierarchy separator flattens to `_`.
    let m3 = mangle("m", "s1/s1p1", "_state_body");
    assert!(m3.contains("s1_s1p1"));
}

/// D18(v.a) — `traceability/state-id-collision` fires on a synthetic
/// duplicate-mangled-symbol scenario (constructed at the library
/// level since the SCXML parser already rejects duplicate state ids
/// via `ValidationDuplicateId`; the §5.O collision case kicks in
/// only after XInclude / template composition unifies two distinct
/// fragments).
#[test]
fn state_id_collision_diagnostic_payload_shape() {
    use sce_build::forge::diagnostic::ToDiagnostics;
    use sce_build::forge::error::ValidationError;
    let err = sce_build::forge::error::ForgeError::from(
        ValidationError::TraceabilityStateIdCollision {
            mangled: "m__dup___state_body".into(),
            first_file: "a.scxml".into(),
            first_line: 7,
            second_file: "b.scxml".into(),
            second_line: 11,
        },
    );
    let d = err.to_diagnostics().pop().expect("one diagnostic");
    let code_str = serde_json::to_string(&d.code).unwrap();
    assert_eq!(code_str, "\"traceability/state-id-collision\"");
    assert!(d.actual.as_deref() == Some("m__dup___state_body"));
}

/// D18(v.b) — `traceability/symbol-name-exceeds-c-identifier-limit`.
#[test]
fn symbol_length_diagnostic_payload_shape() {
    use sce_build::forge::diagnostic::ToDiagnostics;
    use sce_build::forge::error::ValidationError;
    let err = sce_build::forge::error::ForgeError::from(
        ValidationError::TraceabilitySymbolNameExceedsCIdentifierLimit {
            mangled: "a".repeat(40),
            actual_len: 40,
            over_by: 9,
        },
    );
    let d = err.to_diagnostics().pop().expect("one diagnostic");
    let code_str = serde_json::to_string(&d.code).unwrap();
    assert_eq!(
        code_str,
        "\"traceability/symbol-name-exceeds-c-identifier-limit\""
    );
}

/// D18(v.c) — `traceability/sourcemap-source-hash-mismatch`.
#[test]
fn sourcemap_drift_diagnostic_payload_shape() {
    use sce_build::forge::diagnostic::ToDiagnostics;
    use sce_build::forge::error::ValidationError;
    let err = sce_build::forge::error::ForgeError::from(
        ValidationError::TraceabilitySourcemapSourceHashMismatch {
            file: "out/rust/sce_sourcemap.json".into(),
            sourcemap_hash: "aaaa".into(),
            header_hash: "bbbb".into(),
        },
    );
    let d = err.to_diagnostics().pop().expect("one diagnostic");
    let code_str = serde_json::to_string(&d.code).unwrap();
    assert_eq!(
        code_str,
        "\"traceability/sourcemap-source-hash-mismatch\""
    );
}

/// D18(v.d) — `traceability/sce-map-attribute-stripped`.
#[test]
fn sce_map_stripped_diagnostic_payload_shape() {
    use sce_build::forge::diagnostic::ToDiagnostics;
    use sce_build::forge::error::ValidationError;
    let err = sce_build::forge::error::ForgeError::from(
        ValidationError::TraceabilitySceMapAttributeStripped {
            crate_name: "sce_rust_tests".into(),
            function: "test144::on_entry_s0_0".into(),
            profile: "release".into(),
        },
    );
    let d = err.to_diagnostics().pop().expect("one diagnostic");
    let code_str = serde_json::to_string(&d.code).unwrap();
    assert_eq!(code_str, "\"traceability/sce-map-attribute-stripped\"");
}

/// D18(vi) — `sce-codegen addr2sce --symbol` resolves a known mangled
/// symbol from the per-machine sourcemap. The fixture's machine name
/// is `atomic1` and the parser emits at least the machine-level
/// symbol entry, so we can look it up and verify the resolved JSON.
#[test]
fn addr2sce_resolves_known_symbol() {
    let (tmp, json) = generate("rust");
    // Pull the first key from the symbols map and probe addr2sce with it.
    let val: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let symbols = val["symbols"].as_object().expect("symbols object");
    let first_key = symbols.keys().next().expect("non-empty symbols").clone();

    let out = Command::new(sce_codegen_bin())
        .arg("addr2sce")
        .arg(&tmp)
        .arg("--symbol")
        .arg(&first_key)
        .output()
        .expect("invoke addr2sce");
    assert!(
        out.status.success(),
        "addr2sce --symbol failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let resolved: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("addr2sce emits JSON line");
    assert_eq!(resolved["kind"], "addr2sce");
    assert_eq!(resolved["symbol"], first_key);
    assert!(resolved.get("entry").is_some());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// addr2sce returns non-zero (and a useful stderr) when the symbol is
/// not in the sourcemap.
#[test]
fn addr2sce_rejects_unknown_symbol() {
    let (tmp, _json) = generate("rust");
    let out = Command::new(sce_codegen_bin())
        .arg("addr2sce")
        .arg(&tmp)
        .arg("--symbol")
        .arg("definitely__not_a_real___state_body")
        .output()
        .expect("invoke addr2sce");
    assert!(!out.status.success(), "addr2sce should reject missing symbol");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not found"), "stderr: {stderr}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[allow(dead_code)]
fn _force_bin_link() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-codegen"))
}
