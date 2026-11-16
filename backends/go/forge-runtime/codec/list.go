// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

import (
	"iter"

	"github.com/newmassrael/sce-forge-runtime/optional"
)

// List is a list or a chain of entries E held to its bound B, a count of
// entries (docs/adr/0077 §3, docs/adr/0065 §6 R2). A codec printed from its
// plan (docs/adr/0081 §2) presents each list as a List of the source
// interface of its entries' codec and a bound type it names, so a list is
// made of any sources of its entries. It is a struct with an unexported
// field, made only by MakeList, which checks the bound once, where the value
// is made: the encode over it has nothing of the bound to refuse, as for
// Bytes and Text. Its entries are read in order through All, an iter.Seq.
//
// Its zero value holds no entry. Where the bound's Least is above zero, that
// is no list a frame carries, as no entry of an interface type can be made
// for it: a value of such a list is made by MakeList.
type List[B Bound, E any] struct {
	held []E
}

// MakeList returns the List that holds entries, or ErrTooManyElements where
// it holds more than the bound's Most and ErrLengthOutOfRange where it holds
// fewer than its Least. The List holds the slice and does not copy it.
func MakeList[B Bound, E any](entries []E) (List[B, E], error) {
	var bound B
	if len(entries) > bound.Most() {
		return List[B, E]{}, ErrTooManyElements
	}
	if len(entries) < bound.Least() {
		return List[B, E]{}, ErrLengthOutOfRange
	}
	return List[B, E]{held: entries}, nil
}

// All yields each entry in order.
func (l List[B, E]) All() iter.Seq[E] {
	return func(yield func(E) bool) {
		for _, entry := range l.held {
			if !yield(entry) {
				return
			}
		}
	}
}

// Len returns how many entries the list holds.
func (l List[B, E]) Len() int {
	return len(l.held)
}

// MakeOptionalList is MakeList over a list a gate may leave out
// (docs/adr/0058 §1): absent where o is, and the List that holds it, or
// MakeList's error, where it is there.
func MakeOptionalList[B Bound, E any](o optional.Of[[]E]) (optional.Of[List[B, E]], error) {
	entries, ok := o.Get()
	if !ok {
		return optional.None[List[B, E]](), nil
	}
	held, err := MakeList[B](entries)
	if err != nil {
		return optional.None[List[B, E]](), err
	}
	return optional.Some(held), nil
}

// SourcesOf returns the source each entry of a decoded list is, its address
// as source answers it, in order: a decode reads the entries as values of
// their codec, and a List holds them as sources of it.
func SourcesOf[T any, E any](entries []T, source func(*T) E) []E {
	sources := make([]E, len(entries))
	for i := range entries {
		sources[i] = source(&entries[i])
	}
	return sources
}

// OptionalSourcesOf is SourcesOf over a list a gate may leave out: absent
// where entries is.
func OptionalSourcesOf[T any, E any](entries optional.Of[[]T], source func(*T) E) optional.Of[[]E] {
	return optional.Convert(entries, func(held []T) []E {
		return SourcesOf(held, source)
	})
}
