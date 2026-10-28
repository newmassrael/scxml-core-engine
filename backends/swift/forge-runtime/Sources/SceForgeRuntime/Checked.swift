// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The integer arithmetic contract's runtime half: see SCE_FORGE.md §3.4.1.
//
// An algorithm that declares `<sce:return may-fail="true">` throws, and the generator
// lowers each of its integer `+ - * / %` and unary `-` to one of the `Checked`
// helpers. A helper computes the operation at the declared width, or records why it
// has no value there in the algorithm's `Failure` and yields 0; the generated body
// checks that record after every statement and throws it. No helper traps: Swift
// traps on an overflow and on a division by zero, so each is refused before it
// happens.
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

/// The first failure a body's checked operations recorded. Later ones are kept out:
/// the statement that failed first is the one the caller learns of, whatever else the
/// same expression evaluated after it.
public struct Failure: Sendable {
    private var recorded: AlgorithmError?

    public init() {}

    /// Records `error` unless a failure is already recorded.
    public mutating func fail(_ error: AlgorithmError) {
        if recorded == nil {
            recorded = error
        }
    }

    /// Whether a failure is recorded.
    public var failed: Bool {
        recorded != nil
    }

    /// The recorded failure; nil when there is none.
    public var error: AlgorithmError? {
        recorded
    }

    /// Throws the recorded failure, if there is one.
    public func throwIfFailed() throws {
        if let recorded {
            throw recorded
        }
    }
}

/// The checked integer operations, by the width of the type they are called at.
public enum Checked {
    /// `a + b` at the width of `T`, or 0 and an overflow.
    public static func add<T: FixedWidthInteger>(_ f: inout Failure, _ a: T, _ b: T) -> T {
        let (value, overflow) = a.addingReportingOverflow(b)
        if overflow {
            f.fail(.overflow)
            return 0
        }
        return value
    }

    /// `a - b` at the width of `T`, or 0 and an overflow.
    public static func sub<T: FixedWidthInteger>(_ f: inout Failure, _ a: T, _ b: T) -> T {
        let (value, overflow) = a.subtractingReportingOverflow(b)
        if overflow {
            f.fail(.overflow)
            return 0
        }
        return value
    }

    /// `a * b` at the width of `T`, or 0 and an overflow.
    public static func mul<T: FixedWidthInteger>(_ f: inout Failure, _ a: T, _ b: T) -> T {
        let (value, overflow) = a.multipliedReportingOverflow(by: b)
        if overflow {
            f.fail(.overflow)
            return 0
        }
        return value
    }

    /// `a / b` truncated toward zero, or 0 and a division by zero, or 0 and an
    /// overflow for `MIN / -1`.
    public static func div<T: FixedWidthInteger>(_ f: inout Failure, _ a: T, _ b: T) -> T {
        if b == 0 {
            f.fail(.divideByZero)
            return 0
        }
        let (value, overflow) = a.dividedReportingOverflow(by: b)
        if overflow {
            f.fail(.overflow)
            return 0
        }
        return value
    }

    /// `a % b` with the dividend's sign, or 0 and a division by zero. A remainder
    /// whose quotient overflows has no value either: `MIN % -1` fails as `MIN / -1`
    /// does, on every backend.
    public static func rem<T: FixedWidthInteger>(_ f: inout Failure, _ a: T, _ b: T) -> T {
        if b == 0 {
            f.fail(.divideByZero)
            return 0
        }
        if a.dividedReportingOverflow(by: b).overflow {
            f.fail(.overflow)
            return 0
        }
        return a.remainderReportingOverflow(dividingBy: b).partialValue
    }

    /// `-a` at the width of `T`, or 0 and an overflow.
    public static func neg<T: FixedWidthInteger>(_ f: inout Failure, _ a: T) -> T {
        let (value, overflow) = T.zero.subtractingReportingOverflow(a)
        if overflow {
            f.fail(.overflow)
            return 0
        }
        return value
    }

    /// A value stored where a narrower integer type is declared is the same value,
    /// or 0 and an overflow when the type cannot hold it: never a wrapped one.
    public static func narrow<From: BinaryInteger, To: FixedWidthInteger>(
        _ f: inout Failure,
        _ value: From,
        to type: To.Type
    ) -> To {
        guard let narrowed = To(exactly: value) else {
            f.fail(.overflow)
            return 0
        }
        return narrowed
    }

