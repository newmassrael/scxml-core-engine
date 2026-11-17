// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

// Counted is a list that says how many entries it holds, as a List does.
type Counted interface {
	Len() int
}

// Aligned is two lists that one count counts (docs/adr/0065 §6 R5): the
// wire states the count once, so a value holds the two only where they hold
// as many entries as each other. A codec printed from its plan (docs/adr/0081
// §2) holds them as one Aligned in the arm of the group that holds both. It
// is a struct with unexported fields, made only by MakeAligned, which checks
// the counts once, where the value is made, so the encode over it has no two
// counts to refuse.
//
// Its zero value holds two zero lists, each of no entry, which agree.
type Aligned[F Counted, S Counted] struct {
	first  F
	second S
}

// MakeAligned returns the Aligned of first and second, and false where they
// hold different numbers of entries, which no one count states.
func MakeAligned[F Counted, S Counted](first F, second S) (Aligned[F, S], bool) {
	if first.Len() != second.Len() {
		return Aligned[F, S]{}, false
	}
	return Aligned[F, S]{first: first, second: second}, true
}

// First returns the list the count is derived from.
func (a Aligned[F, S]) First() F {
	return a.first
}

// Second returns the list that shares the first's count.
func (a Aligned[F, S]) Second() S {
	return a.second
}
