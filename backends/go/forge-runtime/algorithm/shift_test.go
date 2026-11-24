// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package algorithm

import (
	"math"
	"testing"
)

// A shift is its exact result reduced to its width for every count at least 0
// (SCE_FORGE.md §3.4.1, "The width of a shift"): every 8-bit value shifted by
// every count up to 10, against the definition made by multiplying and
// flooring.
func TestAShiftIsItsExactResultReducedToItsWidth(t *testing.T) {
	for a := -128; a <= 127; a++ {
		for n := 0; n <= 10; n++ {
			scale := 1 << n
			low := ((a*scale)%256 + 256) % 256
			if low >= 128 {
				low -= 256
			}
			floorQ := a / scale
			if a < 0 && a%scale != 0 {
				floorQ--
			}
			pattern := (a + 256) % 256
			logical := pattern / scale
			if logical >= 128 {
				logical -= 256
			}
			var f Failure
			for _, c := range []struct {
				what      string
				got, want int
			}{
				{"total <<", int(WrappingShl(int8(a), uint64(n))), low},
				{"total >>", int(WrappingShr(int8(a), uint64(n))), floorQ},
				{"checked <<", int(ShlInt8(&f, int8(a), int64(n))), low},
				{"checked >>", int(ShrInt8(&f, int8(a), int64(n))), floorQ},
				{"checked >>>", int(UshrInt8(&f, int8(a), int64(n))), logical},
				{"unsigned >>>", int(UshrUint8(&f, uint8(pattern), int64(n))), pattern / scale},
			} {
				if c.got != c.want {
					t.Errorf("%s of %d by %d: got %d, want %d", c.what, a, n, c.got, c.want)
				}
			}
			if f.Failed() {
				t.Errorf("a count of %d failed: %v", n, f.Err())
			}
		}
	}
	if got := WrappingShl(uint64(1), 64); got != 0 {
		t.Errorf("1 << 64 of a uint64: got %d", got)
	}
	if got := WrappingShr(int64(math.MinInt64), 200); got != -1 {
		t.Errorf("MIN >> 200 of an int64: got %d", got)
	}
	if got := WrappingShr(uint32(math.MaxUint32), 32); got != 0 {
		t.Errorf("MAX >> 32 of a uint32: got %d", got)
	}
	var f Failure
	if got := UshrInt32(&f, -1, 28); got != 15 || f.Failed() {
		t.Errorf("-1 >>> 28 of an int32: got %d (err %v)", got, f.Err())
	}
}

// A count below 0 has no exact result: the checked shift records it as out of
// range and yields 0, never a panic.
func TestACountBelowZeroIsOutOfRange(t *testing.T) {
	for _, run := range []func(*Failure) int64{
		func(f *Failure) int64 { return int64(ShlInt32(f, 1, -1)) },
		func(f *Failure) int64 { return ShrInt64(f, -1, math.MinInt64) },
		func(f *Failure) int64 { return int64(UshrInt16(f, -1, -8)) },
		func(f *Failure) int64 { return int64(ShlUint8(f, 1, -1)) },
	} {
		var f Failure
		got := run(&f)
		if got != 0 || !f.Failed() || f.Err().Error() != "out-of-range" {
			t.Errorf("got %d (err %v), want the failure out-of-range", got, f.Err())
		}
	}
}
