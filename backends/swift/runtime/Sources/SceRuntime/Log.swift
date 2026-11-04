// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation

/// Writes one diagnostic line to the standard error stream, prefixed `[sce]`.
///
/// Only for what a host cannot read anywhere else: a refusal, a ceiling reached, an
/// anomaly. The engine's counters and `last*` readers are the contract; this is the
/// line beside them that says it happened when nobody was asking.
func sceLog(_ message: @autoclosure () -> String) {
    FileHandle.standardError.write(Data("[sce] \(message())\n".utf8))
}

/// Writes, through `sceLog`, the line a generated machine's `<sce:render>` writes once it has
/// replaced `location` with the rendering of `template` (docs/adr/0087, section 3.1, item 6):
/// the machine, the template and the variable, never the text, which may hold what a person
/// wrote. The same line the Rust, C++, C11 and Go machines write through their logs; a render
/// whose hole failed raised `error.execution` instead and writes no line.
public func sceLogRender(machine: String, template: String, location: String) {
    sceLog("\(machine): <sce:render template=\"\(template)\"> wrote \(location)")
}

extension Duration {
    /// This duration in whole milliseconds, truncated toward zero.
    var wholeMilliseconds: Int64 {
        let (seconds, attoseconds) = components
        return seconds * 1000 + attoseconds / 1_000_000_000_000_000
    }
}
