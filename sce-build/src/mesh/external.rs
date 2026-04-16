// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// External infrastructure config resolution (SCE_MESH.md §13, §14).
//
// Runs between deploy.yaml parsing and topology analysis. Two inputs
// converge into one uniform per-event ID table:
//
//   1. Per-event `events:` block in deploy.yaml (spec canonical): each
//      SCXML event gets its own name-based references resolved against
//      vsomeip.json into a tagged `SomeipEventIds`.
//   2. Flat per-binding sugar (`method:` / `event_group:` / `getter:` /
//      `setter:` at binding level): one resolved `BindingDefaultIds`
//      that topology later fans out to every matching-pattern event.
//
// `service:` always resolves at binding level (`service_id` / `instance_id`
// are per-target, not per-event) and lands in
// `PerBindingResolution.service_ids`. Unresolved names are batched into a
// single `UnresolvedNames` error per machine so operators see every
// mismatch at once.
//
// deploy.yaml never declares SOME/IP numeric IDs directly — the key names
// `service_id`, `method_id`, etc. are reserved and rejected here.
// Numeric IDs come from vsomeip.json, referenced by name.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::mesh::deploy::{BindingConfig, DeployConfig, EventBinding};
use crate::mesh::error::{ExternalConfigError, UnresolvedName};
use crate::mesh::target::TargetId;
use crate::mesh::topology::{BindingDefaultIds, SomeipEventIds, SomeipServiceIds};
use crate::mesh::vsomeip_config::{Service, VsomeipConfig};

/// deploy.yaml key names reserved for SOME/IP numeric IDs. These are
/// never user-facing — the IDs come from vsomeip.json, referenced by
/// name. Any occurrence in `BindingConfig.extra` is rejected as
/// [`ExternalConfigError::ReservedSomeipIdKeys`]. Kept as a
/// grep-locatable list so rejection site and diagnostic stay in sync.
const RESERVED_SOMEIP_ID_KEYS: &[&str] = &[
    "service_id",
    "instance_id",
    "method_id",
    "event_group_id",
    "event_id",
    "getter_id",
    "setter_id",
];

/// Per-binding resolution output.
///
/// `by_event` carries explicit per-event mappings from deploy.yaml's
/// `events:` block — a tagged [`SomeipEventIds`] per entry because each
/// event addresses exactly one SOME/IP resource kind. `default` carries
/// the binding-level flat sugar, which is inherently multi-kind (one
/// binding may declare `method:` AND `event_group:` simultaneously as
/// defaults) — it stays a loose [`BindingDefaultIds`] and is projected
/// per-event by `finalize_targets`. `default.is_empty()` means no flat
/// sugar was declared.
///
/// `service_ids` is non-optional: a binding for which `service:` does not
/// resolve never lands in [`ExternalResolution::bindings`] in the first
/// place (the failure is recorded as [`UnresolvedName`] and the build
/// fails with [`ExternalConfigError::UnresolvedNames`]). So topology can
/// read `service_ids` directly without a probe.
#[derive(Debug, Clone)]
pub struct PerBindingResolution {
    pub service_ids: SomeipServiceIds,
    pub by_event: BTreeMap<String, SomeipEventIds>,
    pub default: BindingDefaultIds,
}

/// Result of the external-config resolution stage.
#[derive(Debug, Default)]
pub struct ExternalResolution {
    /// Per-binding resolved IDs, keyed by `(machine_name, target_id)`.
    /// Consumed by `topology::finalize_targets` to produce per-event
    /// codegen context.
    pub bindings: HashMap<(String, TargetId), PerBindingResolution>,
    /// Server-side resolved IDs, keyed by machine name.
    /// Consumed by `topology::resolve_server_binding` to produce
    /// server-side transport state for codegen.
    pub server_bindings: HashMap<String, PerBindingResolution>,
}