    /// `value` when it holds no more than `capacity` bytes (the bound a string
    /// variable of a sce-static machine declares, counted in UTF-8 bytes) and the
    /// empty text with a capacity failure recorded otherwise.
    ///
    /// Counted in bytes, as every backend counts them, so a machine holds the same
    /// value wherever it runs. A Swift `String.count` is grapheme clusters, which two
    /// characters of four bytes each would make two against a bound of four.
    public static func bounded(_ f: inout Failure, _ value: String, capacity: UInt32) -> String {
        if UInt64(value.utf8.count) > UInt64(capacity) {
            f.fail(.capacityExceeded)
            return ""
        }
        return value
    }

    /// `bounded` for a byte string: a copy of `value` when it holds no more than
    /// `capacity` bytes, and the empty value with a capacity failure recorded
    /// otherwise (docs/adr/0005, decision 2).
    public static func bounded(_ f: inout Failure, _ value: [UInt8], capacity: UInt32) -> [UInt8] {
        if UInt64(value.count) > UInt64(capacity) {
            f.fail(.capacityExceeded)
            return []
        }
        return value
    }

    /// `items` when it holds no more than `capacity` elements (the bound a list
    /// variable of a sce-static machine declares) and an empty list with a capacity
    /// failure recorded otherwise. A list an algorithm returns may hold more than the
    /// bound it declared on a backend whose lists grow, and a machine holds the same
    /// list wherever it runs, so the list it takes is held to its own.
    public static func within<T>(_ f: inout Failure, _ items: [T], capacity: UInt32) -> [T] {
        if UInt64(items.count) > UInt64(capacity) {
            f.fail(.capacityExceeded)
            return []
        }
        return items
    }

    /// The element of `xs` at `i`, or `fallback` and an out-of-range failure when `i`
    /// is below 0 or not below the length: never a trap.
    public static func at<T, I: BinaryInteger>(
        _ f: inout Failure,
        _ xs: [T],
        _ i: I,
        fallback: @autoclosure () -> T
    ) -> T {
        guard let index = Int(exactly: i), index >= 0, index < xs.count else {
            f.fail(.outOfRange)
            return fallback()
        }
        return xs[index]
    }

    /// `value` as an element of `list`, which gives a literal the element's type: the
    /// value an append stores is computed into a local before it is appended, and a
    /// local of a literal alone would be an `Int`.
    public static func element<T>(of list: [T], _ value: T) -> T {
        value
    }

    /// `value`, or an out-of-range failure thrown for a reading that found nothing: the
    /// shape an enum's `fromUnderlying` answers (a number no variant declares), which a
    /// call that may fail passes on through `take`.
    public static func found<T>(_ value: T?) throws -> T {
        guard let value else {
            throw AlgorithmError.outOfRange
        }
        return value
    }

    /// `take` of a call that answers an enum: the first case stands where it failed,
    /// the one value of the type that can be named without naming the type.
    public static func takeCase<T: CaseIterable>(_ f: inout Failure, _ call: () throws -> T) -> T {
        do {
            return try call()
        } catch let error as AlgorithmError {
            f.fail(error)
            guard let first = T.allCases.first else {
                preconditionFailure("Checked.takeCase: an enum with no case")
            }
            return first
        } catch {
            preconditionFailure(
                "Checked.takeCase: a may-fail reading threw an error that is not an AlgorithmError: \(error)"
            )
        }
    }

    /// A call to another may-fail algorithm: its value, or `fallback` with its
    /// failure recorded in the caller's `Failure` under the callee's own name, which
    /// the statement around it throws.
    public static func take<T>(_ f: inout Failure, fallback: @autoclosure () -> T, _ call: () throws -> T) -> T {
        do {
            return try call()
        } catch let error as AlgorithmError {
            f.fail(error)
            return fallback()
        } catch {
            preconditionFailure(
                "Checked.take: a may-fail algorithm threw an error that is not an AlgorithmError: \(error)"
            )
        }
    }
}
