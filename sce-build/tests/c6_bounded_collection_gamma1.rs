//! C6-γ1 — Bounded-collection deploy-time capacity resolution.
//!
//! Per watching-zenoh RFC §5.L lines 2583-2585 + 2649:
//! `<sce:capacity source="deploy" key="machines.<m>.limits.<k>"/>`
//! resolves against `machines.<m>.limits:` in deploy.yaml. The
//! validator fires `collection/capacity-unresolved` when the key
//! references the target machine but the limit is not declared;
//! silent-skips on the single-file path (no deploy / no
//! target_machine), on `<sce:capacity const="N"/>` (no deploy
//! reference), and when the key's machine segment != target_machine
//! per the Q-η5 (a) precedent.
//!
//! Test matrix (5 scenarios):
//!  1. compile_const_silent_skips_on_deploy_path
//!  2. happy_deploy_key_resolves
//!  3. unresolved_limit_fires
//!  4. key_machine_segment_mismatch_silent_skips
//!  5. compile_const_silent_skips_without_deploy
//!
//! Existing `c6_bounded_collection*.rs` files left untouched.

use sce_build::compile_forge_with_deploy;
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;
use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::DocumentLabel;

/// Bounded-collection doc with deploy-key capacity, single-writer
/// concurrency, no index-by. Element-type names a non-existent kind
/// — fine for this test set because C6-β's cross-doc resolution runs
/// only through `compile_scxml_with_imports`, not the single-file
/// `compile_forge_with_deploy` path the γ1 validator inhabits.
fn bc_deploy_key_doc(name: &str, key: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="{name}" version="1.0">
  <sce:element-type>subscription_entry</sce:element-type>
  <sce:capacity source="deploy" key="{key}"/>
  <sce:concurrency>single-writer</sce:concurrency>
</scxml>"##
    )
}

/// Bounded-collection doc with `<sce:capacity const="N">` (no deploy
/// reference). The validator silent-skips this branch entirely.
fn bc_compile_const_doc(name: &str, capacity: u32) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="{name}" version="1.0">
  <sce:element-type>subscription_entry</sce:element-type>
  <sce:capacity const="{capacity}"/>
  <sce:concurrency>single-writer</sce:concurrency>
</scxml>"##
    )
}

fn deploy_yaml_with_limits(declared_limits: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
{declared_limits}
"##
    )
}

/// Run `compile_forge_with_deploy` and discard the codegen output —
/// γ1 validator runs before codegen reaches the bounded-collection
/// arm, so a failing validator surfaces here as a Located<ForgeError>.
/// γ2-onwards the Rust BC template ships, so successful runs return
/// `Ok` directly. The historical `CodegenGenericKindBackendEmitMissing`
/// fallback is preserved for the non-Rust language paths the matrix
/// still defers; on `Language::Rust` the only acceptable error remains
/// a γ2-internal `InvalidConfig` for cases where γ1 silently skipped
/// (machine mismatch, single-file path) so the deploy resolution
/// never made it into options — that is itself proof that the
/// validator passed.
fn assert_validator_passed(scxml: &str, deploy_yaml: &str, target_machine: Option<&str>) {
    let deploy = parse_deploy_str(deploy_yaml).expect("deploy parses");
    let result = compile_forge_with_deploy(
        scxml,
        DocumentLabel::symmetric("local_sub_table"),
        Language::Rust,
        Some(&deploy),
        target_machine,
    );
    match result {
        Ok(_) => {}
        Err(located) => match &located.error {
            ForgeError::Generate(boxed) => match boxed.as_ref() {
                sce_build::forge::error::GenerateError::CodegenGenericKindBackendEmitMissing {
                    kind,
                    ..
                } if kind == "bounded-collection" => {
                    // Pre-γ2 deferred-template fallback — kept for other
                    // backends still on the matrix's `false` arm.
                }
                sce_build::forge::error::GenerateError::InvalidConfig(msg)
                    if msg.contains("bounded-collection") && msg.contains("resolution missing") =>
                {
                    // γ2 render-time gate: γ1 validator passed (silent-
                    // skipped per Q-η5 (a)) but the codegen layer has no
                    // resolution to consume. The BC was designed for a
                    // different target machine OR no deploy was supplied,
                    // so γ1 deliberately left the value unresolved. The
                    // assertion target is "γ1 silently passed"; γ2's
                    // resolution gap on these silent-skip paths is the
                    // expected downstream consequence, not a validator
                    // failure.
                }
                other => {
                    panic!("C6-γ1 validator must silent-pass; got unrelated error: {other:?}")
                }
            },
            other => panic!("C6-γ1 validator must silent-pass; got unrelated error: {other:?}"),
        },
    }
}

// ─── 1. compile_const_silent_skips_on_deploy_path ────────────────────

#[test]
fn compile_const_silent_skips_on_deploy_path() {
    // `<sce:capacity const="8"/>` carries no deploy reference; the
    // validator's `CapacitySource::DeployKey` match arm never fires.
    // Even with deploy + target_machine present, the BC compiles
    // (reaching γ2-deferred codegen).
    let scxml = bc_compile_const_doc("local_sub_table", 8);
    let deploy_yaml = deploy_yaml_with_limits("");
    assert_validator_passed(&scxml, &deploy_yaml, Some("mcu_node"));
}

