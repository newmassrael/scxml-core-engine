// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Protocol-Synthesis RFC §synth-5-O — sourcemap + symbol mangling +
// addr2sce integration fixture.
//
// The contract (spec lines 3055-3057, 3219-3243, 3253-3278,
// 3321-3324):
//
//   D18(i)   Sourcemap JSON shape — version + source_hash +
//            template_hash + symbols map.
//   D18(ii)  Byte-identity across the 6 backends for the same SCXML.
//   D18(iii) source_hash byte-equal to §synth-6.2.6 drift header.
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
    let tmp = std::env::temp_dir().join(format!("sce_atomic1_{lang}_{:x}", rand_suffix(),));
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
/// it. Only checks the non-language-specific portion: the
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
            if let Some(file) = v
                .get_mut("scxml_file")
                .and_then(|f| f.as_str())
                .map(String::from)
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

/// D18(iii) — sourcemap.source_hash byte-equal to the §synth-6.2.6 header
/// `source-hash` value embedded in the generated SM file.
#[test]
fn sourcemap_source_hash_matches_drift_header() {
    let (tmp, json) = generate("rust");
    let val: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let map_hash = val["source_hash"].as_str().unwrap().to_string();

    // Parse the §synth-6.2.6 header from the emitted *_sm.rs.
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
    assert_eq!(
        (a.as_str(), b.as_str(), c.as_str()),
        ("motor", "running", "_state_body")
    );

    // Machine name with literal `__` escapes to `_u_` and round-trips.
    let m2 = mangle("ma__ch", "s1", "_state_body");
    let (a2, b2, c2) = demangle(&m2).unwrap();
    assert_eq!(
        (a2.as_str(), b2.as_str(), c2.as_str()),
        ("ma__ch", "s1", "_state_body")
    );

    // State path with hierarchy separator flattens to `_`.
    let m3 = mangle("m", "s1/s1p1", "_state_body");
    assert!(m3.contains("s1_s1p1"));
}

/// D18(v.a) — `traceability/state-id-collision` fires on a synthetic
/// duplicate-mangled-symbol scenario (constructed at the library
/// level since the SCXML parser already rejects duplicate state ids
/// via `ValidationDuplicateId`; the §synth-5-O collision case kicks in
/// only after XInclude / template composition unifies two distinct
/// fragments).
#[test]
fn state_id_collision_diagnostic_payload_shape() {
    use sce_build::forge::diagnostic::ToDiagnostics;
    use sce_build::forge::error::ValidationError;
    let err =
        sce_build::forge::error::ForgeError::from(ValidationError::TraceabilityStateIdCollision {
            mangled: "m__dup___state_body".into(),
            first_file: "a.scxml".into(),
            first_line: 7,
            second_file: "b.scxml".into(),
            second_line: 11,
        });
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
    assert_eq!(code_str, "\"traceability/sourcemap-source-hash-mismatch\"");
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
    assert_eq!(
        out.status.code(),
        Some(1),
        "a miss exits 1 — the status SCE_ERROR_CONTRACT.md §6 registers \
         for `cli/query-no-match`",
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("matched nothing"), "stderr: {stderr}");
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── Reverse direction: sce2sym ──────────────────────────────────
//
// `addr2sce` answers "which SCXML produced this symbol"; `sce2sym`
// answers the opposite. The two read one table from opposite ends, so
// the load-bearing claim is that they agree about it — asserted below
// as a round trip over every symbol the fixture emits, not over one
// hand-picked entry.

/// Run `sce2sym` and return (exit-success, NDJSON records).
fn sce2sym(args: &[&str]) -> (bool, Vec<serde_json::Value>) {
    let out = Command::new(sce_codegen_bin())
        .arg("sce2sym")
        .args(args)
        .output()
        .expect("invoke sce2sym");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let records = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("record is JSON: {e}\n{l}")))
        .collect();
    (out.status.success(), records)
}

