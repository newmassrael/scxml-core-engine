// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The runner program of a generated suite (docs/SCE_TTCN3_LANE.md §4, §10.5): it runs every test case
// a world selects against its recorded input trace and writes each one's canonical test log.
//
// The unit of a suite generates the program's entry, `<Suite>.main()`, which hands `runnerMain`
// (`RunnerMain.swift`) the suite's tables and the reader of its module parameters
// (`<Suite>.moduleParameter(id:literal:)`); a package makes it a program with an executable target whose
// `main.swift` calls it. The command line is the same on every backend:
//
//     <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
//
// - `--trace-dir`: the directory of the traces, `<test case>.json` each
//   (`schemas/sce-test-trace.v1.schema.json`);
// - `--log-dir`: the directory the logs are written to, `<test case>.log.json` each, the canonical test
//   log's text with nothing after it (`schemas/sce-test-log.v1.schema.json`), made where it is missing;
// - `--world`: the features the world enables, by name, separated by commas; none where it is not given
//   (ADR 0057 §2);
// - `--module-parameter`: a module parameter's value in place of its default, written as
//   `<sce:module-parameter value>` writes a literal of its type (docs/SCE_TEST_PROFILE.md §11.4, §11.8);
//   repeated, one per parameter.
//
// Each test case run prints `<test case> <verdict>` on its own line, and nothing else is printed on the
// standard output. A test case whose trace is missing or refused ends with `error` and its log is
// written, the reason on the standard error (P13). The exit status is 0 when every selected test case
// ran and its log was written, whatever its verdict: the verdict is the log's. It is 2 when the command
// line is not one of the above, a module parameter's value is refused, or a log cannot be written.
//
// The program's logic is here, over the files and streams of `RunnerFiles`, so a test runs it in place
// as the program runs it.
//
// Cross-language siblings: `sce_rust_runtime::test::runner` (Rust), `com.sce.runtime.test.runRunner`
// (Kotlin), `sce/include/test/Runner.h` (C++).

import SceRuntime

/// The reader of a suite's module parameters, generated as `<Suite>.moduleParameter(id:literal:)`: the
/// saved value of the module parameter `id` of its declared type, read from `literal`, the value the
/// command line gives it, or why it is not one (an `OverrideRefusal`, where the suite declares no such
/// parameter or the literal is no value of its type).
public typealias ParameterReader =
    (_ id: String, _ literal: String) throws(OverrideRefusal) -> ModuleParameterValue

/// The program's usage line.
public let runnerUsage =
    "usage: <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]..."

/// A command line that is not the runner's, refused, naming why.
public struct RunnerUsageRefusal: Error, Equatable, Sendable {
    /// Why.
    public let reason: String

    public init(_ reason: String) {
        self.reason = reason
    }
}

/// What the command line asks of the runner.
public struct RunnerOptions: Equatable, Sendable {
    /// The directory of the traces, `<test case>.json` each.
    public var traceDir: String
    /// The directory the logs are written to, `<test case>.log.json` each.
    public var logDir: String
    /// The features the world enables.
    public var world: Features
    /// Each `--module-parameter`, its id and the literal it gives, in the order given.
    public var moduleParameters: [ModuleParameterLiteral]

    /// One `--module-parameter`: the id of the module parameter and the literal the command line gives.
    public struct ModuleParameterLiteral: Equatable, Sendable {
        public var id: String
        public var literal: String

        public init(id: String, literal: String) {
            self.id = id
            self.literal = literal
        }
    }

    public init(traceDir: String, logDir: String, world: Features, moduleParameters: [ModuleParameterLiteral]) {
        self.traceDir = traceDir
        self.logDir = logDir
        self.world = world
        self.moduleParameters = moduleParameters
    }

