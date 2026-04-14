// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// External infrastructure config resolution (SCE_MESH.md §13).
//
// This stage runs between deploy.yaml parsing and topology analysis:
//
//   1. For each device that declares `transports.someip.config:`, load
//      the referenced vsomeip.json into a `VsomeipConfig`.
//   2. For each binding on a SOME/IP target, resolve any name-based
//      references (`service`, `method`, `event_group`, `getter`, `setter`)
//      into numeric IDs and inject them into `binding.extra` under the
//      keys the existing template reads (`service_id`, `method_id`, etc.).
//   3. Batch all unresolved names into one error per (machine, config)
//      pair so operators see every mismatch at once.
//
// After this stage the rest of the pipeline (pattern validation, codegen)
// sees a DeployConfig that is indistinguishable from one with inline
// numeric IDs — templates do not branch on "was this resolved or inline".
//
// Stage 1 deprecation (SCE_MESH.md §13, §14): bindings may carry inline
// numeric IDs (`service_id: "0x1234"`) alongside name-based references.
// Inline IDs are tolerated but emit a `DeprecationWarning`; a name-based
// reference that resolves takes precedence over an inline value.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::diagnostics::DeprecationWarning;
use crate::mesh::deploy::{BindingConfig, DeployConfig};
use crate::mesh::error::{ExternalConfigError, UnresolvedName};
use crate::mesh::target::TargetId;
use crate::mesh::vsomeip_config::VsomeipConfig;

/// Keys the existing mesh_transport.h.jinja2 template reads out of
/// `target.extra`. Kept here as constants so the injection site and the
/// template are at least grep-compatible.
const KEY_SERVICE_ID: &str = "service_id";
const KEY_INSTANCE_ID: &str = "instance_id";
const KEY_METHOD_ID: &str = "method_id";
const KEY_EVENT_GROUP_ID: &str = "event_group_id";
const KEY_EVENT_ID: &str = "event_id";
const KEY_GETTER_ID: &str = "getter_id";
const KEY_SETTER_ID: &str = "setter_id";

/// Deploy.yaml inline numeric ID keys that are Stage 1 deprecated.
/// Each appears in a `DeprecationWarning` when encountered.
const DEPRECATED_INLINE_IDS: &[&str] = &[
    KEY_SERVICE_ID,
    KEY_INSTANCE_ID,
    KEY_METHOD_ID,
    KEY_EVENT_GROUP_ID,
    KEY_EVENT_ID,
    KEY_GETTER_ID,
    KEY_SETTER_ID,
];

/// Result of the external-config resolution stage.
#[derive(Debug)]
pub struct ExternalResolution {
    /// Deprecation warnings for inline numeric IDs and other Stage 1
    /// tolerations. Surfaced on `MeshResult` for CLI emission.
    pub deprecation_warnings: Vec<DeprecationWarning>,
}