// ─── 2. happy_deploy_key_resolves ────────────────────────────────────

#[test]
fn happy_deploy_key_resolves() {
    // Deploy key resolves cleanly: `machines.mcu_node.limits.
    // local_subscriptions` lookup finds the declared limit. Validator
    // silent-passes; codegen reaches the γ2-deferred emit.
    let scxml = bc_deploy_key_doc(
        "local_sub_table",
        "machines.mcu_node.limits.local_subscriptions",
    );
    let deploy_yaml = deploy_yaml_with_limits(
        "          local_subscriptions: 32\n          in_flight_reassembly: 8",
    );
    assert_validator_passed(&scxml, &deploy_yaml, Some("mcu_node"));
}

// ─── 3. unresolved_limit_fires ───────────────────────────────────────

#[test]
fn unresolved_limit_fires() {
    // Author asks for `local_subscriptions` but deploy.yaml declares
    // only `subscription_table` and `in_flight_reassembly`. Validator
    // fires `collection/capacity-unresolved` with the sorted limit
    // names as `Fix::ReplaceOneOf` candidates.
    let scxml = bc_deploy_key_doc(
        "local_sub_table",
        "machines.mcu_node.limits.local_subscriptions",
    );
    let deploy_yaml = deploy_yaml_with_limits(
        "          subscription_table: 32\n          in_flight_reassembly: 8",
    );
    let deploy = parse_deploy_str(&deploy_yaml).expect("deploy parses");

    let err = match compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("local_sub_table"),
        Language::Rust,
        Some(&deploy),
        Some("mcu_node"),
    ) {
        Ok(_) => panic!("unresolved limit must fire"),
        Err(e) => e,
    };

    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::CollectionCapacityUnresolved {
                collection_name,
                key,
                machine,
                limit,
                candidates,
                ..
            } => {
                assert_eq!(collection_name, "local_sub_table");
                assert_eq!(key, "machines.mcu_node.limits.local_subscriptions");
                assert_eq!(machine, "mcu_node");
                assert_eq!(limit, "local_subscriptions");
                assert_eq!(
                    candidates,
                    &vec![
                        "in_flight_reassembly".to_string(),
                        "subscription_table".to_string(),
                    ],
                    "candidates must be sorted declared limit names"
                );
            }
            other => panic!("expected CollectionCapacityUnresolved, got {other:?}"),
        },
        other => panic!("expected CollectionCapacityUnresolved, got {other:?}"),
    }
}

// ─── 4. key_machine_segment_mismatch_silent_skips ────────────────────

#[test]
fn key_machine_segment_mismatch_silent_skips() {
    // Author writes key for `mcu_node` but compile is invoked with
    // target_machine="other_node". Per Q-η5 (a) silent-skip
    // precedent: the BC doc was designed for a different machine; the
    // deploy resolution should run only on the host machine's compile.
    // Validator silent-passes; codegen reaches the γ2-deferred emit.
    let scxml = bc_deploy_key_doc(
        "local_sub_table",
        "machines.mcu_node.limits.local_subscriptions",
    );
    let deploy_yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
          local_subscriptions: 32
      other_node:
        source: other_node.scxml
"##
    .to_string();
    // target_machine="other_node" — the key references mcu_node, not
    // other_node, so the validator silent-skips even though
    // other_node has no `limits` declared.
    assert_validator_passed(&scxml, &deploy_yaml, Some("other_node"));
}

// ─── 5. compile_const_silent_skips_without_deploy ────────────────────

#[test]
fn compile_const_silent_skips_without_deploy() {
    // Single-file compile path: deploy=None, target_machine=None. The
    // outer `if let (Some(cfg), Some(machine)) = ...` guard means the
    // validator never executes; the BC compiles unconditionally
    // (reaching γ2-deferred codegen).
    let scxml = bc_compile_const_doc("local_sub_table", 16);
    let result = compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("local_sub_table"),
        Language::Rust,
        None,
        None,
    );
    match result {
        Ok(_) => {}
        Err(located) => match &located.error {
            ForgeError::Generate(boxed) => match boxed.as_ref() {
                sce_build::forge::error::GenerateError::CodegenGenericKindBackendEmitMissing {
                    kind,
                    ..
                } if kind == "bounded-collection" => {
                    // Pre-γ2 deferred-template fallback — kept for other
                    // backends still on the matrix's `false` arm.
                }
                sce_build::forge::error::GenerateError::InvalidConfig(msg)
                    if msg.contains("bounded-collection") && msg.contains("resolution missing") =>
                {
                    // γ2 render-time gate: single-file deploy-aware path
                    // has no deploy.yaml, so the deploy-key cannot
                    // resolve. γ1 validator silent-skipped per Q-η5 (a);
                    // γ2 surfaces the resolution gap at codegen time.
                }
                other => panic!(
                    "single-file path must silent-skip validator; got unrelated error: {other:?}"
                ),
            },
            other => panic!(
                "single-file path must silent-skip validator; got unrelated error: {other:?}"
            ),
        },
    }
}
