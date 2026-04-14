// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// External infrastructure config resolution (SCE_MESH.md §13).
//
// This stage runs between deploy.yaml parsing and topology analysis. Three
// inputs converge into one uniform per-event ID table:
//
//   1. Per-event `events:` block in deploy.yaml (spec canonical): each
//      SCXML event gets its own name-based references resolved against
//      vsomeip.json into a per-event `EventResolvedIds`.
//   2. Flat per-binding sugar (`method:` / `event_group:` / `getter:` /
//      `setter:` at binding level): one resolved set that topology later
//      fans out to every matching-pattern event on this target.
//   3. Stage 1 deprecated inline numeric IDs (`service_id: "0x1234"`):
//      tolerated with a `DeployDeprecationWarning`; topology fans them
//      out the same way as flat sugar.
//
// `service:` always resolves at binding level (service_id / instance_id
// are per-target, not per-event) and is injected into `binding.extra`.
// Unresolved names are batched into a single `UnresolvedNames` error per
// machine so operators see every mismatch at once.
//
// After this stage runs, `topology::attach_event_bindings` expands the
// flat/inline paths into per-event entries so the downstream codegen
// sees exactly one data structure (`ResolvedTarget.event_bindings`).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::diagnostics::DeployDeprecationWarning;
use crate::mesh::deploy::{BindingConfig, DeployConfig, EventBinding};
use crate::mesh::error::{ExternalConfigError, UnresolvedName};
use crate::mesh::target::TargetId;
use crate::mesh::topology::EventResolvedIds;
use crate::mesh::vsomeip_config::{Service, VsomeipConfig};

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
/// Each appears in a `DeployDeprecationWarning` when encountered.
const DEPRECATED_INLINE_IDS: &[&str] = &[
    KEY_SERVICE_ID,
    KEY_INSTANCE_ID,
    KEY_METHOD_ID,
    KEY_EVENT_GROUP_ID,
    KEY_EVENT_ID,
    KEY_GETTER_ID,
    KEY_SETTER_ID,
];

/// Per-binding resolution output.
///
/// `by_event` carries explicit per-event mappings from deploy.yaml's
/// `events:` block. `default` carries the binding-level flat sugar or
/// Stage 1 inline numeric IDs — an all-events-on-this-target fallback
/// that topology fans out to every SCXML event of matching pattern.
/// `default.is_empty()` means neither flat nor inline was set.
#[derive(Debug, Clone, Default)]
pub struct PerBindingResolution {
    pub by_event: BTreeMap<String, EventResolvedIds>,
    pub default: EventResolvedIds,
}

/// Result of the external-config resolution stage.
#[derive(Debug, Default)]
pub struct ExternalResolution {
    /// Deploy.yaml deprecation warnings for inline numeric IDs and other
    /// Stage 1 tolerations. Surfaced on `MeshResult` for CLI emission.
    pub deprecation_warnings: Vec<DeployDeprecationWarning>,
    /// Per-binding resolved IDs, keyed by `(machine_name, target_id)`.
    /// Consumed by `topology::attach_event_bindings` to produce per-event
    /// codegen context.
    pub bindings: HashMap<(String, TargetId), PerBindingResolution>,
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
    let mut result = ExternalResolution::default();

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
                device_name,
                machine_name,
                &machine.bindings,
                &mut result.deprecation_warnings,
            );

            for (target, binding) in machine.bindings.iter_mut() {
                if binding.transport != "someip" {
                    // Only SOME/IP currently uses name-based resolution.
                    // Zenoh bindings resolve their key expression at runtime;
                    // other transports have no external config concept. If a
                    // non-someip binding carries SOME/IP-only fields that is
                    // a deploy.yaml bug — catch it here.
                    reject_someip_fields_on_foreign_transport(
                        machine_name,
                        target,
                        binding,
                    )?;
                    continue;
                }

                // Spec schema conflict: flat sugar and events: block are
                // mutually exclusive. Caught here before any name resolution
                // runs so the diagnostic points at the schema mismatch, not
                // at a downstream symptom.
                reject_mixed_schema(machine_name, target, binding)?;

                // Pure-legacy binding (only inline extra IDs, no name-based
                // refs) has no resolution work; the default EventResolvedIds
                // is built by topology from binding.extra.
                if !binding_uses_named_references(binding) && binding.events.is_empty() {
                    continue;
                }

                // Resolution needed → device must declare transports.someip.config.
                let (config_path, someip) = someip_cache.get(device_name).ok_or_else(|| {
                    ExternalConfigError::NamedReferenceWithoutConfig {
                        machine: machine_name.clone(),
                        device: device_name.clone(),
                        target: target.as_str().to_string(),
                    }
                })?;

                let per_binding = resolve_binding(
                    machine_name,
                    target,
                    binding,
                    someip,
                    config_path.as_path(),
                    &mut missing,
                )?;
                result
                    .bindings
                    .insert((machine_name.clone(), target.clone()), per_binding);
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

    Ok(result)
}

