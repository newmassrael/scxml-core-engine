// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package sce

// ActionRealization is what a host does with the duties of a machine
// (docs/adr/0038).
//
// A `<sce:action effect="duty">` is an operation the machine's correctness
// depends on the host performing. The generated host interface of a machine
// that states one carries `Realization() sce.ActionRealization`, so the author
// of a host says beside the code whether it performs the duties or only
// simulates them (a recorder, a test double). `New<Machine>Policy` panics for a
// host that simulates; `NewSimulated<Machine>Policy` is the caller saying on
// purpose that it means to run the machine on one.
//
// ActionPerforms is a statement the host's author makes about the bodies of its
// methods, in a line a reviewer can find. SCE cannot read the bodies, and an
// empty body under ActionPerforms is that author's false statement and not an
// unwritten fact.
type ActionRealization int

const (
	// ActionSimulates: the host does not perform the duties; it records, drops or
	// fakes them. It is the zero value, so a host that returns a value no
	// constant names is refused as a simulating one is.
	ActionSimulates ActionRealization = iota
	// ActionPerforms: the host performs the duties; its methods do what the
	// operations' names say.
	ActionPerforms
)

// String names the realization as the interface's documentation does.
func (r ActionRealization) String() string {
	if r == ActionPerforms {
		return "Performs"
	}
	return "Simulates"
}
