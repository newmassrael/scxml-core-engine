// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package filter

import (
	"math"
	"testing"
)

func TestMovingAverageFirstSample(t *testing.T) {
	ma := NewMovingAverage[float64](4)
	if got := ma.Update(1.0); math.Abs(got-1.0) > 1e-12 {
		t.Errorf("MA first sample = %v, want 1.0", got)
	}
}

func TestMovingAverageFloat32(t *testing.T) {
	// Verifies the generic constraint accepts float32 too.
	ma := NewMovingAverage[float32](2)
	if got := ma.Update(1.0); got != 1.0 {
		t.Errorf("MA<float32> first sample = %v, want 1.0", got)
	}
}

func TestLowPassFirstSamplePassThrough(t *testing.T) {
	lp := NewLowPass[float64](0.1)
	if got := lp.Update(7.0); math.Abs(got-7.0) > 1e-12 {
		t.Errorf("LowPass first sample = %v, want 7.0", got)
	}
}

func TestDebounceUntilFilled(t *testing.T) {
	db := NewDebounce[bool](3)
	if got := db.Update(true); got != true {
		t.Errorf("Debounce sample 1 = %v, want true", got)
	}
	if got := db.Update(false); got != false {
		t.Errorf("Debounce sample 2 = %v, want false", got)
	}
}

func TestDebounceInteger(t *testing.T) {
	// Verifies that Debounce works with non-Boolean comparable types.
	db := NewDebounce[int](2)
	if got := db.Update(5); got != 5 {
		t.Errorf("Debounce[int] first sample = %v, want 5", got)
	}
}