/// True iff any name-based SOME/IP reference is set on this binding —
/// either at binding level (flat sugar) or inside a per-event entry.
fn binding_uses_named_references(binding: &BindingConfig) -> bool {
    binding.service.is_some()
        || binding.has_flat_event_fields()
        || binding.events.values().any(|e| !e.is_empty())
}

/// Reject deploy.yaml that mixes the flat sugar fields with a per-event
/// `events:` block on the same binding. There is no defined precedence,
/// and silently picking one would surprise the reader on the other.
fn reject_mixed_schema(
    machine: &str,
    target: &TargetId,
    binding: &BindingConfig,
) -> Result<(), ExternalConfigError> {
    if binding.events.is_empty() || !binding.has_flat_event_fields() {
        return Ok(());
    }
    let mut flat: Vec<&'static str> = Vec::new();
    if binding.method.is_some() {
        flat.push("method");
    }
    if binding.event_group.is_some() {
        flat.push("event_group");
    }
    if binding.getter.is_some() {
        flat.push("getter");
    }
    if binding.setter.is_some() {
        flat.push("setter");
    }
    Err(ExternalConfigError::ConflictingEventSchema {
        machine: machine.to_string(),
        target: target.as_str().to_string(),
        flat_fields: flat,
    })
}

/// Non-SOME/IP bindings must not carry SOME/IP-only name-based fields.
/// The correction is transport-specific (either switch to SOME/IP or drop
/// the fields), which is semantically different from "declare a config
/// path" — hence its own error variant.
fn reject_someip_fields_on_foreign_transport(
    machine: &str,
    target: &TargetId,
    binding: &BindingConfig,
) -> Result<(), ExternalConfigError> {
    if !binding_uses_named_references(binding) {
        return Ok(());
    }
    let mut fields: Vec<&'static str> = Vec::new();
    if binding.service.is_some() {
        fields.push("service");
    }
    if binding.method.is_some() {
        fields.push("method");
    }
    if binding.event_group.is_some() {
        fields.push("event_group");
    }
    if binding.getter.is_some() {
        fields.push("getter");
    }
    if binding.setter.is_some() {
        fields.push("setter");
    }
    Err(ExternalConfigError::SomeipFieldOnNonSomeipTransport {
        machine: machine.to_string(),
        target: target.as_str().to_string(),
        transport: binding.transport.clone(),
        fields,
    })
}

/// Format the deploy.yaml path of a specific binding field for diagnostics.
fn binding_path(device: &str, machine: &str, target: &TargetId, field: &str) -> String {
    format!(
        "topology.{device}.machines.{machine}.bindings[{target}].{field}",
        target = target.as_str()
    )
}

/// Record a deploy-layer deprecation warning for every Stage 1 inline
/// numeric ID present on any binding of this machine.
///
/// Warnings are emitted one per (binding, field) occurrence so a reader
/// can locate each site exactly. Deduplication across bindings is not
/// needed — a deploy.yaml with N inline IDs genuinely has N migration
/// targets.
fn collect_inline_id_deprecations(
    device: &str,
    machine: &str,
    bindings: &HashMap<TargetId, BindingConfig>,
    out: &mut Vec<DeployDeprecationWarning>,
) {
    for (target, binding) in bindings {
        for &key in DEPRECATED_INLINE_IDS {
            if binding.extra.contains_key(key) {
                out.push(DeployDeprecationWarning {
                    field: key.to_string(),
                    location: binding_path(device, machine, target, key),
                    reason: "inline SOME/IP numeric IDs are deprecated \
                             (SCE_MESH.md §13). Replace with a name-based \
                             reference (e.g. `service:`, `method:`) and \
                             declare `transports.someip.config:` at the \
                             device level."
                        .to_string(),
                });
            }
        }
    }
}

