// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package algorithm is the integer arithmetic contract's runtime half — see
// SCE_FORGE.md §3.4.1.
//
// An algorithm that declares `<sce:return may-fail="true">` returns
// `(value, error)`, and the generator lowers each of its integer `+ - * / %`
// and unary `-` to one of the Checked helpers. A helper computes the
// operation at the declared width, or records why it has no value there in
// the algorithm's Failure and yields 0; the generated body checks that record
// after every statement and returns it. No helper panics: a Go integer
// division by zero is a run-time panic, so it is refused before it happens,
// and an overflow — which Go would silently wrap — is refused too.
package algorithm

import "math"

// Error is why a may-fail algorithm has no value to return.
type Error int

const (
	// Overflow is an integer result outside its declared width.
	Overflow Error = iota
	// DivideByZero is an integer `/` or `%` by zero.
	DivideByZero
	// CapacityExceeded is a buffer append past its declared capacity. This
	// backend's buffers grow past their capacity (SCE_FORGE.md §4.12), so it
	// never reports one; the case exists because the failure has one name on
	// every backend.
	CapacityExceeded
)

// ContractName is the failure's name in the contract — the spelling every
// backend shares.
func (e Error) ContractName() string {
	switch e {
	case Overflow:
		return "overflow"
	case DivideByZero:
		return "divide-by-zero"
	case CapacityExceeded:
		return "capacity-exceeded"
	}
	return "overflow"
}

// Error makes the failure an error, so a caller reads it as Go reads every
// failure.
func (e Error) Error() string { return e.ContractName() }

// Failure is the first failure a body's checked operations recorded. Later
// ones are kept out: the statement that failed first is the one the caller
// learns of, whatever else the same expression evaluated after it.
type Failure struct {
	failed bool
	err    Error
}

// Fail records err unless a failure is already recorded.
func (f *Failure) Fail(err Error) {
	if !f.failed {
		f.failed = true
		f.err = err
	}
}

// Failed reports whether a failure is recorded.
func (f *Failure) Failed() bool { return f.failed }

// Err is the recorded failure, as an error; nil when there is none.
func (f *Failure) Err() error {
	if !f.failed {
		return nil
	}
	return f.err
}

// Take passes a call to another may-fail algorithm through the calling
// body's f: `Take[uint32](&sceFailure)(tick(n))` is the call's value, or 0
// with its failure recorded in f, which the statement around it returns.
// The value's type is spelled because Go infers none from f alone, and the
// call's two results reach the returned function as its two arguments.
//
// A generated algorithm's error is always an Error (its Failure.Err), so
// any other is a broken invariant, not a failure with a name to pass on.
func Take[T any](f *Failure) func(T, error) T {
	return func(value T, err error) T {
		if err == nil {
			return value
		}
		e, ok := err.(Error)
		if !ok {
			panic("scealgorithm.Take: a may-fail algorithm returned an error that is not an algorithm.Error: " + err.Error())
		}
		f.Fail(e)
		var zero T
		return zero
	}
}

type narrow interface {
	~int8 | ~int16 | ~int32 | ~uint8 | ~uint16 | ~uint32
}

// fit is v as a T when it lies in [lo, hi]; 0 and an overflow otherwise.
// Every width up to 32 bits computes exactly in int64, where none of these
// operations can overflow, and is then held to its own range. Division
// truncates toward zero and % takes the dividend's sign, as Go defines both.
func fit[T narrow](f *Failure, v, lo, hi int64) T {
	if v < lo || v > hi {
		f.Fail(Overflow)
		return 0
	}
	return T(v)
}

func addN[T narrow](f *Failure, a, b T, lo, hi int64) T { return fit[T](f, int64(a)+int64(b), lo, hi) }
func subN[T narrow](f *Failure, a, b T, lo, hi int64) T { return fit[T](f, int64(a)-int64(b), lo, hi) }
func mulN[T narrow](f *Failure, a, b T, lo, hi int64) T { return fit[T](f, int64(a)*int64(b), lo, hi) }
func negN[T narrow](f *Failure, a T, lo, hi int64) T    { return fit[T](f, -int64(a), lo, hi) }

func divN[T narrow](f *Failure, a, b T, lo, hi int64) T {
	if b == 0 {
		f.Fail(DivideByZero)
		return 0
	}
	return fit[T](f, int64(a)/int64(b), lo, hi)
}

// A remainder whose quotient overflows has no value either: MIN % -1 fails as
// MIN / -1 does, on every backend.
func remN[T narrow](f *Failure, a, b T, lo, hi int64) T {
	if b == 0 {
		f.Fail(DivideByZero)
		return 0
	}
	if fit[T](f, int64(a)/int64(b), lo, hi); f.failed {
		return 0
	}
	return T(int64(a) % int64(b))
}