/// Resolve all name-based external references in `deploy_cfg`.
///
/// `deploy_dir` is the directory containing deploy.yaml; external config
/// paths are resolved relative to it (absolute paths are honored as-is).
///
/// Produces an [`ExternalResolution`] that topology consumes — `deploy_cfg`
/// itself is read-only (resolved IDs live on the returned map, not mutated
/// back into binding.extra).
pub fn resolve_external_bindings(
    deploy_cfg: &DeployConfig,
    deploy_dir: &Path,
) -> Result<ExternalResolution, ExternalConfigError> {
    let mut result = ExternalResolution::default();

    // Cache parsed vsomeip.json per device — the same file can be referenced
    // by many bindings and there's no value in parsing it more than once.
    let mut someip_cache: HashMap<String, (PathBuf, VsomeipConfig)> = HashMap::new();

    // HashMap iteration is non-deterministic within a device, which is fine
    // here because we batch all unresolved names before erroring.
    for (device_name, device) in deploy_cfg.topology.iter() {
        // Parse vsomeip.json once if referenced.
        let someip_config_path =
            device.transports.someip.as_ref().and_then(|s| s.config.clone());

        if let Some(rel_path) = someip_config_path.as_ref() {
            let abs_path = resolve_relative(deploy_dir, rel_path);
            let cfg = VsomeipConfig::load(&abs_path)?;
            someip_cache.insert(device_name.clone(), (abs_path, cfg));
        }

        // Resolve each machine's bindings against the device's external config.
        for (machine_name, machine) in device.machines.iter() {
            let mut missing: Vec<UnresolvedName> = Vec::new();

            for (target, binding) in machine.bindings.iter() {
                // Reject the reserved SOME/IP numeric-ID key names before
                // any other validation so the diagnostic names the exact
                // keys instead of surfacing as "service: missing" downstream.
                reject_reserved_id_keys(machine_name, target, binding)?;

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

                // SOME/IP targets must reference the OEM vsomeip.json. A
                // binding without `service:` or `events:` has nothing to
                // resolve and leaves topology with no service identity —
                // catch it here with the precise diagnostic instead of a
                // downstream MissingBindingField.
                if !binding_uses_named_references(binding) && binding.events.is_empty() {
                    return Err(ExternalConfigError::NamedReferenceWithoutConfig {
                        machine: machine_name.clone(),
                        device: device_name.clone(),
                        target: target.as_str().to_string(),
                    });
                }

                // Resolution needed → device must declare transports.someip.config.
                let (config_path, someip) = someip_cache.get(device_name).ok_or_else(|| {
                    ExternalConfigError::NamedReferenceWithoutConfig {
                        machine: machine_name.clone(),
                        device: device_name.clone(),
                        target: target.as_str().to_string(),
                    }
                })?;

                // `resolve_binding` only emits an entry when `service:`
                // actually resolves; an unresolved service pushes a
                // `UnresolvedName` into `missing` and the entire machine
                // fails below with a batched diagnostic.
                if let Some(per_binding) = resolve_binding(
                    machine_name,
                    target,
                    binding,
                    someip,
                    config_path.as_path(),
                    &mut missing,
                )? {
                    result
                        .bindings
                        .insert((machine_name.clone(), target.clone()), per_binding);
                }
            }

            // Server-side SOME/IP resolution (SCE_MESH.md §13 Session E).
            // Same resolution pipeline as client bindings — service name +
            // per-event method names resolve against vsomeip.json.
            if let Some(ref server_cfg) = machine.server {
                if server_cfg.transport == "someip" {
                    if let Some((config_path, someip)) = someip_cache.get(device_name) {
                        if let Some(per_binding) = resolve_server_config(
                            machine_name,
                            server_cfg,
                            someip,
                            config_path.as_path(),
                            &mut missing,
                        )? {
                            result
                                .server_bindings
                                .insert(machine_name.clone(), per_binding);
                        }
                    } else if server_cfg.service.is_some() {
                        return Err(ExternalConfigError::NamedReferenceWithoutConfig {
                            machine: machine_name.clone(),
                            device: device_name.clone(),
                            target: "#_server".to_string(),
                        });
                    }
                }
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

/// Reject the reserved SOME/IP numeric-ID key names on a binding. The
/// SOME/IP identity goes through name-based references against
/// vsomeip.json (SCE_MESH.md §14); numeric keys in deploy.yaml have no
/// defined meaning. Lists every offending key in one error so the
/// operator fixes them all in one pass.
fn reject_reserved_id_keys(
    machine: &str,
    target: &TargetId,
    binding: &BindingConfig,
) -> Result<(), ExternalConfigError> {
    let offenders: Vec<&'static str> = RESERVED_SOMEIP_ID_KEYS
        .iter()
        .copied()
        .filter(|k| binding.extra.contains_key(*k))
        .collect();
    if offenders.is_empty() {
        return Ok(());
    }
    Err(ExternalConfigError::ReservedSomeipIdKeys {
        machine: machine.to_string(),
        target: target.as_str().to_string(),
        transport: binding.transport.clone(),
        fields: offenders,
    })
}

/// Resolve a single binding into a `PerBindingResolution`.
///
/// Two sub-paths converge:
///   1. `service:` (binding level) → `SomeipServiceIds` on the result.
///   2. Per-event `events:` block → one tagged [`SomeipEventIds`] per entry,
///      OR flat sugar (`method:` / `event_group:` / ...) → one
///      [`BindingDefaultIds`] that topology fans out to matching events.
///
/// Unresolved names go into `missing` for batched reporting. If `service:`
/// itself does not resolve the function returns `Ok(None)`: without a
/// service identity there is nothing meaningful for topology to consume,
/// and the unresolved-service record in `missing` will surface as a
/// [`ExternalConfigError::UnresolvedNames`] at the end of the outer loop.
/// Event-level names are still pushed into `missing` in this failure case
/// so the operator sees every mismatch in one error instead of fixing
/// them round by round.
fn resolve_binding(
    machine: &str,
    target: &TargetId,
    binding: &BindingConfig,
    someip: &VsomeipConfig,
    config_path: &Path,
    missing: &mut Vec<UnresolvedName>,
) -> Result<Option<PerBindingResolution>, ExternalConfigError> {
    // Service → service_id + instance_id. Without a service there is
    // nothing for method/event_group/getter/setter to hang off; per-event
    // resolutions still run so every unresolved name is batched into
    // `missing` for one consolidated error.
    let service_ref = resolve_service(binding, someip, missing);

    let mut by_event: BTreeMap<String, SomeipEventIds> = BTreeMap::new();
    let mut default = BindingDefaultIds::default();

    if !binding.events.is_empty() {
        // Per-event path (spec canonical). Each entry is expected to set
        // exactly one field family — the tagged `SomeipEventIds` enum
        // makes the invariant explicit.
        for (event_name, event_binding) in &binding.events {
            if let Some(ids) = resolve_event_binding_to_tag(
                machine,
                target,
                config_path,
                event_name,
                event_binding,
                service_ref,
                missing,
            )? {
                by_event.insert(event_name.clone(), ids);
            }
            // An entry with no field set (`events.foo: {}`) is user error
            // but harmless: nothing lands in `by_event`; topology will
            // report the event as unused if it is never `<send>`-ed.
        }
    }

    if binding.has_flat_event_fields() {
        // Flat sugar path: binding-level defaults may legitimately carry
        // multiple field kinds at once (e.g. `method: foo` AND
        // `event_group: bar` — defaults for the two pattern families
        // co-present on the target). The schema-conflict gate above
        // guarantees `events:` is empty here.
        let synthetic = EventBinding {
            method: binding.method.clone(),
            event_group: binding.event_group.clone(),
            getter: binding.getter.clone(),
            setter: binding.setter.clone(),
        };
        default = resolve_event_binding_to_default(
            machine,
            target,
            config_path,
            "<all events>",
            &synthetic,
            service_ref,
            missing,
        )?;
    }

    // Only emit a resolution entry when the service identity is real —
    // `PerBindingResolution.service_ids` is a non-Option so the type
    // expresses the invariant "service is resolved if we have a record".
    Ok(service_ref.map(|svc| PerBindingResolution {
        service_ids: SomeipServiceIds {
            service_id: svc.service,
            instance_id: svc.instance,
        },
        by_event,
        default,
    }))
}

/// Resolve `binding.service` against vsomeip.json and return the matching
/// `Service`. Downstream consumers read the typed `service_ids` field on
/// `PerBindingResolution`; `binding.extra` is never mutated.
fn resolve_service<'a>(
    binding: &BindingConfig,
    someip: &'a VsomeipConfig,
    missing: &mut Vec<UnresolvedName>,
) -> Option<&'a Service> {
    let name = binding.service.as_deref()?;
    match someip.resolve_service(name) {
        Some(svc) => Some(svc),
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

/// Resolve a per-event `events:` entry into a tagged [`SomeipEventIds`].
/// Exactly one of the EventBinding's fields (`method` / `event_group` /
/// `getter` / `setter`) must be set; zero fields returns `None` (the
/// entry contributes no mapping), multiple set fields is a user error
/// diagnosed as [`ExternalConfigError::ConflictingEventFieldKinds`].
fn resolve_event_binding_to_tag(
    machine: &str,
    target: &TargetId,
    config_path: &Path,
    event_name: &str,
    event_binding: &EventBinding,
    service_ref: Option<&Service>,
    missing: &mut Vec<UnresolvedName>,
) -> Result<Option<SomeipEventIds>, ExternalConfigError> {
    // Enforce single-field-family invariant at resolution time. Each
    // per-event entry maps to exactly one SOME/IP resource kind —
    // otherwise `SomeipEventIds` cannot be constructed unambiguously.
    let mut set_fields: Vec<&'static str> = Vec::new();
    if event_binding.method.is_some()      { set_fields.push("method"); }
    if event_binding.event_group.is_some() { set_fields.push("event_group"); }
    if event_binding.getter.is_some()      { set_fields.push("getter"); }
    if event_binding.setter.is_some()      { set_fields.push("setter"); }

    // Exactly one field family must be set. Both "zero fields" and
    // "multiple fields" are user errors caught here, so the match below
    // does not need a fall-through branch and topology's invariant
    // "every someip binding that lands in ExternalResolution has a
    // resolvable per-event map" stays intact.
    match set_fields.len() {
        0 => {
            return Err(ExternalConfigError::EmptyEventEntry {
                machine: machine.to_string(),
                target: target.as_str().to_string(),
                event: event_name.to_string(),
            });
        }
        1 => {}
        _ => {
            return Err(ExternalConfigError::ConflictingEventFieldKinds {
                machine: machine.to_string(),
                target: target.as_str().to_string(),
                event: event_name.to_string(),
                fields: set_fields.iter().map(|s| s.to_string()).collect(),
            });
        }
    }
    let kind = set_fields[0];

    let defaults = resolve_event_binding_to_default(
        machine, target, config_path, event_name, event_binding, service_ref, missing,
    )?;

    // Promote the single populated family into the tagged enum. The kind
    // discriminator came from the user's own deploy.yaml declaration —
    // `defaults` is the name-resolved view of that same declaration, so the
    // matching slot must be `Some(_)` unless upstream silently dropped a
    // partial resolution (would be a `resolve_event_binding_to_default` bug).
    // Any divergence is `unreachable!` rather than a silent `None` so the
    // bug surfaces immediately at build time instead of leaking into codegen
    // as "event entry mysteriously missing".
    //
    // Name lookups that simply failed (event group not found, method not
    // found) record the failure in `missing` and leave the slot empty; that
    // case is represented here by returning `Ok(None)` for the matched kind,
    // which topology combines with the batched `UnresolvedNames` error.
    let ids = match kind {
        "method" => defaults.method_id.map(|method_id| SomeipEventIds::Method { method_id }),
        "event_group" => match (defaults.event_group_id, defaults.event_id) {
            (Some(event_group_id), Some(event_id)) => {
                Some(SomeipEventIds::EventGroup { event_group_id, event_id })
            }
            (None, None) => None,
            // Partial resolution — event_group_id came back but event_id
            // did not, or vice-versa. The only way this could happen is a
            // resolve_event_binding_to_default bug (every successful
            // event_group lookup emits both or neither). Surface it loudly.
            partial => unreachable!(
                "event_group/event_id partial resolution for '{event_name}' on '{}': {:?}",
                target.as_str(),
                partial,
            ),
        },
        "getter" => defaults.getter_id.map(|getter_id| SomeipEventIds::Getter { getter_id }),
        "setter" => defaults.setter_id.map(|setter_id| SomeipEventIds::Setter { setter_id }),
        other => unreachable!(
            "ConflictingEventFieldKinds gate allowed unknown field kind {other:?}"
        ),
    };
    Ok(ids)
}

/// Resolve an `EventBinding` into a multi-field default record. Used for
/// the flat-sugar path at binding level (where multiple field families
/// may co-exist as defaults) and as the resolution primitive feeding
/// `resolve_event_binding_to_tag`.
fn resolve_event_binding_to_default(
    machine: &str,
    target: &TargetId,
    config_path: &Path,
    event_name: &str,
    event_binding: &EventBinding,
    service_ref: Option<&Service>,
    missing: &mut Vec<UnresolvedName>,
) -> Result<BindingDefaultIds, ExternalConfigError> {
    let mut out = BindingDefaultIds::default();
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

/// Join `base` with `rel`, honoring absolute paths.
fn resolve_relative(base: &Path, rel: &Path) -> PathBuf {
    if rel.is_absolute() {
        rel.to_path_buf()
    } else {
        base.join(rel)
    }
}

// ── Server-side resolution (SCE_MESH.md §13 Session E) ──────

/// Resolve a deploy.yaml `server:` section for a machine.
///
/// Same pipeline as client binding resolution: `service:` resolves to
/// `service_id` + `instance_id`, per-event method names resolve to IDs.
fn resolve_server_config(
    machine: &str,
    server_cfg: &crate::mesh::deploy::ServerConfig,
    someip: &VsomeipConfig,
    config_path: &Path,
    missing: &mut Vec<UnresolvedName>,
) -> Result<Option<PerBindingResolution>, ExternalConfigError> {
    // Resolve service name → service_id + instance_id
    let service_name = match server_cfg.service.as_deref() {
        Some(name) => name,
        None => return Ok(None),
    };
    let service_ref = someip.resolve_service(service_name);
    if service_ref.is_none() {
        missing.push(UnresolvedName {
            kind: "service",
            name: service_name.to_string(),
            context: Some(format!("server section of machine \"{machine}\"")),
        });
    }

    // Resolve per-event method names
    let server_target = TargetId::new("#_server").expect("static target ID");
    let mut by_event: BTreeMap<String, SomeipEventIds> = BTreeMap::new();
    for (event_name, event_binding) in &server_cfg.events {
        if let Some(ids) = resolve_event_binding_to_tag(
            machine,
            &server_target,
            config_path,
            event_name,
            event_binding,
            service_ref,
            missing,
        )? {
            by_event.insert(event_name.clone(), ids);
        }
    }

    Ok(service_ref.map(|svc| PerBindingResolution {
        service_ids: SomeipServiceIds {
            service_id: svc.service,
            instance_id: svc.instance,
        },
        by_event,
        default: BindingDefaultIds::default(),
    }))
}

impl ExternalResolution {
    /// Look up server-side service IDs for a SOME/IP server machine.
    pub fn resolve_server_service(
        &self,
        machine_name: &str,
        _service_name: &str,
    ) -> Result<SomeipServiceIds, ExternalConfigError> {
        self.server_bindings
            .get(machine_name)
            .map(|r| r.service_ids)
            .ok_or_else(|| ExternalConfigError::NamedReferenceWithoutConfig {
                machine: machine_name.to_string(),
                device: String::new(),
                target: "#_server".to_string(),
            })
    }

    /// Look up a server-side method ID for a specific event.
    pub fn resolve_server_method(
        &self,
        machine_name: &str,
        _service_name: &str,
        _method_name: &str,
    ) -> Result<u16, ExternalConfigError> {
        let resolution = self
            .server_bindings
            .get(machine_name)
            .ok_or_else(|| ExternalConfigError::NamedReferenceWithoutConfig {
                machine: machine_name.to_string(),
                device: String::new(),
                target: "#_server".to_string(),
            })?;
        // Find the method_id from the per-event resolution (any event
        // that resolved to a Method variant). The caller's method_name
        // is the same one that was resolved during resolve_server_config.
        for ids in resolution.by_event.values() {
            if let SomeipEventIds::Method { method_id } = ids {
                return Ok(*method_id);
            }
        }
        Err(ExternalConfigError::NamedReferenceWithoutConfig {
            machine: machine_name.to_string(),
            device: String::new(),
            target: "#_server".to_string(),
        })
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

        // Service identity goes onto the typed `service_ids` field —
        // resolution no longer mutates `binding.extra` for these.
        let binding = &deploy.topology["ecu1"].machines["brake"].bindings["#motor"];
        assert!(!binding.extra.contains_key("service_id"));
        assert!(!binding.extra.contains_key("instance_id"));

        let key = ("brake".to_string(), TargetId::new("#motor").unwrap());
        let per_binding = res.bindings.get(&key).expect("binding resolution");
        assert_eq!(
            per_binding.service_ids,
            SomeipServiceIds { service_id: 0x1234, instance_id: 0x0001 },
        );

        // Per-event IDs live on the resolution map's `default` slot
        // (sample_deploy uses flat sugar fields, not an events: block).
        assert!(per_binding.by_event.is_empty(), "flat sugar → no by_event entries");
        let d = &per_binding.default;
        assert_eq!(d.method_id, Some(0x0421));
        assert_eq!(d.event_group_id, Some(0x0001));
        assert_eq!(d.event_id, Some(0x8001));
        assert_eq!(d.getter_id, Some(0x0100));
        assert_eq!(d.setter_id, Some(0x0101));

        std::fs::remove_dir_all(&dir).ok();
        let _ = res;
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
            per_binding.by_event["service.request.compute_force"],
            SomeipEventIds::Method { method_id: 0x0421 }
        );
        assert_eq!(
            per_binding.by_event["field.get.status"],
            SomeipEventIds::Getter { getter_id: 0x0100 }
        );
        assert_eq!(
            per_binding.by_event["field.set.mode"],
            SomeipEventIds::Setter { setter_id: 0x0101 }
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
    fn reserved_someip_id_keys_rejected() {
        // deploy.yaml does not declare SOME/IP numeric IDs directly — the
        // names are reserved and the resolution layer rejects them with a
        // diagnostic naming every offending key so the operator fixes
        // them in one edit.
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
        let deploy = crate::mesh::deploy::parse_deploy_str(yaml).expect("parse");
        match resolve_external_bindings(&deploy, Path::new(".")) {
            Err(ExternalConfigError::ReservedSomeipIdKeys { fields, target, .. }) => {
                assert_eq!(target, "#motor");
                assert!(fields.contains(&"service_id"));
                assert!(fields.contains(&"instance_id"));
                assert!(fields.contains(&"method_id"));
            }
            other => panic!("expected ReservedSomeipIdKeys, got {other:?}"),
        }
    }

    #[test]
    fn zenoh_binding_unaffected() {
        // External-config resolution must not touch zenoh bindings and
        // must not emit SOME/IP-only errors for them.
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
        let deploy = crate::mesh::deploy::parse_deploy_str(yaml).expect("parse");
        let res = resolve_external_bindings(&deploy, Path::new(".")).expect("resolve");
        assert!(res.bindings.is_empty(), "zenoh bindings produce no someip resolution entries");
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