/// Resolve a single binding into a `PerBindingResolution`.
///
/// Three distinct sub-paths share `resolve_event_binding` for per-event
/// detail and the `service:` resolution at the top:
///   1. `service:` (always at binding level) → `service_id` + `instance_id`
///      injected into `binding.extra` for the legacy template path.
///   2. Per-event `events:` block → one `EventResolvedIds` per entry.
///   3. Flat sugar (`method:` / `event_group:` / ...) → a single default
///      `EventResolvedIds` that topology fans out to matching events.
///
/// Unresolved names go into `missing` for batched reporting.
fn resolve_binding(
    machine: &str,
    target: &TargetId,
    binding: &mut BindingConfig,
    someip: &VsomeipConfig,
    config_path: &Path,
    missing: &mut Vec<UnresolvedName>,
) -> Result<PerBindingResolution, ExternalConfigError> {
    // Service → service_id + instance_id. Without a service there is
    // nothing for method/event_group/getter/setter to hang off, so the
    // per-event resolutions short-circuit when service is unset or
    // unresolved (still recording the unresolved-method as missing for
    // batched reporting).
    let service_ref = resolve_service(binding, someip, missing);

    let mut out = PerBindingResolution::default();

    if !binding.events.is_empty() {
        // Per-event path (spec canonical).
        for (event_name, event_binding) in &binding.events {
            let ids = resolve_event_binding(
                machine,
                target,
                config_path,
                event_name,
                event_binding,
                service_ref,
                missing,
            )?;
            // Skip empty entries — an `events.foo: {}` with no fields is
            // user error but harmless; topology will report it as unused
            // if the event is sent without any pattern requirement.
            if !ids.is_empty() {
                out.by_event.insert(event_name.clone(), ids);
            }
        }
    }

    if binding.has_flat_event_fields() {
        // Flat sugar path. The flat fields collapse into a single
        // EventBinding so resolution shares one code path with the
        // events: block. The schema-conflict gate above guarantees
        // events: is empty here.
        let synthetic = EventBinding {
            method: binding.method.clone(),
            event_group: binding.event_group.clone(),
            getter: binding.getter.clone(),
            setter: binding.setter.clone(),
        };
        out.default = resolve_event_binding(
            machine,
            target,
            config_path,
            "<all events>",
            &synthetic,
            service_ref,
            missing,
        )?;
    }

    Ok(out)
}

/// Resolve `binding.service` against vsomeip.json, injecting
/// `service_id`/`instance_id` into `binding.extra` so the legacy template
/// path stays unchanged. Returns the resolved `Service` for downstream
/// per-event lookups.
fn resolve_service<'a>(
    binding: &mut BindingConfig,
    someip: &'a VsomeipConfig,
    missing: &mut Vec<UnresolvedName>,
) -> Option<&'a Service> {
    let name = binding.service.as_deref()?;
    match someip.resolve_service(name) {
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
    }
}