func AddInt8(f *Failure, a, b int8) int8 { return addN(f, a, b, math.MinInt8, math.MaxInt8) }
func SubInt8(f *Failure, a, b int8) int8 { return subN(f, a, b, math.MinInt8, math.MaxInt8) }
func MulInt8(f *Failure, a, b int8) int8 { return mulN(f, a, b, math.MinInt8, math.MaxInt8) }
func DivInt8(f *Failure, a, b int8) int8 { return divN(f, a, b, math.MinInt8, math.MaxInt8) }
func RemInt8(f *Failure, a, b int8) int8 { return remN(f, a, b, math.MinInt8, math.MaxInt8) }
func NegInt8(f *Failure, a int8) int8    { return negN(f, a, math.MinInt8, math.MaxInt8) }

func AddInt16(f *Failure, a, b int16) int16 { return addN(f, a, b, math.MinInt16, math.MaxInt16) }
func SubInt16(f *Failure, a, b int16) int16 { return subN(f, a, b, math.MinInt16, math.MaxInt16) }
func MulInt16(f *Failure, a, b int16) int16 { return mulN(f, a, b, math.MinInt16, math.MaxInt16) }
func DivInt16(f *Failure, a, b int16) int16 { return divN(f, a, b, math.MinInt16, math.MaxInt16) }
func RemInt16(f *Failure, a, b int16) int16 { return remN(f, a, b, math.MinInt16, math.MaxInt16) }
func NegInt16(f *Failure, a int16) int16    { return negN(f, a, math.MinInt16, math.MaxInt16) }

func AddInt32(f *Failure, a, b int32) int32 { return addN(f, a, b, math.MinInt32, math.MaxInt32) }
func SubInt32(f *Failure, a, b int32) int32 { return subN(f, a, b, math.MinInt32, math.MaxInt32) }
func MulInt32(f *Failure, a, b int32) int32 { return mulN(f, a, b, math.MinInt32, math.MaxInt32) }
func DivInt32(f *Failure, a, b int32) int32 { return divN(f, a, b, math.MinInt32, math.MaxInt32) }
func RemInt32(f *Failure, a, b int32) int32 { return remN(f, a, b, math.MinInt32, math.MaxInt32) }
func NegInt32(f *Failure, a int32) int32    { return negN(f, a, math.MinInt32, math.MaxInt32) }

func AddUint8(f *Failure, a, b uint8) uint8 { return addN(f, a, b, 0, math.MaxUint8) }
func SubUint8(f *Failure, a, b uint8) uint8 { return subN(f, a, b, 0, math.MaxUint8) }
func MulUint8(f *Failure, a, b uint8) uint8 { return mulN(f, a, b, 0, math.MaxUint8) }
func DivUint8(f *Failure, a, b uint8) uint8 { return divN(f, a, b, 0, math.MaxUint8) }
func RemUint8(f *Failure, a, b uint8) uint8 { return remN(f, a, b, 0, math.MaxUint8) }
func NegUint8(f *Failure, a uint8) uint8    { return negN(f, a, 0, math.MaxUint8) }

func AddUint16(f *Failure, a, b uint16) uint16 { return addN(f, a, b, 0, math.MaxUint16) }
func SubUint16(f *Failure, a, b uint16) uint16 { return subN(f, a, b, 0, math.MaxUint16) }
func MulUint16(f *Failure, a, b uint16) uint16 { return mulN(f, a, b, 0, math.MaxUint16) }
func DivUint16(f *Failure, a, b uint16) uint16 { return divN(f, a, b, 0, math.MaxUint16) }
func RemUint16(f *Failure, a, b uint16) uint16 { return remN(f, a, b, 0, math.MaxUint16) }
func NegUint16(f *Failure, a uint16) uint16    { return negN(f, a, 0, math.MaxUint16) }

func AddUint32(f *Failure, a, b uint32) uint32 { return addN(f, a, b, 0, math.MaxUint32) }
func SubUint32(f *Failure, a, b uint32) uint32 { return subN(f, a, b, 0, math.MaxUint32) }
func MulUint32(f *Failure, a, b uint32) uint32 { return mulN(f, a, b, 0, math.MaxUint32) }
func DivUint32(f *Failure, a, b uint32) uint32 { return divN(f, a, b, 0, math.MaxUint32) }
func RemUint32(f *Failure, a, b uint32) uint32 { return remN(f, a, b, 0, math.MaxUint32) }
func NegUint32(f *Failure, a uint32) uint32    { return negN(f, a, 0, math.MaxUint32) }

// 64 bits have no wider type to compute in, so each operation tests its
// operands against the bounds before it runs (CERT INT32-C / INT30-C).

func AddInt64(f *Failure, a, b int64) int64 {
	if (b > 0 && a > math.MaxInt64-b) || (b < 0 && a < math.MinInt64-b) {
		f.Fail(Overflow)
		return 0
	}
	return a + b
}

func SubInt64(f *Failure, a, b int64) int64 {
	if (b < 0 && a > math.MaxInt64+b) || (b > 0 && a < math.MinInt64+b) {
		f.Fail(Overflow)
		return 0
	}
	return a - b
}

