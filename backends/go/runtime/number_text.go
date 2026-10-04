// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

import (
	"math"
	"strconv"
	"strings"
)

// NumberText spells a 64-bit float as ECMAScript's Number::toString does,
// radix 10.
//
// The one spelling SCE writes on every engine (ARCHITECTURE.md, "JSON Number
// Text (Single Source of Truth)"): the fewest digits that read back as the same
// double, in decimal notation when 1e-6 <= |x| < 1e21 and as d[.ddd]e[+-]n
// otherwise, no fraction on a whole value and "0" for either zero. A value
// that is not finite is spelled NaN, Infinity or -Infinity, as String(x) does;
// a JSON writer, which has no spelling for those, tests for them before it
// calls this. tests/json_text/real_text.json holds the cases.
//
// Go's %g chooses its notation by its own thresholds and a whole value written
// through int64 stops being right at 2^63: neither is this rule, so the digits
// come from strconv's shortest form and the layout is the specification's.
func NumberText(value float64) string {
	switch {
	case math.IsNaN(value):
		return "NaN"
	case math.IsInf(value, 1):
		return "Infinity"
	case math.IsInf(value, -1):
		return "-Infinity"
	case value == 0:
		return "0"
	}
	// strconv's 'e' form with precision -1 is the shortest round-trip digits,
	// d[.ddd]e±dd.
	scientific := strconv.FormatFloat(math.Abs(value), 'e', -1, 64)
	mantissa, exponentText, _ := strings.Cut(scientific, "e")
	digits := strings.Replace(mantissa, ".", "", 1)
	exponent, err := strconv.Atoi(exponentText)
	if err != nil {
		// Unreachable: FormatFloat's exponent is a signed whole number.
		panic("NumberText: " + scientific)
	}
	// The value is 0.<digits> * 10^point; the specification's k and n.
	k := len(digits)
	point := exponent + 1
	var out strings.Builder
	if value < 0 {
		out.WriteByte('-')
	}
	switch {
	case k <= point && point <= 21:
		out.WriteString(digits)
		out.WriteString(strings.Repeat("0", point-k))
	case 0 < point && point <= 21:
		out.WriteString(digits[:point])
		out.WriteByte('.')
		out.WriteString(digits[point:])
	case -6 < point && point <= 0:
		out.WriteString("0.")
		out.WriteString(strings.Repeat("0", -point))
		out.WriteString(digits)
	default:
		power := point - 1
		out.WriteString(digits[:1])
		if k > 1 {
			out.WriteByte('.')
			out.WriteString(digits[1:])
		}
		out.WriteByte('e')
		if power < 0 {
			out.WriteByte('-')
			power = -power
		} else {
			out.WriteByte('+')
		}
		out.WriteString(strconv.Itoa(power))
	}
	return out.String()
}
