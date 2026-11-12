// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The integer arithmetic contract's runtime half: see SCE_FORGE.md §3.4.1.
//
// A failure is not a value (docs/adr/0098): an operation the contract says has no
// result throws its `AlgorithmError` and answers nothing, so nothing after it in the
// expression is evaluated and no value of any type stands in for it. An algorithm
// that declares `<sce:return may-fail="true">` is `throws(AlgorithmError)`, and the
// generator lowers each of its integer `+ - * / %` and unary `-` to one of the
// `Checked` helpers, called with `try`; a machine catches the failure where the
// statement stands and raises `error.execution`. No helper traps: Swift traps on an
// overflow and on a division by zero, so each is refused before it happens.
//
// Swift's own operators are not used for the checked operations because they trap,
// and the wrapping ones (`&+`) answer a value the contract says does not exist. The
// standard library's `...ReportingOverflow` operations compute the exact answer or
// say it does not fit, at any width, 64 bits included, which is what the contract
// asks of each backend.
//
// Cross-language siblings: `algorithm` (Go), `com.sce.forge.runtime.SceChecked`
// (Kotlin) and `sce_forge_runtime` (Rust).

/// Why a may-fail algorithm has no value to return.
public enum AlgorithmError: Error, Sendable, Equatable, CustomStringConvertible {
    /// An integer result outside its declared width.
    case overflow

    /// An integer `/` or `%` by zero.
    case divideByZero

    /// A buffer append past its declared capacity.
    case capacityExceeded

    /// A `<sce:require>` precondition that does not hold: an input outside the
    /// algorithm's domain.
    case precondition

    /// An index below 0 or not below its collection's length.
    case outOfRange

    /// The failure's name in the contract: the spelling every backend shares.
    public var contractName: String {
        switch self {
        case .overflow: return "overflow"
        case .divideByZero: return "divide-by-zero"
        case .capacityExceeded: return "capacity-exceeded"
        case .precondition: return "precondition"
        case .outOfRange: return "out-of-range"
        }
    }

    public var description: String {
        contractName
    }
}

/// The checked integer operations, by the width of the type they are called at.
public enum Checked {
    /// `a + b` at the width of `T`, or an overflow.
    public static func add<T: FixedWidthInteger>(_ a: T, _ b: T) throws(AlgorithmError) -> T {
        let (value, overflow) = a.addingReportingOverflow(b)
        if overflow {
            throw .overflow
        }
        return value
    }

    /// `a - b` at the width of `T`, or an overflow.
    public static func sub<T: FixedWidthInteger>(_ a: T, _ b: T) throws(AlgorithmError) -> T {
        let (value, overflow) = a.subtractingReportingOverflow(b)
        if overflow {
            throw .overflow
        }
        return value
    }

    /// `a * b` at the width of `T`, or an overflow.
    public static func mul<T: FixedWidthInteger>(_ a: T, _ b: T) throws(AlgorithmError) -> T {
        let (value, overflow) = a.multipliedReportingOverflow(by: b)
        if overflow {
            throw .overflow
        }
        return value
    }

    /// `a / b` truncated toward zero, or a division by zero, or an overflow for
    /// `MIN / -1`.
    public static func div<T: FixedWidthInteger>(_ a: T, _ b: T) throws(AlgorithmError) -> T {
        if b == 0 {
            throw .divideByZero
        }
        let (value, overflow) = a.dividedReportingOverflow(by: b)
        if overflow {
            throw .overflow
        }
        return value
    }

    /// `a % b` with the dividend's sign, or a division by zero. A remainder whose
    /// quotient overflows has no value either: `MIN % -1` fails as `MIN / -1` does, on
    /// every backend.
    public static func rem<T: FixedWidthInteger>(_ a: T, _ b: T) throws(AlgorithmError) -> T {
        if b == 0 {
            throw .divideByZero
        }
        if a.dividedReportingOverflow(by: b).overflow {
            throw .overflow
        }
        return a.remainderReportingOverflow(dividingBy: b).partialValue
    }

    /// `-a` at the width of `T`, or an overflow.
    public static func neg<T: FixedWidthInteger>(_ a: T) throws(AlgorithmError) -> T {
        let (value, overflow) = T.zero.subtractingReportingOverflow(a)
        if overflow {
            throw .overflow
        }
        return value
    }

