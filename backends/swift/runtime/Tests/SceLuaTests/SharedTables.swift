// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceLua

// The tables every engine reads live under `tests/ecmascript` at the repository root, and these
// tests read them where they are and do not copy them: a per-engine copy of the expectations drifts
// toward the engine that reads it.

/// The repository root, found from this file's own path.
func repositoryRoot(file: String = #filePath) -> URL {
    // <root>/backends/swift/runtime/Tests/SceLuaTests/<this file>
    var url = URL(fileURLWithPath: file)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url
}

/// The cases of the shared table `tests/ecmascript/<table>`.
func cases(of table: String) throws -> [[String: Any]] {
    let url = repositoryRoot().appendingPathComponent("tests/ecmascript").appendingPathComponent(table)
    let object = try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any]
    return try #require(object?["cases"] as? [[String: Any]])
}

/// Whether `actual` is the answer `expect` names. An engine may hold a whole number as an integer or
/// as a double, and ECMA-262 has one Number type, so both spellings answer a `number` case. A case
/// whose expectation names no answer is not a case that passes: reading it as "no answer" would let a
/// typo in a key retire a case silently.
func answers(_ actual: LuaValue, _ expect: [String: Any]) throws -> Bool {
    if let want = expect["bool"] as? Bool {
        return actual == .boolean(want)
    }
    if let want = expect["number"] as? Double {
        switch actual {
        case .integer(let value): return abs(Double(value) - want) < 1e-9
        case .number(let value): return abs(value - want) < 1e-9
        default: return false
        }
    }
    if let want = expect["string"] as? String {
        return actual == .string(want)
    }
    if expect["empty"] != nil {
        return actual == .nilValue
    }
    throw LuaError.runtime("an expectation that names no answer: \(expect)")
}
