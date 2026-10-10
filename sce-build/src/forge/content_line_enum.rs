// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The vocabulary an `enum:<alias>` entry of a content-line codec reads and
// writes (SCE_FORGE.md §4.6.4, docs/adr/0015).
//
// A codec reads a text off a line and names the variant whose text it is, so
// the enum it imports must give every variant a text that can stand on a line
// and that no other variant has. The enum document holds the rules for an
// explicit `sce:text` (an `iana-token`, unique under ASCII case folding); a
// variant that has none is called by its declared name, and a name is not
// held to either rule where an enum is used for anything else, so it is held
// here, where a codec reads it by that name.

use std::collections::BTreeMap;
use std::path::Path;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::import_source;
use crate::forge::model::{ForgeDocument, ParsedForge, SceType};

/// The characters of an `iana-token` (RFC 5545 §3.1): ASCII letters, digits and
/// `-`, and at least one.
pub fn is_token(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Refuse a content-line codec whose `enum:<alias>` entry imports an enum with a
/// variant that has no text a line can carry, or two that have the same one.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    let ForgeDocument::Codec(codec) = &parsed.document else {
        return Ok(());
    };
    let Some(lines) = codec.content_line.as_ref() else {
        return Ok(());
    };
    // One enum is one vocabulary however many entries read it, so it is judged
    // once, at the first entry that names it.
    let mut judged: Vec<&str> = Vec::new();
    for entry in &lines.entries {
        let SceType::Enum(eref) = &entry.sce_type else {
            continue;
        };
        if judged.contains(&eref.alias.as_str()) {
            continue;
        }
        judged.push(&eref.alias);
        // An unreadable import is silent here; see `import_source::parse_quietly`.
        let Some(vocabulary) = import_source::enum_model(parsed, base_dir, &eref.alias) else {
            continue;
        };
        let refuse = |rule: String| {
            Err(Located::new(
                ValidationError::AttributeRuleViolated {
                    element: format!("entry '{}'", entry.id),
                    attr: "sce:type".into(),
                    value: format!("enum:{}", eref.alias),
                    rule,
                }
                .into(),
                document,
                entry.line,
                None,
            ))
        };
        let mut seen: BTreeMap<String, &str> = BTreeMap::new();
        for variant in &vocabulary.variants {
            let text = variant.wire_text();
            if !is_token(text) {
                return refuse(format!(
                    "an enum every variant of which has a text of ASCII letters, digits and \
                     `-`; variant `{}` has `{text}` (give it an sce:text)",
                    variant.name
                ));
            }
            if let Some(first) = seen.insert(text.to_ascii_lowercase(), &variant.name) {
                return refuse(format!(
                    "an enum whose variants have texts that differ under ASCII case folding; \
                     variants `{first}` and `{}` are both `{text}`",
                    variant.name
                ));
            }
        }
    }
    Ok(())
}
