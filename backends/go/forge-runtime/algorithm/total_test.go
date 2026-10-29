// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package algorithm

import (
	"math"
	"testing"
)

// totalCase is one operation of a type on its operands (b unread for neg),
// with the value each rule gives it. The values are worked from the
// definition (docs/adr/0063, section 1): wrap is the exact result moved by a
// multiple of 2^W into [m, M], sat is the exact result held to [m, M].
type totalCase[T integer] struct {
	op        string
	a, b      T
	wrap, sat T
}

type integer interface {
	~int8 | ~int16 | ~int32 | ~int64 | ~uint8 | ~uint16 | ~uint32 | ~uint64
}

type binary[T integer] func(a, b T) T

// totalFamily is one type's helpers under each rule. An unsigned type has no
// saturating negation, so its saturating table has no "neg", and a case of
// any other operation the table lacks is a broken table, not a skip.
type totalFamily[T integer] struct {
	wrapping, saturating map[string]binary[T]
}

func unary[T integer](neg func(T) T) binary[T] { return func(a, _ T) T { return neg(a) } }

func checkTotal[T integer](t *testing.T, typ string, family totalFamily[T], cases []totalCase[T]) {
	t.Helper()
	for _, c := range cases {
		wrap, ok := family.wrapping[c.op]
		if !ok {
			t.Fatalf("%s: no wrapping %s", typ, c.op)
		}
		if got := wrap(c.a, c.b); got != c.wrap {
			t.Errorf("%s wrapping %s(%d, %d) = %d, want %d", typ, c.op, c.a, c.b, got, c.wrap)
		}
		sat, ok := family.saturating[c.op]
		if !ok {
			if c.op != "neg" {
				t.Fatalf("%s: no saturating %s", typ, c.op)
			}
			continue
		}
		if got := sat(c.a, c.b); got != c.sat {
			t.Errorf("%s saturating %s(%d, %d) = %d, want %d", typ, c.op, c.a, c.b, got, c.sat)
		}
	}
}

// signedCases are the cases every signed type shares, at its own m and M
// (passed as int64 and converted, so one list serves every width): the wrap
// point and the saturation point on each side with their neighbours inside,
// MIN * -1, -MIN and MIN - 1, and the products past 2^W but within 2^(2W)
// that M * M, m * m and M * m make.
func signedCases[T ~int8 | ~int16 | ~int32 | ~int64](m, mx T) []totalCase[T] {
	return []totalCase[T]{
		{"add", mx, 1, m, mx},
		{"add", mx - 1, 1, mx, mx},
		{"add", m, -1, mx, m},
		{"add", m + 1, -1, m, m},
		{"add", mx, mx, -2, mx},
		{"add", m, m, 0, m},
		{"add", mx, m, -1, -1},
		{"add", 0, 0, 0, 0},
		{"add", -1, 1, 0, 0},
		{"sub", m, 1, mx, m},
		{"sub", m + 1, 1, m, m},
		{"sub", mx, -1, m, mx},
		{"sub", mx - 1, -1, mx, mx},
		{"sub", 0, m, m, mx},
		{"sub", -1, m, mx, mx},
		{"sub", m, mx, 1, m},
		{"sub", mx, m, -1, mx},
		{"sub", 0, 1, -1, -1},
		{"sub", mx, mx, 0, 0},
		{"mul", m, -1, m, mx},
		{"mul", -1, m, m, mx},
		{"mul", m + 1, -1, mx, mx},
		{"mul", mx, 2, -2, mx},
		{"mul", m, 2, 0, m},
		{"mul", mx, mx, 1, mx},
		{"mul", m, m, 0, mx},
		{"mul", mx, m, m, m},
		{"mul", m, 1, m, m},
		{"mul", 1, mx, mx, mx},
		{"mul", -1, -1, 1, 1},
		{"mul", 0, m, 0, 0},
		{"neg", m, 0, m, mx},
		{"neg", m + 1, 0, mx, mx},
		{"neg", mx, 0, m + 1, m + 1},
		{"neg", 0, 0, 0, 0},
		{"neg", 1, 0, -1, -1},
		{"neg", -1, 0, 1, 1},
	}
}