func MulInt64(f *Failure, a, b int64) int64 {
	if a == 0 || b == 0 {
		return 0
	}
	var overflow bool
	if a > 0 {
		if b > 0 {
			overflow = a > math.MaxInt64/b
		} else {
			overflow = b < math.MinInt64/a
		}
	} else {
		if b > 0 {
			overflow = a < math.MinInt64/b
		} else {
			overflow = b < math.MaxInt64/a
		}
	}
	if overflow {
		f.Fail(Overflow)
		return 0
	}
	return a * b
}

func DivInt64(f *Failure, a, b int64) int64 {
	if b == 0 {
		f.Fail(DivideByZero)
		return 0
	}
	if a == math.MinInt64 && b == -1 {
		f.Fail(Overflow)
		return 0
	}
	return a / b
}

func RemInt64(f *Failure, a, b int64) int64 {
	if b == 0 {
		f.Fail(DivideByZero)
		return 0
	}
	if a == math.MinInt64 && b == -1 {
		f.Fail(Overflow)
		return 0
	}
	return a % b
}

func NegInt64(f *Failure, a int64) int64 {
	if a == math.MinInt64 {
		f.Fail(Overflow)
		return 0
	}
	return -a
}

func AddUint64(f *Failure, a, b uint64) uint64 {
	if a > math.MaxUint64-b {
		f.Fail(Overflow)
		return 0
	}
	return a + b
}

func SubUint64(f *Failure, a, b uint64) uint64 {
	if b > a {
		f.Fail(Overflow)
		return 0
	}
	return a - b
}

func MulUint64(f *Failure, a, b uint64) uint64 {
	if a != 0 && b > math.MaxUint64/a {
		f.Fail(Overflow)
		return 0
	}
	return a * b
}

func DivUint64(f *Failure, a, b uint64) uint64 {
	if b == 0 {
		f.Fail(DivideByZero)
		return 0
	}
	return a / b
}

func RemUint64(f *Failure, a, b uint64) uint64 {
	if b == 0 {
		f.Fail(DivideByZero)
		return 0
	}
	return a % b
}

func NegUint64(f *Failure, a uint64) uint64 {
	if a != 0 {
		f.Fail(Overflow)
		return 0
	}
	return a
}

// A value stored where a narrower integer type is declared is the same
// value, or 0 and an overflow when the type cannot hold it — never a
// wrapped one. The value arrives as int64 (from a signed type) or uint64
// (from an unsigned one), either of which holds it exactly, so one helper
// per target and signedness serves every source width. A pair whose every
// value fits (int64 from signed, uint64 from unsigned) has no helper: the
// generator emits no check there.

func narrowFromInt[T narrow](f *Failure, v, lo, hi int64) T { return fit[T](f, v, lo, hi) }

func narrowFromUint[T narrow](f *Failure, v uint64, hi int64) T {
	if v > uint64(hi) {
		f.Fail(Overflow)
		return 0
	}
	return T(v)
}

func NarrowInt8FromInt(f *Failure, v int64) int8 {
	return narrowFromInt[int8](f, v, math.MinInt8, math.MaxInt8)
}
func NarrowInt8FromUint(f *Failure, v uint64) int8 { return narrowFromUint[int8](f, v, math.MaxInt8) }
func NarrowInt16FromInt(f *Failure, v int64) int16 {
	return narrowFromInt[int16](f, v, math.MinInt16, math.MaxInt16)
}
func NarrowInt16FromUint(f *Failure, v uint64) int16 {
	return narrowFromUint[int16](f, v, math.MaxInt16)
}
func NarrowInt32FromInt(f *Failure, v int64) int32 {
	return narrowFromInt[int32](f, v, math.MinInt32, math.MaxInt32)
}
func NarrowInt32FromUint(f *Failure, v uint64) int32 {
	return narrowFromUint[int32](f, v, math.MaxInt32)
}
func NarrowUint8FromInt(f *Failure, v int64) uint8 {
	return narrowFromInt[uint8](f, v, 0, math.MaxUint8)
}
func NarrowUint8FromUint(f *Failure, v uint64) uint8 {
	return narrowFromUint[uint8](f, v, math.MaxUint8)
}
func NarrowUint16FromInt(f *Failure, v int64) uint16 {
	return narrowFromInt[uint16](f, v, 0, math.MaxUint16)
}
func NarrowUint16FromUint(f *Failure, v uint64) uint16 {
	return narrowFromUint[uint16](f, v, math.MaxUint16)
}
func NarrowUint32FromInt(f *Failure, v int64) uint32 {
	return narrowFromInt[uint32](f, v, 0, math.MaxUint32)
}
func NarrowUint32FromUint(f *Failure, v uint64) uint32 {
	return narrowFromUint[uint32](f, v, math.MaxUint32)
}

func NarrowInt64FromUint(f *Failure, v uint64) int64 {
	if v > math.MaxInt64 {
		f.Fail(Overflow)
		return 0
	}
	return int64(v)
}

func NarrowUint64FromInt(f *Failure, v int64) uint64 {
	if v < 0 {
		f.Fail(Overflow)
		return 0
	}
	return uint64(v)
}
