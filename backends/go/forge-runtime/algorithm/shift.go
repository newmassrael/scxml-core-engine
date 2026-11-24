// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package algorithm

// A shift (SCE_FORGE.md §3.4.1, "The width of a shift"): for a type T of width
// W and a count n at least 0, the exact result reduced to W, which has a value
// on every such count. x << n is x * 2^n reduced to W, so 0 from n = W; x >> n
// is the floor of x / 2^n, so 0 from n = W, or -1 for a negative x; x >>> n is
// the floor of u / 2^n, u the W-bit unsigned pattern of x.
//
// Go's own << and >> compute exactly that for a count of any integer type (The
// Go Programming Language Specification, "Arithmetic operators": the shifts
// behave as if the left operand is shifted n times by 1), so the generator
// writes the operator for a count it knows is at least 0. A constant count at
// or past the width is the one it cannot write so, since `go vet` reports the
// shift as too large for its type: WrappingShl and WrappingShr take it, and
// compute the same. A count below 0 has no exact result, and Go panics on one:
// the checked family below records it as out of range instead.

// fixedWidth is the eight integer types a document declares.
type fixedWidth interface {
	~int8 | ~int16 | ~int32 | ~int64 | ~uint8 | ~uint16 | ~uint32 | ~uint64
}

// WrappingShl is a << n, 0 for a count at or past the width.
func WrappingShl[T fixedWidth](a T, n uint64) T { return a << n }

// WrappingShr is a >> n, 0 or -1 for a count at or past the width.
func WrappingShr[T fixedWidth](a T, n uint64) T { return a >> n }

// places is the count n as a shift's number of places, or false with an
// out-of-range failure recorded in f for a count below 0.
func places(f *Failure, n int64) (uint64, bool) {
	if n < 0 {
		f.Fail(OutOfRange)
		return 0, false
	}
	return uint64(n), true
}

func shlChecked[T fixedWidth](f *Failure, a T, n int64) T {
	c, ok := places(f, n)
	if !ok {
		return 0
	}
	return a << c
}

func shrChecked[T fixedWidth](f *Failure, a T, n int64) T {
	c, ok := places(f, n)
	if !ok {
		return 0
	}
	return a >> c
}

// A logical shift of a signed T is made on the bits of U, the unsigned type of
// its width.
func ushrChecked[T fixedWidth, U fixedWidth](f *Failure, a T, n int64) T {
	c, ok := places(f, n)
	if !ok {
		return 0
	}
	return T(U(a) >> c)
}

// The checked shifts of a may-fail body: the shift of each width, or 0 with an
// out-of-range failure recorded in f for a count below 0, which the statement
// around it returns. The count arrives as int64.

func ShlInt8(f *Failure, a int8, n int64) int8    { return shlChecked(f, a, n) }
func ShrInt8(f *Failure, a int8, n int64) int8    { return shrChecked(f, a, n) }
func UshrInt8(f *Failure, a int8, n int64) int8   { return ushrChecked[int8, uint8](f, a, n) }
func ShlInt16(f *Failure, a int16, n int64) int16 { return shlChecked(f, a, n) }
func ShrInt16(f *Failure, a int16, n int64) int16 { return shrChecked(f, a, n) }
func UshrInt16(f *Failure, a int16, n int64) int16 {
	return ushrChecked[int16, uint16](f, a, n)
}
func ShlInt32(f *Failure, a int32, n int64) int32 { return shlChecked(f, a, n) }
func ShrInt32(f *Failure, a int32, n int64) int32 { return shrChecked(f, a, n) }
func UshrInt32(f *Failure, a int32, n int64) int32 {
	return ushrChecked[int32, uint32](f, a, n)
}
func ShlInt64(f *Failure, a int64, n int64) int64 { return shlChecked(f, a, n) }
func ShrInt64(f *Failure, a int64, n int64) int64 { return shrChecked(f, a, n) }
func UshrInt64(f *Failure, a int64, n int64) int64 {
	return ushrChecked[int64, uint64](f, a, n)
}

func ShlUint8(f *Failure, a uint8, n int64) uint8     { return shlChecked(f, a, n) }
func ShrUint8(f *Failure, a uint8, n int64) uint8     { return shrChecked(f, a, n) }
func UshrUint8(f *Failure, a uint8, n int64) uint8    { return shrChecked(f, a, n) }
func ShlUint16(f *Failure, a uint16, n int64) uint16  { return shlChecked(f, a, n) }
func ShrUint16(f *Failure, a uint16, n int64) uint16  { return shrChecked(f, a, n) }
func UshrUint16(f *Failure, a uint16, n int64) uint16 { return shrChecked(f, a, n) }
func ShlUint32(f *Failure, a uint32, n int64) uint32  { return shlChecked(f, a, n) }
func ShrUint32(f *Failure, a uint32, n int64) uint32  { return shrChecked(f, a, n) }
func UshrUint32(f *Failure, a uint32, n int64) uint32 { return shrChecked(f, a, n) }
func ShlUint64(f *Failure, a uint64, n int64) uint64  { return shlChecked(f, a, n) }
func ShrUint64(f *Failure, a uint64, n int64) uint64  { return shrChecked(f, a, n) }
func UshrUint64(f *Failure, a uint64, n int64) uint64 { return shrChecked(f, a, n) }