// unsignedCases are the cases every unsigned type shares, at its own M: the
// wrap point and the saturation point at each end, and M * M, which is
// 2^(2W) - 2^(W+1) + 1 and so 1 modulo 2^W.
func unsignedCases[T ~uint8 | ~uint16 | ~uint32 | ~uint64](mx T) []totalCase[T] {
	half := mx/2 + 1 // 2^(W-1)
	return []totalCase[T]{
		{"add", mx, 1, 0, mx},
		{"add", mx - 1, 1, mx, mx},
		{"add", 1, mx - 1, mx, mx},
		{"add", mx, mx, mx - 1, mx},
		{"add", 0, 0, 0, 0},
		{"sub", 0, 1, mx, 0},
		{"sub", 1, 1, 0, 0},
		{"sub", 0, mx, 1, 0},
		{"sub", mx - 1, mx, mx, 0},
		{"sub", mx, mx, 0, 0},
		{"sub", mx, 0, mx, mx},
		{"mul", mx, mx, 1, mx},
		{"mul", mx, 2, mx - 1, mx},
		{"mul", half, 2, 0, mx},
		{"mul", 1, mx, mx, mx},
		{"mul", 0, mx, 0, 0},
		{"neg", 0, 0, 0, 0},
		{"neg", 1, 0, mx, 0},
		{"neg", mx, 0, 1, 0},
		{"neg", half, 0, half, 0},
	}
}

func TestWrappingAndSaturatingInt8(t *testing.T) {
	family := totalFamily[int8]{
		wrapping: map[string]binary[int8]{
			"add": WrappingAddInt8, "sub": WrappingSubInt8, "mul": WrappingMulInt8, "neg": unary(WrappingNegInt8),
		},
		saturating: map[string]binary[int8]{
			"add": SaturatingAddInt8, "sub": SaturatingSubInt8, "mul": SaturatingMulInt8, "neg": unary(SaturatingNegInt8),
		},
	}
	checkTotal(t, "int8", family, signedCases[int8](math.MinInt8, math.MaxInt8))
	checkTotal(t, "int8", family, []totalCase[int8]{
		{"mul", 17, 15, -1, math.MaxInt8},
		{"mul", -17, 15, 1, math.MinInt8},
		{"mul", 16, 16, 0, math.MaxInt8},
		{"add", 100, 100, -56, math.MaxInt8},
	})
}

func TestWrappingAndSaturatingInt16(t *testing.T) {
	family := totalFamily[int16]{
		wrapping: map[string]binary[int16]{
			"add": WrappingAddInt16, "sub": WrappingSubInt16, "mul": WrappingMulInt16, "neg": unary(WrappingNegInt16),
		},
		saturating: map[string]binary[int16]{
			"add": SaturatingAddInt16, "sub": SaturatingSubInt16, "mul": SaturatingMulInt16, "neg": unary(SaturatingNegInt16),
		},
	}
	checkTotal(t, "int16", family, signedCases[int16](math.MinInt16, math.MaxInt16))
	checkTotal(t, "int16", family, []totalCase[int16]{
		{"mul", 257, 255, -1, math.MaxInt16},
		{"mul", -257, 255, 1, math.MinInt16},
		{"mul", 256, 256, 0, math.MaxInt16},
	})
}

func TestWrappingAndSaturatingInt32(t *testing.T) {
	family := totalFamily[int32]{
		wrapping: map[string]binary[int32]{
			"add": WrappingAddInt32, "sub": WrappingSubInt32, "mul": WrappingMulInt32, "neg": unary(WrappingNegInt32),
		},
		saturating: map[string]binary[int32]{
			"add": SaturatingAddInt32, "sub": SaturatingSubInt32, "mul": SaturatingMulInt32, "neg": unary(SaturatingNegInt32),
		},
	}
	checkTotal(t, "int32", family, signedCases[int32](math.MinInt32, math.MaxInt32))
	checkTotal(t, "int32", family, []totalCase[int32]{
		{"mul", 65537, 65535, -1, math.MaxInt32},
		{"mul", -65537, 65535, 1, math.MinInt32},
		{"mul", 65536, 65536, 0, math.MaxInt32},
	})
}