/// Resolve all name-based external references in `deploy_cfg` in place.
///
/// `deploy_dir` is the directory containing deploy.yaml; external config
/// paths are resolved relative to it (absolute paths are honored as-is).
///
/// Mutates `deploy_cfg` so downstream stages see numeric IDs in
/// `BindingConfig.extra` without having to know about the external config
/// layer.
pub fn resolve_external_bindings(
    deploy_cfg: &mut DeployConfig,
    deploy_dir: &Path,
) -> Result<ExternalResolution, ExternalConfigError> {
    let mut deprecation_warnings = Vec::new();

    // Cache parsed vsomeip.json per device — the same file can be referenced
    // by many bindings and there's no value in parsing it more than once.
    let mut someip_cache: HashMap<String, (PathBuf, VsomeipConfig)> = HashMap::new();

    // `iter_mut` on HashMap yields a stable view; order within a device is
    // not deterministic but that does not matter because we batch all
    // unresolved names before erroring.
    for (device_name, device) in deploy_cfg.topology.iter_mut() {
        // Parse vsomeip.json once if referenced.
        let someip_config_path =
            device.transports.someip.as_ref().and_then(|s| s.config.clone());

        if let Some(rel_path) = someip_config_path.as_ref() {
            let abs_path = resolve_relative(deploy_dir, rel_path);
            let cfg = VsomeipConfig::load(&abs_path)?;
            someip_cache.insert(device_name.clone(), (abs_path, cfg));
        }

        // Resolve each machine's bindings against the device's external config.
        for (machine_name, machine) in device.machines.iter_mut() {
            let mut missing: Vec<UnresolvedName> = Vec::new();

            // First collect deprecation warnings (read-only, borrow-safe).
            collect_inline_id_deprecations(
                machine_name,
                &machine.bindings,
                &mut deprecation_warnings,
            );

            for (target, binding) in machine.bindings.iter_mut() {
                if binding.transport != "someip" {
                    // Only SOME/IP currently uses name-based resolution.
                    // Zenoh bindings are resolved at runtime from zenoh.json5;
                    // other transports have no external config concept.
                    ensure_no_stray_someip_names(
                        machine_name,
                        device_name,
                        target,
                        binding,
                    )?;
                    continue;
                }

                let needs_resolution = binding_uses_named_references(binding);
                if !needs_resolution {
                    // All-inline binding — Stage 1 deprecation path. The
                    // DeprecationWarning was already recorded above; nothing
                    // to resolve.
                    continue;
                }

                // Resolution needed → device must declare transports.someip.config.
                let (config_path, someip) = someip_cache.get(device_name).ok_or_else(|| {
                    ExternalConfigError::MissingConfigReference {
                        machine: machine_name.clone(),
                        device: device_name.clone(),
                        target: target.as_str().to_string(),
                    }
                })?;

                resolve_binding_names(
                    machine_name,
                    target,
                    binding,
                    someip,
                    config_path.as_path(),
                    &mut missing,
                )?;
            }

            if !missing.is_empty() {
                let (config_path, _) = someip_cache
                    .get(device_name)
                    .expect("someip_cache populated when names need resolution");
                return Err(ExternalConfigError::UnresolvedNames {
                    machine: machine_name.clone(),
                    config_path: config_path.display().to_string(),
                    missing,
                });
            }
        }
    }

    Ok(ExternalResolution {
        deprecation_warnings,
    })
}

/// True iff any name-based SOME/IP reference is set on this binding.
fn binding_uses_named_references(binding: &BindingConfig) -> bool {
    binding.service.is_some()
        || binding.method.is_some()
        || binding.event_group.is_some()
        || binding.getter.is_some()
        || binding.setter.is_some()
}

/// Non-SOME/IP bindings must not carry SOME/IP-only name-based fields —
/// catches misconfigured deploy.yaml at build time instead of letting the
/// values be silently ignored by the template.
fn ensure_no_stray_someip_names(
    machine: &str,
    device: &str,
    target: &TargetId,
    binding: &BindingConfig,
) -> Result<(), ExternalConfigError> {
    if binding_uses_named_references(binding) {
        // Reuse MissingConfigReference's shape for the diagnostic — the
        // correction path is the same ("declare transports.someip.config
        // or remove the SOME/IP-only fields").
        return Err(ExternalConfigError::MissingConfigReference {
            machine: machine.to_string(),
            device: device.to_string(),
            target: target.as_str().to_string(),
        });
    }
    Ok(())
}

/// Record a `DeprecationWarning` for every Stage 1 inline numeric ID
/// present on any binding of this machine. Deduplicated by (target, key)
/// so a single deploy.yaml does not spam N identical warnings.
fn collect_inline_id_deprecations(
    machine: &str,
    bindings: &HashMap<TargetId, BindingConfig>,
    out: &mut Vec<DeprecationWarning>,
) {
    for (target, binding) in bindings {
        for &key in DEPRECATED_INLINE_IDS {
            if binding.extra.contains_key(key) {
                out.push(DeprecationWarning {
                    attribute: format!("{key}:"),
                    event: Some(format!("{machine} {target}")),
                    reason: format!(
                        "inline SOME/IP numeric IDs are deprecated (SCE_MESH.md §13). \
                         Replace with name-based reference (e.g. `service:`, `method:`) \
                         and declare `transports.someip.config:` at the device level."
                    ),
                });
            }
        }
    }
}

