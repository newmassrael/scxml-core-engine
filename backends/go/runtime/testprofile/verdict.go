// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import "fmt"

// The verdicts of the test profile (docs/SCE_TEST_PROFILE.md §5): the four a
// document sets with `<sce:setverdict>` (P12) and `error`, which only the
// runtime sets (P13), in the order ETSI ES 201 873-1 §24.1 gives them.
//
// Cross-language siblings: `sce_rust_runtime::test::Verdict` (Rust),
// `SCE::Test::Verdict` (C++), `com.sce.runtime.test.Verdict` (Kotlin).

// Verdict is a test verdict, declared in the order
// `none < pass < inconclusive < fail < error`.
//
// The order is the one ES 201 873-1 §24.1 overwrites by: a verdict only ever
// becomes worse, so the order of the constants is the order, and the zero
// value is VerdictNone.
type Verdict int

const (
	// VerdictNone is `none`: no verdict was set.
	VerdictNone Verdict = iota
	// VerdictPass is `pass`.
	VerdictPass
	// VerdictInconclusive is `inconc`.
	VerdictInconclusive
	// VerdictFail is `fail`.
	VerdictFail
	// VerdictError is `error`: set only by the runtime (P13).
	VerdictError
)

// String is the verdict's spelling in the test log and in TTCN-3. A value no
// constant declares, which no run produces, is shown by its number, as the Go
// standard library shows such a value of its own enumerations.
func (v Verdict) String() string {
	switch v {
	case VerdictNone:
		return "none"
	case VerdictPass:
		return "pass"
	case VerdictInconclusive:
		return "inconc"
	case VerdictFail:
		return "fail"
	case VerdictError:
		return "error"
	}
	return fmt.Sprintf("Verdict(%d)", int(v))
}

// Worse is the worse of a and b: what `setverdict` leaves a local verdict at
// (P12, -4 §9.45), and how the test case verdict folds the local ones (P14).
func Worse(a, b Verdict) Verdict {
	if b > a {
		return b
	}
	return a
}