// At 64 bits the saturating helpers test their operands against the bounds,
// so the cases also hold a product just inside the range beside one just past
// it: 3037000499^2 is below 2^63 - 1 and 3037000500^2 above it.
func TestWrappingAndSaturatingInt64(t *testing.T) {
	family := totalFamily[int64]{
		wrapping: map[string]binary[int64]{
			"add": WrappingAddInt64, "sub": WrappingSubInt64, "mul": WrappingMulInt64, "neg": unary(WrappingNegInt64),
		},
		saturating: map[string]binary[int64]{
			"add": SaturatingAddInt64, "sub": SaturatingSubInt64, "mul": SaturatingMulInt64, "neg": unary(SaturatingNegInt64),
		},
	}
	checkTotal(t, "int64", family, signedCases[int64](math.MinInt64, math.MaxInt64))
	checkTotal(t, "int64", family, []totalCase[int64]{
		{"mul", 4294967297, 4294967295, -1, math.MaxInt64},
		{"mul", -4294967297, 4294967295, 1, math.MinInt64},
		{"mul", 4294967296, 4294967296, 0, math.MaxInt64},
		{"mul", 3037000499, 3037000499, 9223372030926249001, 9223372030926249001},
		{"mul", 3037000500, 3037000500, -9223372036709301616, math.MaxInt64},
		{"mul", -3037000500, 3037000500, 9223372036709301616, math.MinInt64},
		{"mul", 3037000500, -3037000500, 9223372036709301616, math.MinInt64},
		{"mul", -3037000500, -3037000500, -9223372036709301616, math.MaxInt64},
	})
}

func TestWrappingAndSaturatingUint8(t *testing.T) {
	family := totalFamily[uint8]{
		wrapping: map[string]binary[uint8]{
			"add": WrappingAddUint8, "sub": WrappingSubUint8, "mul": WrappingMulUint8, "neg": unary(WrappingNegUint8),
		},
		saturating: map[string]binary[uint8]{
			"add": SaturatingAddUint8, "sub": SaturatingSubUint8, "mul": SaturatingMulUint8,
		},
	}
	checkTotal(t, "uint8", family, unsignedCases[uint8](math.MaxUint8))
	checkTotal(t, "uint8", family, []totalCase[uint8]{
		{"mul", 16, 16, 0, math.MaxUint8},
		{"mul", 17, 15, 255, 255},
		{"mul", 17, 17, 33, math.MaxUint8},
	})
}

func TestWrappingAndSaturatingUint16(t *testing.T) {
	family := totalFamily[uint16]{
		wrapping: map[string]binary[uint16]{
			"add": WrappingAddUint16, "sub": WrappingSubUint16, "mul": WrappingMulUint16, "neg": unary(WrappingNegUint16),
		},
		saturating: map[string]binary[uint16]{
			"add": SaturatingAddUint16, "sub": SaturatingSubUint16, "mul": SaturatingMulUint16,
		},
	}
	checkTotal(t, "uint16", family, unsignedCases[uint16](math.MaxUint16))
	checkTotal(t, "uint16", family, []totalCase[uint16]{
		{"mul", 256, 256, 0, math.MaxUint16},
		{"mul", 257, 255, 65535, 65535},
		{"mul", 257, 257, 513, math.MaxUint16},
	})
}

