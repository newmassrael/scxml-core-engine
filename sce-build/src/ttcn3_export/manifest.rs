// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The structural manifest of a suite's types module (ADR 0091 C2;
//! docs/SCE_TEST_PROFILE.md §9.5): the model the module is written from
//! ([`super::types::TypesModule`]), serialized, a wire surface of its own
//! (`schemas/sce-ttcn3-types-manifest.v1.schema.json`).
//!
//! It states every definition of the module by its name, its kind and its
//! structure, and the codec functions the module declares, so a consumer can
//! hold a types module written apart from SCE, with a codec of its own, to the
//! one the export writes: same names, same kinds, same fields in the same
//! order, the same bounds, the same functions. It names nothing of the run
//! that wrote it; the module beside it names its inputs in its header.

use super::types::TypesModule;

/// The stability status of the manifest's wire schema, pinned to the schema
/// file's own `x-sce-schema-status` by `schema_file_declares_status`.
/// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
pub const TYPES_MANIFEST_SCHEMA_STATUS: &str = "pre-release";

/// The manifest's text: `module` as JSON, two spaces an indentation level,
/// every object's keys in the order the model declares them, every value
/// that may pass 2^53 (a bound of an integer type, the number of a variant)
/// the text of its digits.
pub fn manifest_text(module: &TypesModule) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = include_str!("../../../schemas/sce-ttcn3-types-manifest.v1.schema.json");

    #[test]
    fn schema_file_declares_status() {
        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("the manifest's schema is JSON");
        assert_eq!(
            schema["x-sce-schema-status"].as_str(),
            Some(TYPES_MANIFEST_SCHEMA_STATUS),
            "schemas/sce-ttcn3-types-manifest.v1.schema.json x-sce-schema-status disagrees \
             with TYPES_MANIFEST_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to \
             move both",
        );
    }
}
