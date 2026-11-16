// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

import (
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
)

// A List is made only within its bound, and yields its entries in order.

// oneToThree is a bound of one to three entries.
type oneToThree struct{}

func (oneToThree) Least() int { return 1 }
func (oneToThree) Most() int  { return 3 }

func TestMakeListHoldsEntriesWithinTheBoundInOrder(t *testing.T) {
	list, err := MakeList[oneToThree]([]int{4, 5, 6})
	if err != nil {
		t.Fatalf("three entries are within one to three: %v", err)
	}
	if list.Len() != 3 {
		t.Fatalf("Len: want 3, got %d", list.Len())
	}
	var got []int
	for entry := range list.All() {
		got = append(got, entry)
	}
	if len(got) != 3 || got[0] != 4 || got[1] != 5 || got[2] != 6 {
		t.Fatalf("All yields the entries in order: got %v", got)
	}
	// A walk that stops early stops the yield.
	for entry := range list.All() {
		if entry != 4 {
			t.Fatalf("the first entry is 4, got %d", entry)
		}
		break
	}
}

func TestMakeListRefusesAListOutsideTheBound(t *testing.T) {
	if _, err := MakeList[oneToThree]([]int{1, 2, 3, 4}); err != ErrTooManyElements {
		t.Fatalf("four entries: want ErrTooManyElements, got %v", err)
	}
	if _, err := MakeList[oneToThree]([]int{}); err != ErrLengthOutOfRange {
		t.Fatalf("no entry: want ErrLengthOutOfRange, got %v", err)
	}
}

func TestAnOptionalListIsAbsentOrHeldToItsBound(t *testing.T) {
	absent, err := MakeOptionalList[oneToThree](optional.None[[]int]())
	if err != nil || absent.IsPresent() {
		t.Fatalf("an absent list is made absent: %v, %v", absent, err)
	}
	present, err := MakeOptionalList[oneToThree](optional.Some([]int{7}))
	if err != nil || !present.IsPresent() || present.Proven().Len() != 1 {
		t.Fatalf("a present list is made: %v", err)
	}
	if _, err := MakeOptionalList[oneToThree](optional.Some([]int{})); err != ErrLengthOutOfRange {
		t.Fatalf("a present list outside its bound: want ErrLengthOutOfRange, got %v", err)
	}
}

func TestSourcesOfAreTheAddressesOfTheEntries(t *testing.T) {
	entries := []int{1, 2}
	sources := SourcesOf(entries, func(e *int) *int { return e })
	if len(sources) != 2 || sources[0] != &entries[0] || sources[1] != &entries[1] {
		t.Fatalf("each source is its entry's address")
	}
	if OptionalSourcesOf(optional.None[[]int](), func(e *int) *int { return e }).IsPresent() {
		t.Fatalf("an absent list has absent sources")
	}
}
