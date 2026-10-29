// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package algorithm

import "math"

// The two total rules of an integer operation (docs/adr/0063). An author who
// names one asks for the exact result e of add, sub, mul or neg brought into
// the operands' type T, of width W and range [m, M]:
//
//   - wrapping is the r in [m, M] with r = e (mod 2^W), the two's-complement
//     representative for a signed T;
//   - saturating is max(m, min(M, e)).
//
// Neither has a failure to record, so neither takes a Failure, and the
// generator calls them in any body, may-fail or not.

// Go defines + - * and unary - on typed integer operands as the wrapping
// operation at their width, for both signednesses (The Go Programming
// Language Specification, "Integer overflow"), so each wrapping helper is the
// operator. It is a function all the same: an expression of two constants is
// folded at compile time and refused when it overflows, and one family keeps
// the emitter uniform with the checked one.

func WrappingAddInt8(a, b int8) int8 { return a + b }
func WrappingSubInt8(a, b int8) int8 { return a - b }
func WrappingMulInt8(a, b int8) int8 { return a * b }
func WrappingNegInt8(a int8) int8    { return -a }

func WrappingAddInt16(a, b int16) int16 { return a + b }
func WrappingSubInt16(a, b int16) int16 { return a - b }
func WrappingMulInt16(a, b int16) int16 { return a * b }
func WrappingNegInt16(a int16) int16    { return -a }

func WrappingAddInt32(a, b int32) int32 { return a + b }
func WrappingSubInt32(a, b int32) int32 { return a - b }
func WrappingMulInt32(a, b int32) int32 { return a * b }
func WrappingNegInt32(a int32) int32    { return -a }

func WrappingAddInt64(a, b int64) int64 { return a + b }
func WrappingSubInt64(a, b int64) int64 { return a - b }
func WrappingMulInt64(a, b int64) int64 { return a * b }
func WrappingNegInt64(a int64) int64    { return -a }

func WrappingAddUint8(a, b uint8) uint8 { return a + b }
func WrappingSubUint8(a, b uint8) uint8 { return a - b }
func WrappingMulUint8(a, b uint8) uint8 { return a * b }
func WrappingNegUint8(a uint8) uint8    { return -a }

func WrappingAddUint16(a, b uint16) uint16 { return a + b }
func WrappingSubUint16(a, b uint16) uint16 { return a - b }
func WrappingMulUint16(a, b uint16) uint16 { return a * b }
func WrappingNegUint16(a uint16) uint16    { return -a }

func WrappingAddUint32(a, b uint32) uint32 { return a + b }
func WrappingSubUint32(a, b uint32) uint32 { return a - b }
func WrappingMulUint32(a, b uint32) uint32 { return a * b }
func WrappingNegUint32(a uint32) uint32    { return -a }

func WrappingAddUint64(a, b uint64) uint64 { return a + b }
func WrappingSubUint64(a, b uint64) uint64 { return a - b }
func WrappingMulUint64(a, b uint64) uint64 { return a * b }
func WrappingNegUint64(a uint64) uint64    { return -a }

type signedNarrow interface {
	~int8 | ~int16 | ~int32
}

type unsignedNarrow interface {
	~uint8 | ~uint16 | ~uint32
}

// A signed width up to 32 bits computes its exact result in int64, where none
// of these operations can overflow, and holds it to [lo, hi].
func saturateSigned[T signedNarrow](e, lo, hi int64) T { return T(max(lo, min(hi, e))) }

func saturatingAddSigned[T signedNarrow](a, b T, lo, hi int64) T {
	return saturateSigned[T](int64(a)+int64(b), lo, hi)
}

func saturatingSubSigned[T signedNarrow](a, b T, lo, hi int64) T {
	return saturateSigned[T](int64(a)-int64(b), lo, hi)
}

func saturatingMulSigned[T signedNarrow](a, b T, lo, hi int64) T {
	return saturateSigned[T](int64(a)*int64(b), lo, hi)
}

func saturatingNegSigned[T signedNarrow](a T, lo, hi int64) T {
	return saturateSigned[T](-int64(a), lo, hi)
}

// An unsigned width up to 32 bits computes a sum or a product exactly in
// uint64: neither is negative, and the product of two 32-bit values is below
// 2^64, where the same product in int64 would not be. A difference lies below
// the range exactly when the subtrahend is the larger.
func saturatingAddUnsigned[T unsignedNarrow](a, b T, hi uint64) T {
	return T(min(hi, uint64(a)+uint64(b)))
}

func saturatingMulUnsigned[T unsignedNarrow](a, b T, hi uint64) T {
	return T(min(hi, uint64(a)*uint64(b)))
}

