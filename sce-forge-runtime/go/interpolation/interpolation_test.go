// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package interpolation

import (
	"math"
	"testing"
)

func TestLinearMidpoint(t *testing.T) {
	axis := []float64{0.0, 1.0, 2.0, 3.0}
	vals := []float64{10.0, 20.0, 30.0, 40.0}
	got := Linear(axis, vals, 1.5)
	if math.Abs(got-25.0) > 1e-12 {
		t.Errorf("Linear(1.5) = %v, want 25.0", got)
	}
}

func TestLinearClampLow(t *testing.T) {
	axis := []float64{1.0, 2.0}
	vals := []float64{10.0, 20.0}
	if got := Linear(axis, vals, 0.5); got != 10.0 {
		t.Errorf("Linear clamp-low = %v, want 10.0", got)
	}
}

func TestLinearClampHigh(t *testing.T) {
	axis := []float64{1.0, 2.0}
	vals := []float64{10.0, 20.0}
	if got := Linear(axis, vals, 5.0); got != 20.0 {
		t.Errorf("Linear clamp-high = %v, want 20.0", got)
	}
}

func TestBilinearCentre(t *testing.T) {
	ax := []float64{0.0, 1.0}
	ay := []float64{0.0, 1.0}
	tab := [][]float64{
		{0.0, 1.0},
		{2.0, 3.0},
	}
	got := Bilinear(ax, ay, tab, 0.5, 0.5)
	if math.Abs(got-1.5) > 1e-12 {
		t.Errorf("Bilinear(0.5, 0.5) = %v, want 1.5", got)
	}
}
