// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// Spells a 64-bit float as ECMAScript's Number::toString does, radix 10.
///
/// The one spelling SCE writes on every engine (ARCHITECTURE.md, "JSON Number
/// Text (Single Source of Truth)"): the fewest digits that read back as the same
/// double, in decimal notation when 1e-6 <= |x| < 1e21 and as `d[.ddd]e[+-]n`
/// otherwise, no fraction on a whole value and "0" for either zero. A value that
/// is not finite is spelled NaN, Infinity or -Infinity, as `String(x)` does; a
/// JSON writer, which has no spelling for those, tests for them before it calls
/// this. `tests/json_text/real_text.json` holds the cases.
///
/// Swift's `description` chooses its notation by its own thresholds and writes a
/// whole value with a `.0`: neither is this rule. Its digits are the shortest
/// that read back, so the digits come from there and the layout is the
/// specification's.
public func numberText(_ value: Double) -> String {
    if value.isNaN {
        return "NaN"
    }
    if value.isInfinite {
        return value < 0 ? "-Infinity" : "Infinity"
    }
    if value == 0 {
        return "0"
    }

    // The digits and the position of the decimal point in them: the value is
    // 0.<digits> * 10^point, the specification's k and n.
    let (digits, point) = shortestDigits(of: abs(value))
    let k = digits.count
    var out = value < 0 ? "-" : ""
    if k <= point && point <= 21 {
        out += digits
        out += String(repeating: "0", count: point - k)
    } else if 0 < point && point <= 21 {
        out += digits.prefix(point)
        out += "."
        out += digits.dropFirst(point)
    } else if -6 < point && point <= 0 {
        out += "0."
        out += String(repeating: "0", count: -point)
        out += digits
    } else {
        var power = point - 1
        out += digits.prefix(1)
        if k > 1 {
            out += "."
            out += digits.dropFirst(1)
        }
        out += "e"
        if power < 0 {
            out += "-"
            power = -power
        } else {
            out += "+"
        }
        out += String(power)
    }
    return out
}

/// The shortest digits that read back as `value` (positive and finite), with no
/// leading or trailing zero, and where the decimal point stands in them.
private func shortestDigits(of value: Double) -> (digits: String, point: Int) {
    // `description` is the shortest round-trip text, either `123.456` or
    // `1.2345e-07`; the exponent part is optional.
    let text = value.description
    let parts = text.split(separator: "e", maxSplits: 1, omittingEmptySubsequences: false)
    let exponent = parts.count == 2 ? Int(parts[1])! : 0
    let mantissa = parts[0].split(separator: ".", maxSplits: 1, omittingEmptySubsequences: false)
    let whole = String(mantissa[0])
    let fraction = mantissa.count == 2 ? String(mantissa[1]) : ""

    var digits = Substring(whole + fraction)
    var point = whole.count + exponent
    while digits.first == "0" {
        digits = digits.dropFirst()
        point -= 1
    }
    while digits.last == "0" {
        digits = digits.dropLast()
    }
    return (String(digits), point)
}
