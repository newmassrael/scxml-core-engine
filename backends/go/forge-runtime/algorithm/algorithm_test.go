// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package algorithm

import (
	"math"
	"testing"
)

// Each failure the contract names is reported by its contract name, and every
// value that fits comes back unchanged.
func TestEachFailureIsNamedAndEveryValuePasses(t *testing.T) {
	cases := []struct {
		name string
		run  func(*Failure) int64
		want int64
		fail string
	}{
		{"max sum", func(f *Failure) int64 { return int64(AddUint32(f, math.MaxUint32-1, 1)) }, math.MaxUint32, ""},
		{"sum past max", func(f *Failure) int64 { return int64(AddUint32(f, math.MaxUint32, 1)) }, 0, "overflow"},
		{"below zero", func(f *Failure) int64 { return int64(SubUint8(f, 0, 1)) }, 0, "overflow"},
		{"product past max", func(f *Failure) int64 { return int64(MulInt16(f, 256, 128)) }, 0, "overflow"},
		{"truncated", func(f *Failure) int64 { return int64(DivInt32(f, -7, 2)) }, -3, ""},
		{"dividend sign", func(f *Failure) int64 { return int64(RemInt32(f, -7, 3)) }, -1, ""},
		{"divide by zero", func(f *Failure) int64 { return int64(DivUint8(f, 1, 0)) }, 0, "divide-by-zero"},
		{"remainder by zero", func(f *Failure) int64 { return RemInt64(f, 1, 0) }, 0, "divide-by-zero"},
		{"MIN / -1", func(f *Failure) int64 { return int64(DivInt8(f, math.MinInt8, -1)) }, 0, "overflow"},
		{"MIN % -1", func(f *Failure) int64 { return int64(RemInt8(f, math.MinInt8, -1)) }, 0, "overflow"},
		{"-MIN", func(f *Failure) int64 { return int64(NegInt32(f, math.MinInt32)) }, 0, "overflow"},
		{"negate", func(f *Failure) int64 { return int64(NegInt32(f, 5)) }, -5, ""},
		{"64-bit product", func(f *Failure) int64 { return MulInt64(f, math.MaxInt64, 2) }, 0, "overflow"},
	}
	for _, c := range cases {
		var f Failure
		got := c.run(&f)
		switch {
		case c.fail == "" && f.Failed():
			t.Errorf("%s: failed with %v, want %d", c.name, f.Err(), c.want)
		case c.fail == "" && got != c.want:
			t.Errorf("%s: got %d, want %d", c.name, got, c.want)
		case c.fail != "" && (!f.Failed() || f.Err().Error() != c.fail):
			t.Errorf("%s: got %d (err %v), want failure %s", c.name, got, f.Err(), c.fail)
		}
	}
}

// A call to another may-fail algorithm is its value, or its failure recorded
// in the caller's Failure under the callee's own name.
func TestTakePassesACalleeFailureOn(t *testing.T) {
	callee := func(fail bool) (uint32, error) {
		var f Failure
		if fail {
			f.Fail(DivideByZero)
			return 0, f.Err()
		}
		return 7, f.Err()
	}
	var f Failure
	if got := Take[uint32](&f)(callee(false)); got != 7 || f.Failed() {
		t.Errorf("got %d (err %v), want 7", got, f.Err())
	}
	if got := Take[uint32](&f)(callee(true)); got != 0 || f.Err() == nil || f.Err().Error() != "divide-by-zero" {
		t.Errorf("got %d (err %v), want the callee's divide-by-zero", got, f.Err())
	}
}

// A conditional of two typed values answers the one its condition names, and its
// type is the values' own.
func TestChooseAnswersTheBranchTheConditionNames(t *testing.T) {
	type mode uint8
	const (
		month mode = 0
		week  mode = 1
	)
	if got := Choose(true, month, week); got != month {
		t.Errorf("Choose(true) = %v, want %v", got, month)
	}
	if got := Choose(false, month, week); got != week {
		t.Errorf("Choose(false) = %v, want %v", got, week)
	}
}

// A constant takes the type of the list it is meant for.
func TestElementOfGivesAConstantItsListsElementType(t *testing.T) {
	days := []uint8{1, 2}
	next := ElementOf(days, 250)
	days = append(days, next)
	if len(days) != 3 || days[2] != 250 {
		t.Errorf("days = %v, want [1 2 250]", days)
	}
}

// The first failure is the one the caller learns of.
func TestTheFirstFailureIsKept(t *testing.T) {
	var f Failure
	DivInt32(&f, 1, 0)
	AddInt32(&f, math.MaxInt32, 1)
	if f.Err().Error() != "divide-by-zero" {
		t.Errorf("kept %v, want divide-by-zero", f.Err())
	}
}