/// Resolve a single `EventBinding` (per-event entry or flat-sugar synthetic)
/// against vsomeip.json. `event_name` and `target` are used in diagnostics
/// so the operator sees which SCXML event the failure originates from.
fn resolve_event_binding(
    machine: &str,
    target: &TargetId,
    config_path: &Path,
    event_name: &str,
    event_binding: &EventBinding,
    service_ref: Option<&Service>,
    missing: &mut Vec<UnresolvedName>,
) -> Result<EventResolvedIds, ExternalConfigError> {
    let mut out = EventResolvedIds::default();
    let svc_ctx = || {
        service_ref.map(|s| format!("in service \"{}\" (event \"{event_name}\")", s.name))
    };

    if let Some(name) = event_binding.method.as_deref() {
        match service_ref.and_then(|svc| svc.methods.iter().find(|m| m.name == name)) {
            Some(m) => out.method_id = Some(m.method),
            None => missing.push(UnresolvedName {
                kind: "method",
                name: name.to_string(),
                context: svc_ctx(),
            }),
        }
    }
    if let Some(name) = event_binding.getter.as_deref() {
        match service_ref.and_then(|svc| svc.methods.iter().find(|m| m.name == name)) {
            Some(m) => out.getter_id = Some(m.method),
            None => missing.push(UnresolvedName {
                kind: "getter",
                name: name.to_string(),
                context: svc_ctx(),
            }),
        }
    }
    if let Some(name) = event_binding.setter.as_deref() {
        match service_ref.and_then(|svc| svc.methods.iter().find(|m| m.name == name)) {
            Some(m) => out.setter_id = Some(m.method),
            None => missing.push(UnresolvedName {
                kind: "setter",
                name: name.to_string(),
                context: svc_ctx(),
            }),
        }
    }
    if let Some(name) = event_binding.event_group.as_deref() {
        match service_ref.and_then(|svc| svc.eventgroups.iter().find(|e| e.name == name)) {
            Some(eg) => {
                out.event_group_id = Some(eg.eventgroup);
                // Event group must contain exactly one event for the
                // current one-event-per-mapping template. Multi-event and
                // zero-event cases are diagnosed with dedicated errors —
                // silently picking events[0] or 0 would route nothing.
                match eg.events.len() {
                    0 => {
                        return Err(ExternalConfigError::EmptyEventGroup {
                            machine: machine.to_string(),
                            target: target.as_str().to_string(),
                            config_path: config_path.display().to_string(),
                            event_group: name.to_string(),
                        });
                    }
                    1 => out.event_id = Some(eg.events[0]),
                    n => {
                        return Err(ExternalConfigError::AmbiguousEventGroup {
                            machine: machine.to_string(),
                            target: target.as_str().to_string(),
                            config_path: config_path.display().to_string(),
                            event_group: name.to_string(),
                            count: n,
                        });
                    }
                }
            }
            None => missing.push(UnresolvedName {
                kind: "event_group",
                name: name.to_string(),
                context: svc_ctx(),
            }),
        }
    }
    Ok(out)
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

        // Service IDs go into binding.extra (binding-level, per-target).
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

        // Per-event IDs live on the resolution map's `default` slot
        // (sample_deploy uses flat sugar fields, not an events: block).
        let key = ("brake".to_string(), TargetId::new("#motor").unwrap());
        let per_binding = res.bindings.get(&key).expect("binding resolution");
        assert!(per_binding.by_event.is_empty(), "flat sugar → no by_event entries");
        let d = &per_binding.default;
        assert_eq!(d.method_id, Some(0x0421));
        assert_eq!(d.event_group_id, Some(0x0001));
        assert_eq!(d.event_id, Some(0x8001));
        assert_eq!(d.getter_id, Some(0x0100));
        assert_eq!(d.setter_id, Some(0x0101));

        // No inline IDs in this fixture → no deprecation warnings.
        assert!(res.deprecation_warnings.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn per_event_block_resolves_into_by_event_map() {
        // Per-event SCE_MESH.md §14 schema: distinct method per event.
        let dir = std::env::temp_dir().join("sce_ext_cfg_per_event");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = write_tmp(&dir, "vsomeip.json", SAMPLE_VSOMEIP);

        let yaml = format!(
            r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: {config}
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            events:
              "service.request.compute_force":
                method: compute_force
              "field.get.status":
                getter: get_status
              "field.set.mode":
                setter: set_mode
"##,
            config = cfg_path.display()
        );
        let mut deploy = crate::mesh::deploy::parse_deploy_str(&yaml).expect("parse");
        let res = resolve_external_bindings(&mut deploy, &dir).expect("resolve");

        let key = ("brake".to_string(), TargetId::new("#motor").unwrap());
        let per_binding = res.bindings.get(&key).expect("resolution");

        // Each event maps to its own resolved IDs — no fan-out, no defaults.
        assert!(per_binding.default.is_empty(), "no flat sugar declared");
        assert_eq!(
            per_binding.by_event["service.request.compute_force"].method_id,
            Some(0x0421)
        );
        assert_eq!(
            per_binding.by_event["field.get.status"].getter_id,
            Some(0x0100)
        );
        assert_eq!(
            per_binding.by_event["field.set.mode"].setter_id,
            Some(0x0101)
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn flat_sugar_and_events_block_conflict_rejected() {
        let dir = std::env::temp_dir().join("sce_ext_cfg_conflict");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = write_tmp(&dir, "vsomeip.json", SAMPLE_VSOMEIP);

        let yaml = format!(
            r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: {config}
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            method: compute_force
            events:
              "service.request.compute_force":
                method: compute_force
"##,
            config = cfg_path.display()
        );
        let mut deploy = crate::mesh::deploy::parse_deploy_str(&yaml).expect("parse");
        match resolve_external_bindings(&mut deploy, &dir) {
            Err(ExternalConfigError::ConflictingEventSchema { flat_fields, .. }) => {
                assert!(flat_fields.contains(&"method"));
            }
            other => panic!("expected ConflictingEventSchema, got {other:?}"),
        }

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
            Err(ExternalConfigError::NamedReferenceWithoutConfig { device, target, .. }) => {
                assert_eq!(device, "ecu1");
                assert_eq!(target, "#motor");
            }
            other => panic!("expected NamedReferenceWithoutConfig, got {other:?}"),
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
        let fields: Vec<_> = res
            .deprecation_warnings
            .iter()
            .map(|w| w.field.as_str())
            .collect();
        assert!(fields.contains(&"service_id"));
        assert!(fields.contains(&"instance_id"));
        assert!(fields.contains(&"method_id"));
        // Location string includes the full YAML path for operator diagnostics.
        let locations: Vec<_> = res
            .deprecation_warnings
            .iter()
            .map(|w| w.location.clone())
            .collect();
        assert!(
            locations.iter().all(|l| l.contains("topology.ecu1.machines.brake.bindings[#motor]")),
            "every warning should carry the YAML location: {locations:?}"
        );
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
            method: compute_force
"##;
        let mut deploy = crate::mesh::deploy::parse_deploy_str(yaml).expect("parse");
        match resolve_external_bindings(&mut deploy, Path::new(".")) {
            Err(ExternalConfigError::SomeipFieldOnNonSomeipTransport {
                transport,
                fields,
                ..
            }) => {
                assert_eq!(transport, "zenoh");
                assert!(fields.contains(&"service"));
                assert!(fields.contains(&"method"));
            }
            other => panic!("expected SomeipFieldOnNonSomeipTransport, got {other:?}"),
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