    /// The options `args` give, the program's name not among them.
    ///
    /// - Throws: `RunnerUsageRefusal` naming why, where they are not the program's command line.
    public static func parse(_ args: [String]) throws(RunnerUsageRefusal) -> RunnerOptions {
        var traceDir: String?
        var logDir: String?
        var world: String?
        var moduleParameters: [ModuleParameterLiteral] = []
        // Every option takes one value: the line is read a pair at a time.
        var index = 0
        while index < args.count {
            let option = args[index]
            guard index + 1 < args.count else {
                throw RunnerUsageRefusal("\(option) is given no value")
            }
            let value = args[index + 1]
            index += 2
            switch option {
            case "--trace-dir" where traceDir == nil:
                traceDir = value
            case "--log-dir" where logDir == nil:
                logDir = value
            case "--world" where world == nil:
                world = value
            case "--module-parameter":
                // Split at the first `=` byte, which is a character of its own in UTF-8, so a literal
                // that starts with a combining mark is split where the other runners split it.
                let bytes = Array(value.utf8)
                guard let split = bytes.firstIndex(of: UInt8(ascii: "=")) else {
                    throw RunnerUsageRefusal("--module-parameter \(value) is not <id>=<value>")
                }
                let id = String(decoding: bytes[..<split], as: UTF8.self)
                if moduleParameters.contains(where: { FromCanonical.same($0.id, id) }) {
                    throw RunnerUsageRefusal("the module parameter '\(id)' is given twice")
                }
                let literal = String(decoding: bytes[(split + 1)...], as: UTF8.self)
                moduleParameters.append(ModuleParameterLiteral(id: id, literal: literal))
            case "--trace-dir", "--log-dir", "--world":
                throw RunnerUsageRefusal("\(option) is given twice")
            default:
                throw RunnerUsageRefusal("\(option) is no option of the runner")
            }
        }
        guard let traceDir else {
            throw RunnerUsageRefusal("--trace-dir is not given")
        }
        guard let logDir else {
            throw RunnerUsageRefusal("--log-dir is not given")
        }
        // Split at each `,` byte, as the `=` of a module parameter is, an empty name left out.
        let features = Array((world ?? "").utf8).split(separator: UInt8(ascii: ","))
            .map { String(decoding: $0, as: UTF8.self) }
        return RunnerOptions(
            traceDir: traceDir, logDir: logDir, world: Set(features), moduleParameters: moduleParameters)
    }
}

/// Why a file of the runner is not read or written, or a directory not made, in the platform's words.
public struct RunnerFileFailure: Error, Equatable, Sendable {
    /// Why.
    public let reason: String

    public init(_ reason: String) {
        self.reason = reason
    }
}

/// The files the runner program reads and writes, on its platform (`RunnerMain.swift` holds
/// Foundation's).
public protocol RunnerFiles {
    /// The path of the file `name` in the directory `dir`, as a message names it.
    func path(_ dir: String, _ name: String) -> String

    /// The text of the file at `path`, in UTF-8.
    ///
    /// - Throws: `RunnerFileFailure` where it is not read.
    func readText(_ path: String) throws(RunnerFileFailure) -> String

    /// Makes the directory `dir`, and those above it, where they are missing.
    ///
    /// - Throws: `RunnerFileFailure` where it is not made.
    func makeDirectories(_ dir: String) throws(RunnerFileFailure)

    /// Writes `text` in UTF-8 as the whole of the file at `path`, with nothing after it.
    ///
    /// - Throws: `RunnerFileFailure` where it is not written.
    func writeText(_ path: String, _ text: String) throws(RunnerFileFailure)
}

/// The trace of the test case `testCase`, `<dir>/<test case>.json`, read by `files`.
///
/// - Throws: `TraceRefusal` where it is not read, naming its path, or is refused.
public func readTrace(_ files: any RunnerFiles, _ dir: String, _ testCase: String) throws(TraceRefusal) -> Trace {
    let path = files.path(dir, "\(testCase).json")
    let text: String
    do throws(RunnerFileFailure) {
        text = try files.readText(path)
    } catch {
        throw TraceRefusal("\(path) is not read: \(error.reason)")
    }
    return try Trace.parse(text)
}

