// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package codec

import "testing"

// An Aligned is made only of two lists of one count, and gives each back.

// upToThree is a bound of none to three entries.
type upToThree struct{}

func (upToThree) Least() int { return 0 }
func (upToThree) Most() int  { return 3 }

func TestMakeAlignedHoldsTwoListsOfOneCount(t *testing.T) {
	first, err := MakeList[upToThree]([]int{1, 2})
	if err != nil {
		t.Fatalf("two entries are within the bound: %v", err)
	}
	second, err := MakeList[upToThree]([]string{"a", "b"})
	if err != nil {
		t.Fatalf("two entries are within the bound: %v", err)
	}
	pair, ok := MakeAligned(first, second)
	if !ok {
		t.Fatal("two lists of two entries each are aligned")
	}
	if pair.First().Len() != 2 || pair.Second().Len() != 2 {
		t.Fatalf("the pair gives back both lists: %d and %d entries", pair.First().Len(), pair.Second().Len())
	}
}

func TestMakeAlignedRefusesTwoListsOfDifferentCounts(t *testing.T) {
	first, err := MakeList[upToThree]([]int{1, 2})
	if err != nil {
		t.Fatalf("two entries are within the bound: %v", err)
	}
	second, err := MakeList[upToThree]([]string{"a"})
	if err != nil {
		t.Fatalf("one entry is within the bound: %v", err)
	}
	if _, ok := MakeAligned(first, second); ok {
		t.Fatal("a list of two and a list of one share no count")
	}
}

func TestTheZeroAlignedHoldsTwoEmptyLists(t *testing.T) {
	var pair Aligned[List[upToThree, int], List[upToThree, string]]
	if pair.First().Len() != 0 || pair.Second().Len() != 0 {
		t.Fatal("the zero pair holds no entry in either list")
	}
}
