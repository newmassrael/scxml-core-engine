// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `sce:encoding="cbor"` codec, generated (SCE_FORGE.md §4.6).
//
// A CBOR codec is one map whose entries are keyed by small unsigned integers:
// order-free on decode, unknown keys skipped, written in ascending key order
// with the shortest heads (RFC 8949 §4.2.1). None of the positional codec's
// machinery — byte offsets, bit sizes, flags, present-if, repeat, TLV — has
// anything to say about it, so it is rendered here rather than through
// `render_codec`'s positional path, which never sees one.

use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::generator::ImportContext;
use crate::forge::model::CodecModel;
use crate::generator::Language;

/// Render a CBOR codec for `lang`.
///
/// A backend whose generation has not landed refuses the document by name
/// rather than emitting a type with no fields: a codec that compiled and
/// wrote an empty map would be a wrong output, and this is a missing one.
pub fn render(
    _env: &minijinja::Environment,
    m: &CodecModel,
    _imports: &[ImportContext],
    lang: Language,
) -> Result<String, ForgeError> {
    Err(ForgeError::from(GenerateError::unsupported_at(
        format!(
            "codec '{}' is sce:encoding=\"cbor\", which has no {lang:?} generation yet",
            m.name
        ),
        m.source_location.clone(),
    )))
}
