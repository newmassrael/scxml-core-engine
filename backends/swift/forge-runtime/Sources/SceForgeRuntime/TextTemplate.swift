// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The fence of a text template's untrusted hole (docs/adr/0087, section 3.1, item 4).
// Siblings: `sce_forge_runtime::text_template` (Rust), `texttemplate.RemoveMarkers` (Go),
// `com.sce.forge.runtime.TextTemplate` (Kotlin), `sce/forge/text_template.h` (C, C++) and
// `sce_forge_runtime.text_template` (Python).

/// The removal of a fence's markers from an untrusted hole's value.
///
/// The value is put between two markers, and every occurrence of either marker is first
/// removed from it, so that the value cannot close the fence and write outside it. The removal
/// repeats, taking the leftmost occurrence of either marker (the longer first where both start
/// at one byte), until neither occurs: a value cannot rebuild a marker out of two halves around
/// one that is removed. It ends because each pass shortens the value.
///
/// It works on the UTF-8 bytes of the text, as every other runtime does: a `String` compares
/// by its grapheme clusters, so `"<"` followed by a combining mark is no `"<"` to it, and a
/// removal over characters would keep a marker the other runtimes remove. The markers are
/// printable ASCII, which no byte of a multi-byte UTF-8 sequence is, so what is left is UTF-8.
public enum TextTemplate {
    /// `value` with every occurrence of `open` and `close` removed, by the rule of the type. An
    /// empty marker is no marker (a template's fence holds none; the build refuses it).
    public static func removeMarkers(_ value: String, open: String, close: String) -> String {
        let opening = Array(open.utf8)
        let closing = Array(close.utf8)
        let longest = max(opening.count, closing.count)
        var text = Array(value.utf8)
        var from = 0
        while true {
            var found: (at: Int, marker: Int)?
            var index = from
            while index < text.count && found == nil {
                let opens = !opening.isEmpty && starts(text, at: index, with: opening)
                let closes = !closing.isEmpty && starts(text, at: index, with: closing)
                if opens && closes {
                    found = (index, longest)
                } else if opens {
                    found = (index, opening.count)
                } else if closes {
                    found = (index, closing.count)
                }
                index += 1
            }
            guard let hit = found else {
                return String(decoding: text, as: UTF8.self)
            }
            let at = hit.at
            text.removeSubrange(at..<(at + hit.marker))
            // Before at - (longest - 1) nothing changed and nothing matched, so a marker the
            // removal made can only start at or after it.
            from = max(0, at - (longest - 1))
        }
    }

    private static func starts(_ text: [UInt8], at index: Int, with marker: [UInt8]) -> Bool {
        guard text.count - index >= marker.count else {
            return false
        }
        return text[index..<(index + marker.count)].elementsEqual(marker)
    }
}