// A product of two uint32 values past 2^63 is the case computing in int64
// would get wrong: (2^32 - 1)^2 saturates to M, not to 0.
func TestWrappingAndSaturatingUint32(t *testing.T) {
	family := totalFamily[uint32]{
		wrapping: map[string]binary[uint32]{
			"add": WrappingAddUint32, "sub": WrappingSubUint32, "mul": WrappingMulUint32, "neg": unary(WrappingNegUint32),
		},
		saturating: map[string]binary[uint32]{
			"add": SaturatingAddUint32, "sub": SaturatingSubUint32, "mul": SaturatingMulUint32,
		},
	}
	checkTotal(t, "uint32", family, unsignedCases[uint32](math.MaxUint32))
	checkTotal(t, "uint32", family, []totalCase[uint32]{
		{"mul", 65536, 65536, 0, math.MaxUint32},
		{"mul", 65537, 65535, 4294967295, 4294967295},
		{"mul", 65537, 65537, 131073, math.MaxUint32},
	})
}

func TestWrappingAndSaturatingUint64(t *testing.T) {
	family := totalFamily[uint64]{
		wrapping: map[string]binary[uint64]{
			"add": WrappingAddUint64, "sub": WrappingSubUint64, "mul": WrappingMulUint64, "neg": unary(WrappingNegUint64),
		},
		saturating: map[string]binary[uint64]{
			"add": SaturatingAddUint64, "sub": SaturatingSubUint64, "mul": SaturatingMulUint64,
		},
	}
	checkTotal(t, "uint64", family, unsignedCases[uint64](math.MaxUint64))
	checkTotal(t, "uint64", family, []totalCase[uint64]{
		{"mul", 4294967296, 4294967296, 0, math.MaxUint64},
		{"mul", 4294967297, 4294967295, math.MaxUint64, math.MaxUint64},
		{"mul", 4294967297, 4294967297, 8589934593, math.MaxUint64},
	})
}

// The checked family's 64-bit helpers read the same bound tests as the
// saturating ones: a result just past either bound fails, one at the bound
// passes.
func TestTheChecked64BitBoundsMatchTheSaturatingOnes(t *testing.T) {
	cases := []struct {
		name string
		run  func(*Failure) int64
		want int64
		fail bool
	}{
		{"MIN - 1", func(f *Failure) int64 { return SubInt64(f, math.MinInt64, 1) }, 0, true},
		{"MIN + -1", func(f *Failure) int64 { return AddInt64(f, math.MinInt64, -1) }, 0, true},
		{"MAX - -1", func(f *Failure) int64 { return SubInt64(f, math.MaxInt64, -1) }, 0, true},
		{"MIN * -1", func(f *Failure) int64 { return MulInt64(f, math.MinInt64, -1) }, 0, true},
		{"(MIN + 1) * -1", func(f *Failure) int64 { return MulInt64(f, math.MinInt64+1, -1) }, math.MaxInt64, false},
		{"MIN * 1", func(f *Failure) int64 { return MulInt64(f, math.MinInt64, 1) }, math.MinInt64, false},
		{"0 - MIN", func(f *Failure) int64 { return SubInt64(f, 0, math.MinInt64) }, 0, true},
		{"-1 - MIN", func(f *Failure) int64 { return SubInt64(f, -1, math.MinInt64) }, math.MaxInt64, false},
	}
	for _, c := range cases {
		var f Failure
		got := c.run(&f)
		if f.Failed() != c.fail || got != c.want {
			t.Errorf("%s: got %d (failed %v), want %d (failed %v)", c.name, got, f.Failed(), c.want, c.fail)
		}
	}
	var f Failure
	if got := AddUint64(&f, math.MaxUint64, 1); !f.Failed() || got != 0 {
		t.Errorf("MAX + 1: got %d (failed %v), want an overflow", got, f.Failed())
	}
	f = Failure{}
	if got := MulUint64(&f, 4294967297, 4294967295); f.Failed() || got != math.MaxUint64 {
		t.Errorf("(2^32 + 1)(2^32 - 1): got %d (failed %v), want MAX", got, f.Failed())
	}
	f = Failure{}
	if got := SubUint64(&f, 0, 1); !f.Failed() || got != 0 {
		t.Errorf("0 - 1: got %d (failed %v), want an overflow", got, f.Failed())
	}
}