/// Every symbol resolves forward and back to itself.
///
/// For each entry in the sourcemap: `addr2sce --symbol S` yields
/// coordinates, and `sce2sym` on those coordinates must list `S`
/// again. A one-symbol spot check would pass even if the reverse
/// filter mishandled a whole kind (the machine symbol carries an empty
/// state path, transitions carry events, and so on), so the round trip
/// is asserted over the full table.
#[test]
fn every_symbol_round_trips_between_the_two_directions() {
    let (tmp, json) = generate("rust");
    let map: serde_json::Value = serde_json::from_str(&json).expect("sourcemap JSON");
    let symbols = map["symbols"].as_object().expect("symbols object");
    assert!(
        symbols.len() >= 3,
        "fixture must emit several symbols to make the round trip meaningful; got {}",
        symbols.len(),
    );

    let mut checked = 0usize;
    for (name, entry) in symbols {
        let state = entry["scxml_state_path"].as_str().expect("state path");
        let kind = entry["kind"].as_str().expect("kind");
        let (ok, records) = sce2sym(&[tmp.to_str().unwrap(), "--state", state, "--kind", kind]);
        assert!(
            ok,
            "sce2sym found nothing for {name} (state={state:?} kind={kind:?})"
        );
        let names: Vec<&str> = records
            .iter()
            .map(|r| r["symbol"].as_str().expect("symbol"))
            .collect();
        assert!(
            names.contains(&name.as_str()),
            "reverse lookup for state={state:?} kind={kind:?} did not list {name}; got {names:?}",
        );
        // Every record must agree with the forward table verbatim.
        for record in &records {
            let listed = record["symbol"].as_str().unwrap();
            assert_eq!(
                &record["entry"], &map["symbols"][listed],
                "sce2sym reported an entry that differs from the sourcemap row for {listed}",
            );
        }
        checked += 1;
    }
    assert_eq!(
        checked,
        symbols.len(),
        "round trip must cover every symbol in the table",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// One invocation spans several backends' sidecars, and each record
/// names the one it came from.
#[test]
fn sce2sym_queries_several_backends_in_one_invocation() {
    let (rust_dir, _) = generate("rust");
    let (cpp_dir, _) = generate("cpp");
    let (ok, records) = sce2sym(&[
        rust_dir.to_str().unwrap(),
        cpp_dir.to_str().unwrap(),
        "--state",
        "s1",
    ]);
    assert!(ok, "s1 must resolve in both backends");

    let mut sources: Vec<String> = records
        .iter()
        .map(|r| r["sourcemap"].as_str().expect("sourcemap path").to_string())
        .collect();
    sources.sort();
    sources.dedup();
    assert_eq!(
        sources.len(),
        2,
        "records must be attributed to both sidecars; got {sources:?}",
    );
    for record in &records {
        assert_eq!(record["kind"], "sce2sym");
        assert_eq!(record["v"], 1);
    }
    let _ = std::fs::remove_dir_all(&rust_dir);
    let _ = std::fs::remove_dir_all(&cpp_dir);
}

/// Filters intersect rather than union — a query naming a real state
/// and a kind that state has no symbol for must miss, not fall back to
/// the state alone.
#[test]
fn sce2sym_filters_intersect() {
    let (tmp, _) = generate("rust");
    let (ok_state, by_state) = sce2sym(&[tmp.to_str().unwrap(), "--state", "s1"]);
    assert!(ok_state, "s1 exists");
    assert!(
        by_state.iter().any(|r| r["entry"]["kind"] == "transition"),
        "s1 has a transition symbol",
    );

    // s1 has no on_exit block, so intersecting with that kind misses.
    let (ok_both, records) =
        sce2sym(&[tmp.to_str().unwrap(), "--state", "s1", "--kind", "on_exit"]);
    assert!(
        !ok_both && records.is_empty(),
        "intersecting a real state with an absent kind must miss: {records:?}",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Transition symbols carry their triggering event, and nothing else
/// does.
///
/// The sourcemap has always declared an `event` field and the schema
/// has always documented it, but the value came from an
/// `extract_event(artifact)` stub that returned `None` unconditionally
/// — the artifact string is `_transition_<idx>` and never held the
/// name. Across the 404 committed sidecars the key appeared zero
/// times. This pins that it now appears exactly where it is claimed to.
#[test]
fn transition_symbols_carry_their_event() {
    let (tmp, json) = generate("rust");
    let map: serde_json::Value = serde_json::from_str(&json).expect("sourcemap JSON");

    let mut with_event = 0usize;
    for (name, entry) in map["symbols"].as_object().expect("symbols") {
        let kind = entry["kind"].as_str().expect("kind");
        match entry.get("event").and_then(|v| v.as_str()) {
            Some(event) => {
                assert_eq!(
                    kind, "transition",
                    "only transitions carry an event; {name} is a {kind}",
                );
                assert!(!event.is_empty(), "{name} carries an empty event name");
                with_event += 1;
            }
            None => assert_ne!(
                kind, "transition",
                "{name} is a transition with a declared event but no event on the wire",
            ),
        }
    }
    assert!(
        with_event >= 1,
        "the fixture declares `event=\"go\"`, so at least one symbol must carry it",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// `--event` selects only symbols that actually carry that event.
///
/// The negative half is the load-bearing one: a symbol with no event
/// must not satisfy an event constraint, otherwise the filter would
/// answer "which symbol handles event X" with every state body in the
/// machine.
#[test]
fn sce2sym_event_filter_excludes_symbols_without_that_event() {
    let (tmp, json) = generate("rust");
    let map: serde_json::Value = serde_json::from_str(&json).expect("sourcemap JSON");
    let dir = tmp.to_str().unwrap();

    let (ok, records) = sce2sym(&[dir, "--event", "go"]);
    assert!(ok, "the fixture declares event=\"go\"");
    assert!(!records.is_empty());
    for record in &records {
        assert_eq!(
            record["entry"]["event"], "go",
            "an --event query returned a symbol carrying a different event (or none)",
        );
    }

    // Every eventless symbol in the table must be absent from that result.
    let selected: Vec<&str> = records
        .iter()
        .map(|r| r["symbol"].as_str().unwrap())
        .collect();
    let mut eventless = 0usize;
    for (name, entry) in map["symbols"].as_object().expect("symbols") {
        if entry.get("event").is_none() {
            eventless += 1;
            assert!(
                !selected.contains(&name.as_str()),
                "{name} carries no event but matched --event go",
            );
        }
    }
    assert!(
        eventless >= 2,
        "the fixture must contain eventless symbols for this to prove anything; got {eventless}",
    );

    // An event nobody declares matches nothing.
    let (ok_missing, missing) = sce2sym(&[dir, "--event", "no_such_event"]);
    assert!(!ok_missing && missing.is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

/// `--line` matches a symbol whose range contains the line, and the
/// range is inclusive at both ends.
///
/// Asserted against the table's own `line_range` values rather than
/// literals so the test cannot drift from the fixture's formatting:
/// both endpoints must hit, and the lines immediately outside must not.
#[test]
fn sce2sym_line_filter_is_inclusive_at_both_ends() {
    let (tmp, json) = generate("rust");
    let map: serde_json::Value = serde_json::from_str(&json).expect("sourcemap JSON");
    let dir = tmp.to_str().unwrap();

    let mut probed = 0usize;
    for (name, entry) in map["symbols"].as_object().expect("symbols") {
        let range = entry["line_range"].as_array().expect("line_range");
        let start = range[0].as_u64().expect("start");
        let end = range[1].as_u64().expect("end");

        for endpoint in [start, end] {
            let (ok, records) = sce2sym(&[dir, "--line", &endpoint.to_string()]);
            assert!(ok, "line {endpoint} must match at least {name}");
            let names: Vec<&str> = records
                .iter()
                .map(|r| r["symbol"].as_str().unwrap())
                .collect();
            assert!(
                names.contains(&name.as_str()),
                "line {endpoint} is an endpoint of {name}'s range [{start},{end}] \
                 but the lookup did not list it; got {names:?}",
            );
        }

        // A line outside the range must not report this symbol. Guard
        // against underflow at line 0, which is not a valid 1-based line.
        if start > 1 {
            let (_, records) = sce2sym(&[dir, "--line", &(start - 1).to_string()]);
            let names: Vec<&str> = records
                .iter()
                .map(|r| r["symbol"].as_str().unwrap())
                .collect();
            assert!(
                !names.contains(&name.as_str()),
                "line {} is before {name}'s range [{start},{end}] but it was listed",
                start - 1,
            );
        }
        let (_, records) = sce2sym(&[dir, "--line", &(end + 1).to_string()]);
        let names: Vec<&str> = records
            .iter()
            .map(|r| r["symbol"].as_str().unwrap())
            .collect();
        assert!(
            !names.contains(&name.as_str()),
            "line {} is past {name}'s range [{start},{end}] but it was listed",
            end + 1,
        );
        probed += 1;
    }
    assert!(
        probed >= 3,
        "line probe covered only {probed} symbols; the fixture should emit more",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// A query matching nothing exits non-zero, so a build gate asking
/// "did this state lower to anything" can fail on the answer.
#[test]
fn sce2sym_rejects_a_query_that_matches_nothing() {
    let (tmp, _) = generate("rust");
    let out = Command::new(sce_codegen_bin())
        .arg("sce2sym")
        .arg(&tmp)
        .arg("--state")
        .arg("definitely_not_a_state")
        .output()
        .expect("invoke sce2sym");
    // §6 gives a miss its own status, distinct from the CLI-boundary
    // 20: the tool ran and the answer was "none".
    assert_eq!(
        out.status.code(),
        Some(1),
        "a miss exits 1 — the status SCE_ERROR_CONTRACT.md §6 registers \
         for `cli/query-no-match`",
    );
    assert!(out.stdout.is_empty(), "a miss must emit no records");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("matched nothing"), "stderr: {stderr}");
    let _ = std::fs::remove_dir_all(&tmp);
}

/// An unconstrained query lists the whole table — the "what did this
/// document lower to" question a key-demanding lookup cannot express.
#[test]
fn sce2sym_without_filters_lists_every_symbol() {
    let (tmp, json) = generate("rust");
    let map: serde_json::Value = serde_json::from_str(&json).expect("sourcemap JSON");
    let expected = map["symbols"].as_object().expect("symbols").len();
    let (ok, records) = sce2sym(&[tmp.to_str().unwrap()]);
    assert!(ok);
    assert_eq!(
        records.len(),
        expected,
        "an unconstrained query must list every symbol",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Both directions emit records valid against the shared wire schema.
#[test]
fn both_lookup_directions_validate_against_the_wire_schema() {
    let schema_value: serde_json::Value = serde_json::from_str(include_str!(
        "../../schemas/sce-symbol-lookup.v1.schema.json"
    ))
    .expect("lookup schema is JSON");

    let (tmp, json) = generate("rust");
    let map: serde_json::Value = serde_json::from_str(&json).expect("sourcemap JSON");
    let first = map["symbols"]
        .as_object()
        .expect("symbols")
        .keys()
        .next()
        .expect("at least one symbol")
        .clone();

    let forward = Command::new(sce_codegen_bin())
        .arg("addr2sce")
        .arg(&tmp)
        .arg("--symbol")
        .arg(&first)
        .output()
        .expect("invoke addr2sce");
    assert!(forward.status.success());
    let forward_line = String::from_utf8_lossy(&forward.stdout).trim().to_string();

    let (ok, reverse) = sce2sym(&[tmp.to_str().unwrap()]);
    assert!(ok);

    let mut instances: Vec<serde_json::Value> =
        vec![serde_json::from_str(&forward_line).expect("forward record is JSON")];
    instances.extend(reverse);
    assert!(
        instances.len() >= 2,
        "both directions must contribute records",
    );

    for instance in &instances {
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema_value)
            .expect("lookup schema compiles");
        let msgs: Vec<String> = match validator.validate(instance) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        };
        assert!(
            msgs.is_empty(),
            "lookup record violates the wire schema: {msgs:?}\n{instance}",
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// The lookup schema rejects a record missing the generator stamp.
///
/// `SCE_WIRE_CONTRACTS.md` policy item 5 requires a negative case per
/// surface, because a positive sweep alone proves only that everything
/// is accepted — a schema that validated every input would pass it.
/// This surface had no negative case at all until this one, so
/// `both_lookup_directions_validate_against_the_wire_schema` was
/// certifying acceptance without ever showing the validator refusing
/// anything.
///
/// The control assertion is what gives the rejection meaning: the
/// record is proven valid before the single field is removed, so the
/// refusal is pinned to that removal and not to some other constraint
/// a hand-typed record would have tripped first.
#[test]
fn lookup_schema_rejects_a_record_without_the_generator_stamp() {
    let schema_value: serde_json::Value = serde_json::from_str(include_str!(
        "../../schemas/sce-symbol-lookup.v1.schema.json"
    ))
    .expect("lookup schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("lookup schema compiles");

    let (tmp, _json) = generate("rust");
    let (ok, mut records) = sce2sym(&[tmp.to_str().unwrap()]);
    assert!(ok);
    let mut record = records.remove(0);

    assert!(
        validator.validate(&record).is_ok(),
        "the control record must be valid before mutation, otherwise \
         the rejection below proves nothing: {record}",
    );
    record
        .as_object_mut()
        .expect("record is an object")
        .remove("generator")
        .expect("control record carried the stamp");
    assert!(
        validator.validate(&record).is_err(),
        "schema must reject a lookup record with no generator stamp: {record}",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Both directions name the generator that produced the answer.
///
/// A lookup record is consumed on its own — a debugger resolving a
/// fault address, a build step mapping a symbol back to its SCXML — with
/// no manifest alongside it, so it has to carry its own attribution or
/// have none. `SCE_WIRE_CONTRACTS.md` policy 1 makes pinning a specific
/// SCE commit the consumer's obligation while the surface is
/// `pre-release`; the record naming the commit is what makes that
/// checkable rather than assumed.
///
/// The sourcemap the record points at cannot stand in for this: its
/// `source_hash` / `template_hash` identify the *inputs* (document bytes,
/// template tree), and two generators whose emit code differs while the
/// templates and document do not produce the same pair of hashes.
#[test]
fn both_lookup_directions_name_the_generator_commit() {
    let version_out = Command::new(sce_codegen_bin())
        .arg("--version")
        .output()
        .expect("sce-codegen must be runnable");
    let version_text = String::from_utf8_lossy(&version_out.stdout).into_owned();
    let expected = version_text
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(c, _)| c.to_string())
        .expect("--version carries a commit");

    let (tmp, json) = generate("rust");
    let map: serde_json::Value = serde_json::from_str(&json).expect("sourcemap JSON");
    let first = map["symbols"]
        .as_object()
        .expect("symbols")
        .keys()
        .next()
        .expect("at least one symbol")
        .clone();

    let forward = Command::new(sce_codegen_bin())
        .arg("addr2sce")
        .arg(&tmp)
        .arg("--symbol")
        .arg(&first)
        .output()
        .expect("invoke addr2sce");
    assert!(forward.status.success());
    let forward_record: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&forward.stdout).trim())
            .expect("forward record is JSON");

    let (ok, reverse) = sce2sym(&[tmp.to_str().unwrap()]);
    assert!(ok);

    let mut records = vec![forward_record];
    records.extend(reverse);
    assert!(
        records.len() >= 2,
        "both directions must contribute records"
    );
    for record in &records {
        assert_eq!(
            record["generator"], expected,
            "every lookup record must name the generator commit: {record}",
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// `--pc` / `--hardfault` refuse loudly when the image they were given
/// cannot answer, rather than resolving to nothing or exiting 0. The
/// resolution paths themselves are pinned by
/// `sce-build/tests/addr2sce_pc.rs`, which synthesises ELF images with
/// known symbol layouts; this case covers the argument-level refusals
/// that share the `--symbol` entry point.
#[test]
fn addr2sce_pc_modes_refuse_an_image_that_cannot_answer() {
    let (tmp, _) = generate("rust");

    // `/dev/null` is the degenerate image: readable, but not an ELF.
    // That is a CLI-boundary input failure — SCE_ERROR_CONTRACT §6 maps
    // `cli/*` to exit 20 — not an argument error, which is what exit 2
    // would claim.
    for args in [
        vec!["--pc", "0x08001234", "--elf", "/dev/null"],
        vec!["--hardfault", "--elf", "/dev/null"],
    ] {
        let out = Command::new(sce_codegen_bin())
            .arg("addr2sce")
            .arg(&tmp)
            .args(&args)
            .output()
            .expect("invoke addr2sce");
        assert_eq!(
            out.status.code(),
            Some(20),
            "mode {args:?} must refuse an unparseable image as cli/* (exit 20)",
        );
        assert!(
            out.stdout.is_empty(),
            "mode {args:?} must emit no record when the image is unusable",
        );
    }

    // Omitting `--elf` is an argument error, and §6 has one status for
    // those too: `cli/usage`, exit 20. This line asserted exit 2 for
    // two rounds, matching what the code did — and 2 is the status §6
    // assigns to `xml/*`, so the caller was told its document was
    // malformed. The correct rule was already written twenty lines
    // above, in this same test.
    let out = Command::new(sce_codegen_bin())
        .arg("addr2sce")
        .arg(&tmp)
        .args(["--pc", "0x08001234"])
        .output()
        .expect("invoke addr2sce");
    assert_eq!(
        out.status.code(),
        Some(20),
        "a malformed invocation is `cli/usage` (exit 20), not exit 2 — \
         2 belongs to `xml/*`",
    );

    // The help text must describe the modes as they behave. It said
    // "NOT IMPLEMENTED" while they exited 2; it must not keep saying so
    // now that they resolve.
    let help = Command::new(sce_codegen_bin())
        .arg("addr2sce")
        .arg("--help")
        .output()
        .expect("invoke addr2sce --help");
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(
        !text.contains("NOT IMPLEMENTED"),
        "help still presents an implemented mode as unimplemented:\n{text}",
    );
    assert!(
        text.contains("--hardfault"),
        "help must document the mode it ships:\n{text}",
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[allow(dead_code)]
fn _force_bin_link() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-codegen"))
}
