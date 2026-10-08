// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `sce:encoding="content-line"` codec, generated (SCE_FORGE.md §4.6.4,
// docs/adr/0010).
//
// A content-line codec is one RFC 5545 component: `BEGIN:<component>`, the
// declared properties as folded content lines, `END:<component>`. None of the
// positional codec's machinery — byte offsets, bit sizes, flags, present-if,
// repeat, TLV — has anything to say about it, so it is rendered here rather
// than through `render_codec`'s positional path, which never sees one.
//
// Declared and refused by name until a backend generates it, as the CBOR codec
// was (`forge::cbor_codec`): a codec that compiled and wrote an empty
// component would be a wrong output, and this is a missing one.

use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::generator::ImportContext;
use crate::forge::model::CodecModel;
use crate::generator::Language;

/// Whether `lang` generates a content-line codec — the one answer the
/// generator's refusal and the conformance harness's schedule both read, so a
/// fixture is run exactly where the generator admits it.
pub fn lowers(_lang: Language) -> bool {
    false
}

/// Why `lang` does not generate the content-line codec `m`, or `None` when it
/// does.
pub fn refusal(lang: Language, m: &CodecModel) -> Option<String> {
    if !lowers(lang) {
        return Some(format!(
            "codec '{}' is sce:encoding=\"content-line\", which has no {lang:?} generation yet",
            m.name
        ));
    }
    None
}

/// Render a content-line codec for `lang`.
pub fn render(
    _env: &minijinja::Environment,
    m: &CodecModel,
    _imports: &[ImportContext],
    lang: Language,
) -> Result<String, ForgeError> {
    match refusal(lang, m) {
        Some(why) => Err(ForgeError::from(GenerateError::unsupported_at(
            why,
            m.source_location.clone(),
        ))),
        None => unreachable!("no backend generates a content-line codec yet"),
    }
}
