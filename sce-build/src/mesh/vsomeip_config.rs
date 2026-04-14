// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Partial-schema parser for OEM-supplied `vsomeip.json` (SCE_MESH.md §13).
//
// vsomeip.json is owned by the platform team / ARXML pipeline — sce-build
// must not rewrite it or demand fields outside the documented vsomeip
// schema. Only the fields that deploy.yaml references by name are declared:
//
//   applications[*].name   → binds generated runtime to a vsomeip identity
//   services[*].name       → resolves service_id + instance_id
//   services[*].methods[*] → resolves method_id (also SOME/IP field getters/setters)
//   services[*].eventgroups[*] → resolves event_group_id + the contained event_id
//
// Every other field in the file passes through untouched (no
// `deny_unknown_fields`) because vsomeip treats the JSON as its own
// configuration surface at runtime.
//
// Numeric IDs in vsomeip.json are conventionally written as `"0x1234"`
// hex strings. We accept both hex strings and JSON integers.

use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::mesh::error::ExternalConfigError;

/// Top-level vsomeip.json partial schema.
#[derive(Debug, Clone, Deserialize)]
pub struct VsomeipConfig {
    #[serde(default)]
    pub applications: Vec<Application>,
    #[serde(default)]
    pub services: Vec<Service>,
}

/// Application identity — deploy.yaml `application_name:` resolves here.
#[derive(Debug, Clone, Deserialize)]
pub struct Application {
    pub name: String,
}

/// Service — deploy.yaml `service: <name>` resolves here.
///
/// Field names match the vsomeip JSON schema exactly:
/// `service`/`instance` (numeric IDs) and `name` (display label).
#[derive(Debug, Clone, Deserialize)]
pub struct Service {
    pub name: String,
    #[serde(deserialize_with = "de_hex_or_int_u16")]
    pub service: u16,
    #[serde(deserialize_with = "de_hex_or_int_u16")]
    pub instance: u16,
    #[serde(default)]
    pub methods: Vec<Method>,
    #[serde(default)]
    pub eventgroups: Vec<EventGroup>,
}

/// Method — deploy.yaml `method:` / `getter:` / `setter:` all resolve here.
///
/// SOME/IP field accessors are encoded as methods in the vsomeip schema;
/// distinguishing getter/setter/method is a deploy.yaml concern, not a
/// vsomeip.json concern.
#[derive(Debug, Clone, Deserialize)]
pub struct Method {
    pub name: String,
    #[serde(deserialize_with = "de_hex_or_int_u16")]
    pub method: u16,
}

/// Event group — deploy.yaml `event_group: <name>` resolves here.
///
/// For the current one-event-per-target template model, the event group
/// must contain exactly one event; multi-event groups require per-event
/// template fanout and are rejected at resolution time.
#[derive(Debug, Clone, Deserialize)]
pub struct EventGroup {
    pub name: String,
    #[serde(deserialize_with = "de_hex_or_int_u16")]
    pub eventgroup: u16,
    #[serde(default, deserialize_with = "de_hex_or_int_u16_vec")]
    pub events: Vec<u16>,
}

impl VsomeipConfig {
    /// Parse a vsomeip.json file from disk.
    pub fn load(path: &Path) -> Result<Self, ExternalConfigError> {
        let content = fs::read_to_string(path).map_err(|source| ExternalConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        serde_json::from_str(&content).map_err(|e| ExternalConfigError::Parse {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Look up a service by its `name` field.
    pub fn resolve_service(&self, name: &str) -> Option<&Service> {
        self.services.iter().find(|s| s.name == name)
    }

    /// Look up a method by (service name, method name).
    ///
    /// Returns the method's numeric id. Callers requiring the owning
    /// service's numeric ids should `resolve_service` first.
    pub fn resolve_method(&self, service: &str, method: &str) -> Option<u16> {
        self.resolve_service(service)?
            .methods
            .iter()
            .find(|m| m.name == method)
            .map(|m| m.method)
    }

    /// Look up an event group by (service name, event group name).
    pub fn resolve_event_group<'a>(
        &'a self,
        service: &str,
        event_group: &str,
    ) -> Option<&'a EventGroup> {
        self.resolve_service(service)?
            .eventgroups
            .iter()
            .find(|e| e.name == event_group)
    }
}

/// Accept either `"0x1234"` (hex string) or `4660` (JSON integer) for u16 ids.
///
/// vsomeip.json traditionally uses `"0x..."`; a minority of configs or
/// hand-edited files use plain integers. Accepting both avoids spurious
/// build failures on files sce-build did not generate.
fn de_hex_or_int_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match &v {
        serde_json::Value::String(s) => parse_hex_or_decimal_u16(s).map_err(D::Error::custom),
        serde_json::Value::Number(n) => n
            .as_u64()
            .and_then(|u| u16::try_from(u).ok())
            .ok_or_else(|| D::Error::custom(format!("id '{n}' does not fit in u16"))),
        other => Err(D::Error::custom(format!(
            "expected hex string or integer id, got {other}"
        ))),
    }
}

fn de_hex_or_int_u16_vec<'de, D>(deserializer: D) -> Result<Vec<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let values: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    values
        .into_iter()
        .map(|v| match v {
            serde_json::Value::String(s) => parse_hex_or_decimal_u16(&s).map_err(D::Error::custom),
            serde_json::Value::Number(n) => n
                .as_u64()
                .and_then(|u| u16::try_from(u).ok())
                .ok_or_else(|| D::Error::custom(format!("id '{n}' does not fit in u16"))),
            other => Err(D::Error::custom(format!(
                "expected hex string or integer id, got {other}"
            ))),
        })
        .collect()
}