/// Resolve the name-based references on a single binding by mutating
/// `binding.extra`. Unresolved names are appended to `missing` (not
/// returned) so all mismatches for a machine are batched.
fn resolve_binding_names(
    _machine: &str,
    _target: &TargetId,
    binding: &mut BindingConfig,
    someip: &VsomeipConfig,
    _config_path: &Path,
    missing: &mut Vec<UnresolvedName>,
) -> Result<(), ExternalConfigError> {
    // Service → service_id + instance_id. Without a service there is
    // nothing for method/event_group/getter/setter to hang off, so
    // method-level resolutions short-circuit when service is unset or
    // unresolved.
    let service = binding.service.as_deref();

    let service_ref = match service {
        Some(name) => match someip.resolve_service(name) {
            Some(svc) => {
                insert_u16(&mut binding.extra, KEY_SERVICE_ID, svc.service);
                insert_u16(&mut binding.extra, KEY_INSTANCE_ID, svc.instance);
                Some(svc)
            }
            None => {
                missing.push(UnresolvedName {
                    kind: "service",
                    name: name.to_string(),
                    context: None,
                });
                None
            }
        },
        None => None,
    };

    if let Some(name) = binding.method.as_deref() {
        match service_ref.and_then(|svc| {
            svc.methods
                .iter()
                .find(|m| m.name == name)
                .map(|m| m.method)
        }) {
            Some(id) => insert_u16(&mut binding.extra, KEY_METHOD_ID, id),
            None => missing.push(UnresolvedName {
                kind: "method",
                name: name.to_string(),
                context: service.map(|s| format!("in service \"{s}\"")),
            }),
        }
    }

    if let Some(name) = binding.getter.as_deref() {
        match service_ref.and_then(|svc| {
            svc.methods
                .iter()
                .find(|m| m.name == name)
                .map(|m| m.method)
        }) {
            Some(id) => insert_u16(&mut binding.extra, KEY_GETTER_ID, id),
            None => missing.push(UnresolvedName {
                kind: "getter",
                name: name.to_string(),
                context: service.map(|s| format!("in service \"{s}\"")),
            }),
        }
    }

    if let Some(name) = binding.setter.as_deref() {
        match service_ref.and_then(|svc| {
            svc.methods
                .iter()
                .find(|m| m.name == name)
                .map(|m| m.method)
        }) {
            Some(id) => insert_u16(&mut binding.extra, KEY_SETTER_ID, id),
            None => missing.push(UnresolvedName {
                kind: "setter",
                name: name.to_string(),
                context: service.map(|s| format!("in service \"{s}\"")),
            }),
        }
    }

    if let Some(name) = binding.event_group.as_deref() {
        match service_ref
            .and_then(|svc| svc.eventgroups.iter().find(|e| e.name == name))
        {
            Some(eg) => {
                insert_u16(&mut binding.extra, KEY_EVENT_GROUP_ID, eg.eventgroup);
                // Event group must contain exactly one event for the
                // current one-event-per-binding template. Multi-event and
                // zero-event cases are diagnosed with dedicated errors —
                // silently picking events[0] or 0 would route nothing.
                match eg.events.len() {
                    0 => {
                        return Err(ExternalConfigError::EmptyEventGroup {
                            machine: _machine.to_string(),
                            target: _target.as_str().to_string(),
                            config_path: _config_path.display().to_string(),
                            event_group: name.to_string(),
                        });
                    }
                    1 => {
                        insert_u16(&mut binding.extra, KEY_EVENT_ID, eg.events[0]);
                    }
                    n => {
                        return Err(ExternalConfigError::AmbiguousEventGroup {
                            machine: _machine.to_string(),
                            target: _target.as_str().to_string(),
                            config_path: _config_path.display().to_string(),
                            event_group: name.to_string(),
                            count: n,
                        });
                    }
                }
            }
            None => missing.push(UnresolvedName {
                kind: "event_group",
                name: name.to_string(),
                context: service.map(|s| format!("in service \"{s}\"")),
            }),
        }
    }

    Ok(())
}

/// Insert a u16 id into `binding.extra` as a hex-string YAML value.
///
/// The existing someip template renders values with default Jinja2
/// formatting and the vsomeip C++ API accepts any u16 integer literal,
/// so either hex or decimal would work. Hex-string mirrors the inline
/// deploy.yaml convention (`"0x1234"`) and keeps diagnostic parity: a
/// resolved binding reads the same as a legacy inline one.
fn insert_u16(extra: &mut HashMap<String, serde_yaml_ng::Value>, key: &str, value: u16) {
    extra.insert(
        key.to_string(),
        serde_yaml_ng::Value::String(format!("0x{value:04X}")),
    );
}