func saturatingSubUnsigned[T unsignedNarrow](a, b T) T {
	if b > a {
		return 0
	}
	return a - b
}

func SaturatingAddInt8(a, b int8) int8 { return saturatingAddSigned(a, b, math.MinInt8, math.MaxInt8) }
func SaturatingSubInt8(a, b int8) int8 { return saturatingSubSigned(a, b, math.MinInt8, math.MaxInt8) }
func SaturatingMulInt8(a, b int8) int8 { return saturatingMulSigned(a, b, math.MinInt8, math.MaxInt8) }
func SaturatingNegInt8(a int8) int8    { return saturatingNegSigned(a, math.MinInt8, math.MaxInt8) }

func SaturatingAddInt16(a, b int16) int16 {
	return saturatingAddSigned(a, b, math.MinInt16, math.MaxInt16)
}
func SaturatingSubInt16(a, b int16) int16 {
	return saturatingSubSigned(a, b, math.MinInt16, math.MaxInt16)
}
func SaturatingMulInt16(a, b int16) int16 {
	return saturatingMulSigned(a, b, math.MinInt16, math.MaxInt16)
}
func SaturatingNegInt16(a int16) int16 { return saturatingNegSigned(a, math.MinInt16, math.MaxInt16) }

func SaturatingAddInt32(a, b int32) int32 {
	return saturatingAddSigned(a, b, math.MinInt32, math.MaxInt32)
}
func SaturatingSubInt32(a, b int32) int32 {
	return saturatingSubSigned(a, b, math.MinInt32, math.MaxInt32)
}
func SaturatingMulInt32(a, b int32) int32 {
	return saturatingMulSigned(a, b, math.MinInt32, math.MaxInt32)
}
func SaturatingNegInt32(a int32) int32 { return saturatingNegSigned(a, math.MinInt32, math.MaxInt32) }

func SaturatingAddUint8(a, b uint8) uint8 { return saturatingAddUnsigned(a, b, math.MaxUint8) }
func SaturatingSubUint8(a, b uint8) uint8 { return saturatingSubUnsigned(a, b) }
func SaturatingMulUint8(a, b uint8) uint8 { return saturatingMulUnsigned(a, b, math.MaxUint8) }

func SaturatingAddUint16(a, b uint16) uint16 { return saturatingAddUnsigned(a, b, math.MaxUint16) }
func SaturatingSubUint16(a, b uint16) uint16 { return saturatingSubUnsigned(a, b) }
func SaturatingMulUint16(a, b uint16) uint16 { return saturatingMulUnsigned(a, b, math.MaxUint16) }

func SaturatingAddUint32(a, b uint32) uint32 { return saturatingAddUnsigned(a, b, math.MaxUint32) }
func SaturatingSubUint32(a, b uint32) uint32 { return saturatingSubUnsigned(a, b) }
func SaturatingMulUint32(a, b uint32) uint32 { return saturatingMulUnsigned(a, b, math.MaxUint32) }

// 64 bits have no wider type, so each operation asks the checked family's
// bound test on which side of the range its exact result lies (algorithm.go)
// and answers that bound, or the operation's own result when it lies inside,
// where the wrapped result is the exact one.
func saturate64[T int64 | uint64](s side, lo, hi, wrapped T) T {
	switch s {
	case belowRange:
		return lo
	case aboveRange:
		return hi
	}
	return wrapped
}

func SaturatingAddInt64(a, b int64) int64 {
	return saturate64(addInt64Side(a, b), math.MinInt64, math.MaxInt64, a+b)
}

func SaturatingSubInt64(a, b int64) int64 {
	return saturate64(subInt64Side(a, b), math.MinInt64, math.MaxInt64, a-b)
}

func SaturatingMulInt64(a, b int64) int64 {
	return saturate64(mulInt64Side(a, b), math.MinInt64, math.MaxInt64, a*b)
}

// The one negation past the range is -MIN, which is M + 1.
func SaturatingNegInt64(a int64) int64 {
	if a == math.MinInt64 {
		return math.MaxInt64
	}
	return -a
}

func SaturatingAddUint64(a, b uint64) uint64 {
	return saturate64(addUint64Side(a, b), 0, math.MaxUint64, a+b)
}

func SaturatingSubUint64(a, b uint64) uint64 {
	return saturate64(subUint64Side(a, b), 0, math.MaxUint64, a-b)
}

func SaturatingMulUint64(a, b uint64) uint64 {
	return saturate64(mulUint64Side(a, b), 0, math.MaxUint64, a*b)
}