fn parse_hex_or_decimal_u16(s: &str) -> Result<u16, String> {
    let trimmed = s.trim();
    let (radix, digits) = if let Some(rest) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        (16u32, rest)
    } else {
        (10u32, trimmed)
    };
    u16::from_str_radix(digits, radix)
        .map_err(|e| format!("invalid u16 literal '{s}': {e}"))
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "applications": [ { "name": "brake_app" } ],
      "services": [
        {
          "name": "motor_control",
          "service": "0x1234",
          "instance": "0x0001",
          "methods": [
            { "name": "compute_force", "method": "0x0421" },
            { "name": "release_force", "method": "0x0422" }
          ],
          "eventgroups": [
            { "name": "status_group", "eventgroup": "0x0001",
              "events": ["0x8001"] }
          ]
        }
      ]
    }"#;

    #[test]
    fn parses_sample_config() {
        let cfg: VsomeipConfig = serde_json::from_str(SAMPLE).expect("parse");
        assert_eq!(cfg.applications.len(), 1);
        assert_eq!(cfg.applications[0].name, "brake_app");
        assert_eq!(cfg.services.len(), 1);
        assert_eq!(cfg.services[0].name, "motor_control");
        assert_eq!(cfg.services[0].service, 0x1234);
        assert_eq!(cfg.services[0].instance, 0x0001);
    }

    #[test]
    fn resolves_service_by_name() {
        let cfg: VsomeipConfig = serde_json::from_str(SAMPLE).unwrap();
        let svc = cfg.resolve_service("motor_control").expect("service");
        assert_eq!(svc.service, 0x1234);
        assert!(cfg.resolve_service("unknown").is_none());
    }

    #[test]
    fn resolves_method() {
        let cfg: VsomeipConfig = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(
            cfg.resolve_method("motor_control", "compute_force"),
            Some(0x0421)
        );
        assert_eq!(
            cfg.resolve_method("motor_control", "release_force"),
            Some(0x0422)
        );
        assert_eq!(cfg.resolve_method("motor_control", "missing"), None);
        assert_eq!(cfg.resolve_method("missing_service", "compute_force"), None);
    }

    #[test]
    fn resolves_event_group_with_events() {
        let cfg: VsomeipConfig = serde_json::from_str(SAMPLE).unwrap();
        let eg = cfg
            .resolve_event_group("motor_control", "status_group")
            .expect("eg");
        assert_eq!(eg.eventgroup, 0x0001);
        assert_eq!(eg.events, vec![0x8001]);
    }

    #[test]
    fn accepts_integer_ids_as_well_as_hex() {
        // Some tooling emits `"service": 4660` rather than `"0x1234"`.
        let cfg: VsomeipConfig = serde_json::from_str(
            r#"{
              "services": [
                { "name": "m", "service": 4660, "instance": 1,
                  "methods": [{ "name": "f", "method": 1057 }] }
              ]
            }"#,
        )
        .expect("parse");
        assert_eq!(cfg.services[0].service, 4660);
        assert_eq!(cfg.services[0].methods[0].method, 1057);
    }

    #[test]
    fn unknown_top_level_fields_ignored() {
        // vsomeip.json carries many fields we don't model (routing, security,
        // trace filters). Partial schema must silently ignore them.
        let cfg: VsomeipConfig = serde_json::from_str(
            r#"{
              "applications": [{ "name": "app" }],
              "services": [],
              "routing": "io.vsomeip.routing",
              "security": { "policies": [] },
              "tracing": { "enable": "false" }
            }"#,
        )
        .expect("parse");
        assert_eq!(cfg.applications[0].name, "app");
    }

    #[test]
    fn rejects_out_of_range_id() {
        let err = serde_json::from_str::<VsomeipConfig>(
            r#"{ "services": [{ "name": "m", "service": "0x1FFFF", "instance": 1 }] }"#,
        )
        .expect_err("must reject > u16::MAX");
        assert!(err.to_string().contains("invalid u16 literal") || err.to_string().contains("u16"));
    }

    #[test]
    fn uppercase_hex_prefix_accepted() {
        assert_eq!(parse_hex_or_decimal_u16("0X1234"), Ok(0x1234));
        assert_eq!(parse_hex_or_decimal_u16("0x1234"), Ok(0x1234));
        assert_eq!(parse_hex_or_decimal_u16("100"), Ok(100));
    }

    #[test]
    fn decimal_only_string_accepted() {
        // Fallback path when no 0x prefix is present.
        let cfg: VsomeipConfig = serde_json::from_str(
            r#"{ "services": [{ "name": "s", "service": "100", "instance": "1" }] }"#,
        )
        .unwrap();
        assert_eq!(cfg.services[0].service, 100);
    }
}
