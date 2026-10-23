// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

/// The repository root, found from this file's own path: the tables every engine
/// reads live under `tests/` at the root, and the tests read them where they are
/// and do not copy them.
func repositoryRoot(file: String = #filePath) -> URL {
    // <root>/backends/swift/runtime/Tests/SceRuntimeTests/<this file>
    var url = URL(fileURLWithPath: file)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url
}

/// The cases of one shared table, `tests/<table>`.
func sharedCases(_ table: String) throws -> [[String: Any]] {
    let data = try Data(contentsOf: repositoryRoot().appendingPathComponent("tests").appendingPathComponent(table))
    let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]
    return try #require(object?["cases"] as? [[String: Any]])
}