    /// A value stored where a narrower integer type is declared is the same value, or
    /// an overflow when the type cannot hold it: never a wrapped one.
    public static func narrow<From: BinaryInteger, To: FixedWidthInteger>(
        _ value: From,
        to type: To.Type
    ) throws(AlgorithmError) -> To {
        guard let narrowed = To(exactly: value) else {
            throw .overflow
        }
        return narrowed
    }

    /// `value` when it holds no more than `capacity` bytes (the bound a string
    /// variable of a sce-static machine declares, counted in UTF-8 bytes), and a
    /// capacity failure otherwise.
    ///
    /// Counted in bytes, as every backend counts them, so a machine holds the same
    /// value wherever it runs. A Swift `String.count` is grapheme clusters, which two
    /// characters of four bytes each would make two against a bound of four.
    public static func bounded(_ value: String, capacity: UInt32) throws(AlgorithmError) -> String {
        if UInt64(value.utf8.count) > UInt64(capacity) {
            throw .capacityExceeded
        }
        return value
    }

    /// `bounded` for a byte string: `value` when it holds no more than `capacity`
    /// bytes, and a capacity failure otherwise (docs/adr/0005, decision 2).
    public static func bounded(_ value: [UInt8], capacity: UInt32) throws(AlgorithmError) -> [UInt8] {
        if UInt64(value.count) > UInt64(capacity) {
            throw .capacityExceeded
        }
        return value
    }

    /// `items` when it holds no more than `capacity` elements (the bound a list
    /// variable of a sce-static machine declares), and a capacity failure otherwise.
    /// A list an algorithm returns may hold more than the bound it declared on a
    /// backend whose lists grow, and a machine holds the same list wherever it runs,
    /// so the list it takes is held to its own.
    public static func within<T>(_ items: [T], capacity: UInt32) throws(AlgorithmError) -> [T] {
        if UInt64(items.count) > UInt64(capacity) {
            throw .capacityExceeded
        }
        return items
    }

    /// The element of `xs` at `i`, or an out-of-range failure when `i` is below 0 or
    /// not below the length: never a trap, and no value of `T` in its place, so an
    /// element of any type is read alike (a union's included, docs/adr/0098).
    public static func at<T, I: BinaryInteger>(_ xs: [T], _ i: I) throws(AlgorithmError) -> T {
        guard let index = Int(exactly: i), index >= 0, index < xs.count else {
            throw .outOfRange
        }
        return xs[index]
    }

    /// `value` as an element of `list`, which gives a literal the element's type: the
    /// value an append stores is computed into a local before it is appended, and a
    /// local of a literal alone would be an `Int`.
    public static func element<T>(of list: [T], _ value: T) -> T {
        value
    }

    /// `value`, or an out-of-range failure for a reading that found nothing: the
    /// shape an enum's `fromUnderlying` answers (a number no variant declares).
    public static func found<T>(_ value: T?) throws(AlgorithmError) -> T {
        guard let value else {
            throw .outOfRange
        }
        return value
    }

    /// `lhs && rhs` where `rhs` can fail: `rhs` is evaluated only where `lhs` holds,
    /// and its failure is the expression's. Spelled here rather than through the
    /// standard library's `&&`, whose `rethrows` need not keep the failure's type.
    public static func and(
        _ lhs: Bool,
        _ rhs: () throws(AlgorithmError) -> Bool
    ) throws(AlgorithmError) -> Bool {
        if !lhs {
            return false
        }
        return try rhs()
    }

    /// `lhs || rhs` where `rhs` can fail: `rhs` is evaluated only where `lhs` does not
    /// hold.
    public static func or(
        _ lhs: Bool,
        _ rhs: () throws(AlgorithmError) -> Bool
    ) throws(AlgorithmError) -> Bool {
        if lhs {
            return true
        }
        return try rhs()
    }

    /// `value ?? fallback` where `fallback` can fail: `fallback` is evaluated only
    /// where `value` is absent (docs/adr/0055, section 2).
    public static func coalesce<T>(
        _ value: T?,
        _ fallback: () throws(AlgorithmError) -> T
    ) throws(AlgorithmError) -> T {
        if let value {
            return value
        }
        return try fallback()
    }

    /// `coalesce` where the fallback may be absent too, so the whole is.
    public static func coalesce<T>(
        _ value: T?,
        _ fallback: () throws(AlgorithmError) -> T?
    ) throws(AlgorithmError) -> T? {
        if let value {
            return value
        }
        return try fallback()
    }
}
