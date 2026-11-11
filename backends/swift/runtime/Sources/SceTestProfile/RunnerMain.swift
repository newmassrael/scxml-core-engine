// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The runner program of a generated suite on its platform (docs/SCE_TTCN3_LANE.md §4): `runRunner` over
// Foundation's files, the standard output and the standard error, ending the process with the exit
// status it answers. The entry of a generated suite's unit, `<Suite>.main()`, calls `runnerMain`, and an
// executable target whose `main.swift` calls that entry is the program
// (`backends/swift/tests/Sources/RuntimeSuiteRunner/main.swift`).
//
// Cross-language siblings: `sce_rust_runtime::test::runner::main` (Rust), `com.sce.runtime.test.runnerMain`
// (Kotlin), `SCE::Test::main` of `sce/include/test/Runner.h` (C++).

import Foundation

#if canImport(Glibc)
    import Glibc
#elseif canImport(Musl)
    import Musl
#elseif canImport(Darwin)
    import Darwin
#endif

/// The platform's files, as the runner reads and writes them (`RunnerFiles`), through Foundation.
public struct FoundationRunnerFiles: RunnerFiles {
    public init() {}

    /// `<dir>/<name>`, the directory as it was given, so a message names the path the command line
    /// gave.
    public func path(_ dir: String, _ name: String) -> String {
        dir.hasSuffix("/") ? dir + name : "\(dir)/\(name)"
    }

    public func readText(_ path: String) throws(RunnerFileFailure) -> String {
        let data: Data
        do {
            data = try Data(contentsOf: URL(fileURLWithPath: path))
        } catch {
            throw RunnerFileFailure(String(describing: error))
        }
        // A text that is not UTF-8 is not read, rather than read with its bytes replaced.
        guard let text = String(data: data, encoding: .utf8) else {
            throw RunnerFileFailure("the file is not UTF-8")
        }
        return text
    }

    public func makeDirectories(_ dir: String) throws(RunnerFileFailure) {
        do {
            try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        } catch {
            throw RunnerFileFailure(String(describing: error))
        }
    }

    public func writeText(_ path: String, _ text: String) throws(RunnerFileFailure) {
        do {
            try Data(text.utf8).write(to: URL(fileURLWithPath: path))
        } catch {
            throw RunnerFileFailure(String(describing: error))
        }
    }
}

/// One line, `line` and a line feed, written whole to the file descriptor `handle` writes.
private func writeLine(_ handle: FileHandle, _ line: String) {
    handle.write(Data((line + "\n").utf8))
}

/// The runner program of `suite`, its module parameters read by `parameter`, on the command line the
/// process was started with (docs/SCE_TTCN3_LANE.md §4): `runRunner` over `FoundationRunnerFiles`, the
/// standard output and the standard error, after which the process exits with the status it answers.
///
/// The standard output is the contract's, the `<test case> <verdict>` lines and nothing else, which a
/// lane parses. So the program keeps a descriptor of it for those lines and points the standard output
/// at the standard error while the cases run: a machine's `<log>`, which a generated Swift statechart
/// writes with `print`, and any other diagnostic of the run goes there. What `print` buffered before is
/// flushed to the standard output first, and what it buffers while the cases run to the standard error
/// before the process ends.
public func runnerMain(suite: Suite, parameter: ParameterReader) -> Never {
    let output = FileHandle.standardOutput.fileDescriptor
    let errors = FileHandle.standardError.fileDescriptor
    fflush(nil)
    // A descriptor of the standard output kept for the contract's lines; where none can be made, the
    // standard output is left where it is and the lines are written to it.
    let kept = dup(output)
    let contract: FileHandle
    if kept >= 0 {
        contract = FileHandle(fileDescriptor: kept, closeOnDealloc: true)
        if dup2(errors, output) < 0 {
            writeLine(FileHandle.standardError, "the standard output is not pointed at the standard error")
        }
    } else {
        contract = FileHandle.standardOutput
        writeLine(FileHandle.standardError, "the standard output is not kept apart from the run's diagnostics")
    }
    let status = runRunner(
        suite: suite, parameter: parameter, args: Array(ProcessInfo.processInfo.arguments.dropFirst()),
        files: FoundationRunnerFiles(),
        out: { line in writeLine(contract, line) },
        err: { line in writeLine(FileHandle.standardError, line) })
    fflush(nil)
    exit(status)
}