/// The runner program of `suite`, its module parameters read by `parameter`, on the command line
/// `args`, the program's name not among them (docs/SCE_TTCN3_LANE.md §4), over `files`, each line of the
/// standard output handed to `out` and of the standard error to `err`: see the file's header. Answers
/// the program's exit status.
public func runRunner(
    suite: Suite, parameter: ParameterReader, args: [String], files: any RunnerFiles,
    out: (String) -> Void, err: (String) -> Void
) -> Int32 {
    // The refusal of the command line: why, then the usage line, and the status 2.
    func usage(_ why: String) -> Int32 {
        err(why)
        err(runnerUsage)
        return 2
    }
    let options: RunnerOptions
    do throws(RunnerUsageRefusal) {
        options = try RunnerOptions.parse(args)
    } catch {
        return usage(error.reason)
    }
    var overrides = ModuleParameters()
    for given in options.moduleParameters {
        do throws(OverrideRefusal) {
            let value = try parameter(given.id, given.literal)
            overrides.set(given.id, value)
        } catch {
            return usage("--module-parameter \(given.id)=\(given.literal): \(error.reason)")
        }
    }
    let results: [CaseResult]
    do throws(OverrideRefusal) {
        results = try runSuiteWithTraces(
            suite, world: options.world,
            traces: { (testCase: String) throws(TraceRefusal) -> Trace in
                try readTrace(files, options.traceDir, testCase)
            },
            overrides: overrides)
    } catch {
        return usage(error.reason)
    }
    do throws(RunnerFileFailure) {
        try files.makeDirectories(options.logDir)
    } catch {
        err("\(options.logDir) is not made: \(error.reason)")
        return 2
    }
    for result in results {
        for line in result.result.log where line.kind == .error {
            err("\(result.testCase): \(line.component): \(line.detail)")
        }
        let path = files.path(options.logDir, "\(result.testCase).log.json")
        do throws(RunnerFileFailure) {
            try files.writeText(path, result.canonicalLog(suite: suite.name))
        } catch {
            err("\(path) is not written: \(error.reason)")
            return 2
        }
        out("\(result.testCase) \(result.result.verdict.spelling)")
    }
    return 0
}

/// The refusal of a value for the module parameter `id`, which the suite does not declare.
public func undeclaredParameter(_ id: String) -> OverrideRefusal {
    OverrideRefusal("the suite declares no module parameter '\(id)'")
}

/// A value of the integer type `type` of the module parameter `id`: the text of its decimal digits, as
/// the canonical form writes one. The suite's unit saves it with `SavedValues.of` of its type.
///
/// - Throws: `OverrideRefusal` naming the parameter, where it is no such text or past the range of
///   `type`.
public func integerParameter<T: FixedWidthInteger>(_ id: String, _ literal: String, _ type: T.Type)
    throws(OverrideRefusal) -> T
{
    do throws(TraceRefusal) {
        return try FromCanonical.integer(.text(literal), type, id)
    } catch {
        throw OverrideRefusal(error.reason)
    }
}

/// A value of the `bool` module parameter `id`: `true` or `false`.
///
/// - Throws: `OverrideRefusal` otherwise.
public func boolParameter(_ id: String, _ literal: String) throws(OverrideRefusal) -> ModuleParameterValue {
    switch literal {
    case "true": return SavedValues.of(true)
    case "false": return SavedValues.of(false)
    default: throw OverrideRefusal("\(id): '\(literal)' is neither true nor false")
    }
}

/// A value of a text module parameter: the text itself. Its bound, like every value's type, is held
/// where the component reads it, as for a default.
public func textParameter(_ literal: String) -> ModuleParameterValue {
    SavedValues.of(literal)
}

/// A value of an enum module parameter: the name its document declares for the variant, which is how a
/// value of an enum is saved. A name its enum does not declare is refused where the component reads it,
/// as for a default.
public func enumParameter(_ literal: String) -> ModuleParameterValue {
    .text(literal)
}

/// A value of the byte string module parameter `id`: its octets in hex of either case, two digits each,
/// as a literal of docs/SCE_TEST_PROFILE.md §11.4 writes them.
///
/// - Throws: `OverrideRefusal` where it is not.
public func bytesParameter(_ id: String, _ literal: String) throws(OverrideRefusal) -> ModuleParameterValue {
    let refusal = OverrideRefusal("\(id): '\(literal)' is not octets in hex")
    let digits = Array(literal.utf8)
    if digits.count % 2 != 0 {
        throw refusal
    }
    // The value of one ASCII hex digit of either case, or nil: digits alone, where a reader of a number
    // would also read a sign.
    func hexDigit(_ digit: UInt8) -> UInt8? {
        switch digit {
        case UInt8(ascii: "0")...UInt8(ascii: "9"): return digit - UInt8(ascii: "0")
        case UInt8(ascii: "a")...UInt8(ascii: "f"): return digit - UInt8(ascii: "a") + 10
        case UInt8(ascii: "A")...UInt8(ascii: "F"): return digit - UInt8(ascii: "A") + 10
        default: return nil
        }
    }
    var octets: [UInt8] = []
    var index = 0
    while index < digits.count {
        guard let high = hexDigit(digits[index]), let low = hexDigit(digits[index + 1]) else {
            throw refusal
        }
        octets.append(high << 4 | low)
        index += 2
    }
    return SavedValues.of(octets)
}
