// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package optional is the Go host type of a value that may be absent,
// `optional<T>` (docs/adr/0055, section 5): the one representation every Go
// surface of SCE spells such a value with.
//
// Of is a struct of two unexported members, built only by Some and None, so a
// value is never present without its value nor absent with one that means
// anything: the absent value holds the zero value of T, which is also what the
// zero value of Of is. It copies by value and never allocates.
//
// A generated machine reads a value out of an Of only where the build has
// proven it present (Proven), or with a default evaluated only when it is absent
// (Otherwise); it tests presence with IsPresent and compares with Equal, which
// is total: two absent values are equal, and an absent value equals no present
// one. A host reads one with Get.
package optional

import (
	"bytes"
	"fmt"
)

// Of is a value of T, or the absence of one.
type Of[T any] struct {
	present bool
	value   T
}

// Some is v, present.
func Some[T any](v T) Of[T] {
	return Of[T]{present: true, value: v}
}

// None is the absent value of T.
func None[T any]() Of[T] {
	return Of[T]{}
}

// Get is the value and true where it is present, and the zero value of T and
// false where it is absent.
func (o Of[T]) Get() (T, bool) {
	return o.value, o.present
}

// IsPresent reports whether the value is present: what `x !== null` reads.
func (o Of[T]) IsPresent() bool {
	return o.present
}

// Proven is the value, read where the build has proven it present (`x.value`
// under a test, docs/adr/0055 section 3). There is no failure path: generated
// code calls it nowhere else, so an absent value never reaches it.
func (o Of[T]) Proven() T {
	return o.value
}

// Equal reports whether o and other are both absent, or both present with
// equal values: `===` between two optional values. A byte string is compared
// by its bytes; every other element, by the value it is.
func (o Of[T]) Equal(other Of[T]) bool {
	if o.present != other.present {
		return false
	}
	if !o.present {
		return true
	}
	if a, ok := any(o.value).([]byte); ok {
		b, _ := any(other.value).([]byte)
		return bytes.Equal(a, b)
	}
	return any(o.value) == any(other.value)
}

// String is how a `<log>` shows the value: `null` where it is absent, and the
// value as the fmt package shows it where it is present (an enum's own String,
// the name its document declares).
func (o Of[T]) String() string {
	if !o.present {
		return "null"
	}
	return fmt.Sprint(o.value)
}

// Otherwise is the value of o where it is present, and what fallback answers
// where it is absent: `x ?? d`. fallback is called only when o is absent, so a
// failure it records is a failure only then.
func Otherwise[T any](o Of[T], fallback func() T) T {
	if o.present {
		return o.value
	}
	return fallback()
}

// OtherwiseOptional is Otherwise where the default may be absent too: `x ?? d`
// with d an optional value. fallback is called only when o is absent.
func OtherwiseOptional[T any](o Of[T], fallback func() Of[T]) Of[T] {
	if o.present {
		return o
	}
	return fallback()
}

// Map is o with f applied to its value where it is present, and absent where it
// is absent: how a present text is held to the bound of the place it is
// written to. f is called only when o is present.
func Map[T any](o Of[T], f func(T) T) Of[T] {
	if !o.present {
		return o
	}
	return Some(f(o.value))
}

// Wire is the value o carries as a JSON value (docs/adr/0055, section 4): nil,
// which the runtime writes as `null`, where it is absent, and what present
// answers for the value where it is present.
func Wire[T any](o Of[T], present func(T) any) any {
	if !o.present {
		return nil
	}
	return present(o.value)
}

// Lift reads the field name of fields, a decoded `_event.data`, as an optional
// value (docs/adr/0055, section 4): absent where the delivery carries `null`, and
// what read answers where it carries a value; read is called only then. A
// delivery that lacks the key is refused: every field of the first revision of
// a schema is carried by every delivery, and `null` is the one spelling of
// absence.
func Lift[T any](fields map[string]any, name string, read func() (T, error)) (Of[T], error) {
	carried, ok := fields[name]
	if !ok {
		return Of[T]{}, fmt.Errorf("the event's data has no %q", name)
	}
	if carried == nil {
		return Of[T]{}, nil
	}
	v, err := read()
	if err != nil {
		return Of[T]{}, err
	}
	return Some(v), nil
}
