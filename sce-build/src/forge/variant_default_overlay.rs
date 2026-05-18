// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// RFC variant-default-overlay Atomic A — apply `deploy.yaml`
// `variant_defaults:` entries onto a parsed forge document.
//
// The wire-spec invariants of a codec (bit positions, MID values per
// `<sce:flag value=...>`) are shared by every consumer of the codec.
// But the *choice* of which arm a freshly-constructed `Default::default()`
// instance dispatches to is per-consumer convention: a zenoh client may
// default a request to query (0x03), a zenoh router may default to push
// (0x1d), and neither choice contradicts the wire spec.
//
// Pre-Atomic A this choice lived inside the SCXML as
// `<sce:arm value="X" default="true"/>`. Atomic A splits it out into
// a deploy overlay so the SCXML stays pure wire-spec and consumers
// pick their own default without forking the codec source.
//
// At codegen time the resolution order is:
//   1. If `variant_defaults` names the codec, that arm wins. The
//      overlay value selects the arm whose `value="..."` matches.
//      `is_default` is set on the matched arm and cleared on every
//      other arm (overrides any SCXML-side `default="true"`).
//   2. Otherwise the SCXML's `<sce:arm default="true"/>` marker wins
//      (legacy path — unchanged from RFC variant-default-uniformity
//      Atomic α-γ).
//   3. Otherwise the `codec/variant-no-default-arm` validator fires
//      at the cross-doc gate (existing γ-3 contract).
//
// Codec names listed in `variant_defaults` that do not match the
// running document are skipped silently — the overlay describes the
// whole doc set, while compile is per-document. Codec names that DO
// match but lack a `<sce:variant>` at all, or whose variant declares
// no `<sce:arm value=...>` matching the overlay value, surface as
// `codec/variant-default-overlay-arm-not-declared`.

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::ForgeDocument;
use crate::mesh::deploy::DeployConfig;

/// Apply a deploy.yaml `variant_defaults:` overlay onto a parsed
/// forge document. Mutates `<sce:variant>` arm `is_default` flags
/// so downstream codegen sees the consumer-chosen arm without
/// needing to consult deploy directly.
///
/// Returns `Ok(())` when:
///   - The document is not a codec (skipped — overlay only applies
///     to `<sce:variant>` which lives on codecs).
///   - The codec is a codec but `variant_defaults` does not name it.
///   - The overlay names this codec AND the named arm value matches
///     a declared `<sce:arm value=...>`. The matched arm's
///     `is_default` is set to `true`; all peer arms have their
///     `is_default` cleared (the overlay is the sole source of
///     truth when present, no SCXML override).
///
/// Returns `Err(CodecVariantDefaultOverlayArmNotDeclared)` when the
/// overlay names this codec but:
///   - The codec has no `<sce:variant>` (no arms to dispatch over).
///   - The codec has a variant but no `<sce:arm value=V/>` matches
///     the overlay's named value.
pub fn apply_variant_default_overlay(
    doc: &mut ForgeDocument,
    deploy: &DeployConfig,
    label_diag: &str,
) -> Result<(), Located<ForgeError>> {
    let codec = match doc {
        ForgeDocument::Codec(c) => c,
        _ => return Ok(()),
    };

    let overlay_arm_value = match deploy.variant_defaults.get(&codec.name) {
        Some(&v) => v,
        None => return Ok(()),
    };

    // Codec is named by overlay — variant must exist and declare a
    // matching arm value.
    let variant = match codec.variant.as_mut() {
        Some(v) => v,
        None => {
            return Err(Located::new(
                ValidationError::CodecVariantDefaultOverlayArmNotDeclared {
                    codec: codec.name.clone(),
                    overlay_arm_value,
                    declared_arms: Vec::new(),
                }
                .into(),
                label_diag,
                None,
                None,
            ));
        }
    };

    let matched_index = variant.arms.iter().position(|a| a.value == overlay_arm_value);
    match matched_index {
        Some(idx) => {
            // Overlay wins — set is_default on the matched arm,
            // clear on every other arm. Idempotent if the SCXML
            // already had `default="true"` on the same arm.
            for (i, arm) in variant.arms.iter_mut().enumerate() {
                arm.is_default = i == idx;
            }
            Ok(())
        }
        None => {
            let mut declared_arms: Vec<u64> = variant.arms.iter().map(|a| a.value).collect();
            declared_arms.sort_unstable();
            Err(Located::new(
                ValidationError::CodecVariantDefaultOverlayArmNotDeclared {
                    codec: codec.name.clone(),
                    overlay_arm_value,
                    declared_arms,
                }
                .into(),
                label_diag,
                None,
                None,
            ))
        }
    }
}