/// Join `base` with `rel`, honoring absolute paths.
fn resolve_relative(base: &Path, rel: &Path) -> PathBuf {
    if rel.is_absolute() {
        rel.to_path_buf()
    } else {
        base.join(rel)
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(content.as_bytes()).expect("write");
        path
    }

    const SAMPLE_VSOMEIP: &str = r#"{
      "applications": [ { "name": "brake_app" } ],
      "services": [{
        "name": "motor_control",
        "service": "0x1234",
        "instance": "0x0001",
        "methods": [
          { "name": "compute_force", "method": "0x0421" },
          { "name": "get_status", "method": "0x0100" },
          { "name": "set_mode", "method": "0x0101" }
        ],
        "eventgroups": [
          { "name": "status_group", "eventgroup": "0x0001",
            "events": ["0x8001"] },
          { "name": "multi", "eventgroup": "0x0002",
            "events": ["0x8001", "0x8002"] },
          { "name": "empty", "eventgroup": "0x0003", "events": [] }
        ]
      }]
    }"#;

    fn sample_deploy(config_path: &Path) -> DeployConfig {
        let yaml = format!(
            r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: {config}
        application_name: brake_app
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            method: compute_force
            event_group: status_group
            getter: get_status
            setter: set_mode
            protocol: udp
"##,
            config = config_path.display()
        );
        crate::mesh::deploy::parse_deploy_str(&yaml).expect("parse")
    }

    #[test]
    fn resolves_all_name_based_references() {
        let dir = std::env::temp_dir().join("sce_ext_cfg_ok");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = write_tmp(&dir, "vsomeip.json", SAMPLE_VSOMEIP);

        let mut deploy = sample_deploy(&cfg_path);
        let res = resolve_external_bindings(&mut deploy, &dir).expect("resolve");

        let binding = &deploy.topology["ecu1"].machines["brake"].bindings["#motor"];
        let id = |k: &str| {
            binding
                .extra
                .get(k)
                .and_then(|v| v.as_str())
                .expect(k)
                .to_string()
        };
        assert_eq!(id("service_id"), "0x1234");
        assert_eq!(id("instance_id"), "0x0001");
        assert_eq!(id("method_id"), "0x0421");
        assert_eq!(id("event_group_id"), "0x0001");
        assert_eq!(id("event_id"), "0x8001");
        assert_eq!(id("getter_id"), "0x0100");
        assert_eq!(id("setter_id"), "0x0101");

        // No inline IDs in this fixture → no deprecation warnings.
        assert!(res.deprecation_warnings.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unresolved_service_produces_batched_error() {
        let dir = std::env::temp_dir().join("sce_ext_cfg_bad_svc");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = write_tmp(&dir, "vsomeip.json", SAMPLE_VSOMEIP);

        let mut deploy = sample_deploy(&cfg_path);
        // Rewrite the binding to reference a name that doesn't exist.
        {
            let b = deploy
                .topology
                .get_mut("ecu1")
                .unwrap()
                .machines
                .get_mut("brake")
                .unwrap()
                .bindings
                .get_mut("#motor")
                .unwrap();
            b.service = Some("ghost_service".into());
        }

        match resolve_external_bindings(&mut deploy, &dir) {
            Err(ExternalConfigError::UnresolvedNames { missing, .. }) => {
                // Service unresolved → downstream method/event_group also
                // unresolved (they depend on a resolved service). Both are
                // batched into a single error so the operator sees
                // everything at once.
                assert!(missing.iter().any(|m| m.kind == "service"));
                assert!(missing.iter().any(|m| m.kind == "method"));
            }
            other => panic!("expected UnresolvedNames, got {other:?}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ambiguous_event_group_rejected() {
        let dir = std::env::temp_dir().join("sce_ext_cfg_ambig");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = write_tmp(&dir, "vsomeip.json", SAMPLE_VSOMEIP);

        let mut deploy = sample_deploy(&cfg_path);
        deploy
            .topology
            .get_mut("ecu1")
            .unwrap()
            .machines
            .get_mut("brake")
            .unwrap()
            .bindings
            .get_mut("#motor")
            .unwrap()
            .event_group = Some("multi".into());

        match resolve_external_bindings(&mut deploy, &dir) {
            Err(ExternalConfigError::AmbiguousEventGroup {
                event_group, count, ..
            }) => {
                assert_eq!(event_group, "multi");
                assert_eq!(count, 2);
            }
            other => panic!("expected AmbiguousEventGroup, got {other:?}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_event_group_rejected() {
        let dir = std::env::temp_dir().join("sce_ext_cfg_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = write_tmp(&dir, "vsomeip.json", SAMPLE_VSOMEIP);

        let mut deploy = sample_deploy(&cfg_path);
        deploy
            .topology
            .get_mut("ecu1")
            .unwrap()
            .machines
            .get_mut("brake")
            .unwrap()
            .bindings
            .get_mut("#motor")
            .unwrap()
            .event_group = Some("empty".into());

        match resolve_external_bindings(&mut deploy, &dir) {
            Err(ExternalConfigError::EmptyEventGroup { event_group, .. }) => {
                assert_eq!(event_group, "empty");
            }
            other => panic!("expected EmptyEventGroup, got {other:?}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn named_reference_without_config_path_rejected() {
        // A binding with `service: motor_control` but no
        // `transports.someip.config:` at the device level → fail.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
"##;
        let mut deploy = crate::mesh::deploy::parse_deploy_str(yaml).expect("parse");
        match resolve_external_bindings(&mut deploy, Path::new(".")) {
            Err(ExternalConfigError::MissingConfigReference { device, target, .. }) => {
                assert_eq!(device, "ecu1");
                assert_eq!(target, "#motor");
            }
            other => panic!("expected MissingConfigReference, got {other:?}"),
        }
    }

    #[test]
    fn inline_ids_emit_deprecation_warnings() {
        // Legacy fixture shape — all IDs inline, no external config.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service_id: "0x1234"
            instance_id: "0x0001"
            method_id: "0x0421"
"##;
        let mut deploy = crate::mesh::deploy::parse_deploy_str(yaml).expect("parse");
        let res = resolve_external_bindings(&mut deploy, Path::new(".")).expect("resolve");
        // One warning per inline key
        assert_eq!(res.deprecation_warnings.len(), 3);
        let attrs: Vec<_> = res
            .deprecation_warnings
            .iter()
            .map(|w| w.attribute.as_str())
            .collect();
        assert!(attrs.contains(&"service_id:"));
        assert!(attrs.contains(&"instance_id:"));
        assert!(attrs.contains(&"method_id:"));
    }

    #[test]
    fn zenoh_binding_unaffected() {
        // External-config resolution must not touch zenoh bindings.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: zenoh
            key: "brake/cmd"
"##;
        let mut deploy = crate::mesh::deploy::parse_deploy_str(yaml).expect("parse");
        let res = resolve_external_bindings(&mut deploy, Path::new(".")).expect("resolve");
        assert!(res.deprecation_warnings.is_empty());
        let b = &deploy.topology["ecu1"].machines["brake"].bindings
            ["#motor"];
        // `key:` stays in extra unchanged.
        assert_eq!(b.extra.get("key").and_then(|v| v.as_str()), Some("brake/cmd"));
    }

    #[test]
    fn non_someip_binding_with_someip_name_rejected() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: zenoh
            service: motor_control
"##;
        let mut deploy = crate::mesh::deploy::parse_deploy_str(yaml).expect("parse");
        match resolve_external_bindings(&mut deploy, Path::new(".")) {
            Err(ExternalConfigError::MissingConfigReference { .. }) => {}
            other => panic!("expected MissingConfigReference, got {other:?}"),
        }
    }

    #[test]
    fn absolute_config_path_honored() {
        let dir = std::env::temp_dir().join("sce_ext_cfg_abs");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = write_tmp(&dir, "vsomeip.json", SAMPLE_VSOMEIP);
        assert!(cfg_path.is_absolute());

        let mut deploy = sample_deploy(&cfg_path);
        // deploy_dir is intentionally unrelated to cfg's parent — absolute
        // path must still resolve.
        let unrelated = std::env::temp_dir().join("sce_unrelated_dir");
        std::fs::create_dir_all(&unrelated).ok();
        resolve_external_bindings(&mut deploy, &unrelated).expect("absolute path resolves");

        std::fs::remove_dir_all(&dir).ok();
    }
}
