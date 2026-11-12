// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import SceRuntime
import SceTestProfile
import Testing

// The recorded input trace, the readers of the canonical form and the runner's command line
// (docs/SCE_TTCN3_LANE.md §4, §10). Each case is the twin of the Rust case of the same name in
// backends/rust/runtime/src/test/trace.rs, from_canonical.rs and runner.rs
// (`a_trace_is_read_from_its_canonical_text_and_written_back` is
// `aTraceIsReadFromItsCanonicalTextAndWrittenBack`) and of the Kotlin one in TraceTest.kt, with the same
// texts and the same refusals. Every text is compared by its UTF-8 bytes, which is what two traces agree
// by.

/// The bytes of `text`.
private func bytes(_ text: String) -> [UInt8] {
    Array(text.utf8)
}

/// The reason `read` is refused for, or nil where it reads.
private func refusal<T>(_ read: () throws(TraceRefusal) -> T) -> String? {
    do throws(TraceRefusal) {
        _ = try read()
        return nil
    } catch {
        return error.reason
    }
}

/// The reason `read` is refused for as an override, or nil where it reads.
private func overrideRefusal<T>(_ read: () throws(OverrideRefusal) -> T) -> String? {
    do throws(OverrideRefusal) {
        _ = try read()
        return nil
    } catch {
        return error.reason
    }
}

/// Whether `reason` holds `expected`.
private func holds(_ reason: String?, _ expected: String) -> Bool {
    reason.map { $0.contains(expected) } ?? false
}

/// The repository, above this file: the file, then SceTestProfileTests, Tests, runtime, swift and
/// backends.
private func repoRoot() -> URL {
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url
}

private let oneInput =
    "{\"inputs\":[{\"at\":\"500\",\"port\":\"sut\",\"type\":\"job_reply\","
    + "\"value\":{\"code\":\"0\",\"job\":\"7\"}}],"
    + "\"schema\":\"sce-test-trace.v1\",\"suite\":\"runtime_suite\","
    + "\"test_case\":\"answers_the_job\"}"

/// `oneInput` with its first `from` replaced by `to`.
private func changed(_ from: String, _ to: String) -> String {
    guard let range = oneInput.range(of: from) else {
        Issue.record("`\(from)` is not in the trace")
        return oneInput
    }
    return oneInput.replacingCharacters(in: range, with: to)
}

/// The reason `Trace.parse` refuses `text` for, or nil where it reads it.
private func refused(_ text: String) -> String? {
    refusal { () throws(TraceRefusal) -> Trace in try Trace.parse(text) }
}

struct TraceTests {
    /// The status the runtime declares is the one the trace's schema file declares, and the schema pins
    /// the name of the shape the runtime reads.
    @Test func theSchemaFileDeclaresTheStatusTheRuntimeDoes() throws {
        let path = repoRoot().appendingPathComponent("schemas/sce-test-trace.v1.schema.json")
        let schema = try parseJSON(try String(contentsOf: path, encoding: .utf8))
        #expect(
            schema.member("x-sce-schema-status") == JSONValue.text(testTraceSchemaStatus),
            Comment(
                rawValue: "schemas/sce-test-trace.v1.schema.json x-sce-schema-status disagrees with "
                    + "testTraceSchemaStatus; SCE_WIRE_CONTRACTS.md requires one commit to move both"))
        let shape = schema.member("properties")?.member("schema")?.member("const")
        #expect(shape == JSONValue.text(testTraceSchema))
    }

    /// A trace is read from its canonical text, with or without the line feed a file ends in, and
    /// written back as the same bytes.
    @Test func aTraceIsReadFromItsCanonicalTextAndWrittenBack() throws {
        let trace = try Trace.parse(oneInput)
        #expect(trace.suite == "runtime_suite")
        #expect(trace.testCase == "answers_the_job")
        let value = Canonical.record(
            ("code", Canonical.integer(UInt32(0))), ("job", Canonical.integer(UInt32(7))))
        #expect(trace.inputs == [TraceInput(at: 500, port: "sut", messageType: "job_reply", value: value)])
        #expect(bytes(trace.canonicalText()) == bytes(oneInput))
        #expect(try Trace.parse(oneInput + "\n") == trace)
    }

    /// Each change of one member of a trace it first reads is refused, by name.
    @Test func aTraceThatIsNotOfItsShapeIsRefusedByName() {
        let cases: [(String, String)] = [
            (changed("sce-test-trace.v1", "sce-test-trace.v2"), "schema is not sce-test-trace.v1"),
            (changed("\"500\"", "500"), "the instant of input 0"),
            (changed("\"500\"", "\"0500\""), "the instant of input 0"),
            (changed("\"500\"", "\"-5\""), "the instant of input 0"),
            (changed("\"500\"", "\"9223372036854775808\""), "past the microseconds"),
            (changed("\"port\":\"sut\"", "\"port\":\"\""), "the port of input 0"),
            (changed("\"type\"", "\"kind\""), "input 0 has no member 'kind'"),
            (changed(",\"suite\":\"runtime_suite\"", ""), "the trace has no 'suite'"),
            (changed("{\"inputs\"", "{ \"inputs\""), "one canonical text"),
            (changed("[{", "{"), "not JSON"),
            // One line feed ends a text file; a second is a text after the trace.
            (oneInput + "\n\n", "one canonical text"),
        ]
        for (text, expected) in cases {
            let reason = refused(text)
            #expect(holds(reason, expected), "\(expected): \(reason ?? "read")")
        }
        let outOfOrder =
            "{\"inputs\":[{\"at\":\"5\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}},"
            + "{\"at\":\"4\",\"port\":\"sut\",\"type\":\"t\",\"value\":{}}],"
            + "\"schema\":\"sce-test-trace.v1\",\"suite\":\"s\",\"test_case\":\"c\"}"
        #expect(holds(refused(outOfOrder), "input 1, at 4, comes after an input at 5"))
    }

    /// A name is read and written back by its bytes: a text canonically equivalent to another, and not
    /// the same bytes, is a name of its own, which Swift's `==` on a text would hold equal to the other.
    @Test func aNameIsReadAndWrittenBackByItsBytes() throws {
        let decomposed = changed("\"type\":\"job_reply\"", "\"type\":\"re\u{301}ply\"")
        let read = try Trace.parse(decomposed)
        #expect(bytes(read.inputs[0].messageType) == bytes("re\u{301}ply"))
        #expect(bytes(read.canonicalText()) == bytes(decomposed))
    }

    /// A number past the range of a double has no canonical text, so a trace whose value holds one is
    /// refused as not written in it, not thrown out of the reader.
    @Test func aTraceWhoseValueHoldsANumberPastADoubleIsRefused() {
        let text = changed("{\"code\":\"0\",\"job\":\"7\"}", "1e400")
        // The Rust reader refuses the same text in the same words
        // (`a_trace_whose_value_holds_a_number_past_a_double_is_refused`).
        #expect(
            refused(text)
                == "the trace is not written in its one canonical text, the JSON Canonicalization Scheme")
    }

    /// An input on a port the configuration does not have, or of a type its port does not take, is
    /// refused, naming which.
    @Test func anInputTheConfigurationDoesNotTakeIsRefusedByName() throws {
        #expect(
            TraceRefusal.notAnInput("sut", "job_request", ["sut"]).reason
                == "'job_request' is not on the `in` list of the type of the system port 'sut'")
        #expect(
            TraceRefusal.notAnInput("oracle", "job_reply", ["sut"]).reason
                == "'oracle' is not a system port of the test case's configuration")
        let trace = try Trace.parse(oneInput)
        let refused = refusal { () throws(TraceRefusal) -> TraceAdapter in
            try TraceAdapter.replaying(trace, noInputs)
        }
        #expect(refused == "input 0, at 500: 'sut' is not a system port of the test case's configuration")
    }

    /// The player hands the run each input at its instant, those of one instant in the trace's order,
    /// announces the instant of the next, and records what the components send.
    @Test func thePlayerDeliversEachInputAtItsInstant() throws {
        let read: InputReader = { (systemPort, typeName, value) throws(TraceRefusal) -> Message in
            Message(event: typeName, value: messageValue(value, typeName, { $0 }))
        }
        let trace = Trace(
            suite: "s", testCase: "c",
            inputs: [
                TraceInput(at: 0, port: "sut", messageType: "first", value: .text("a")),
                TraceInput(at: 0, port: "sut", messageType: "second", value: .text("b")),
                TraceInput(at: 250, port: "sut", messageType: "third", value: .text("c")),
            ])
        let player = try TraceAdapter.replaying(trace, read)
        #expect(player.nextInput() == 0)
        #expect(player.takeInputsDue(now: 0).map(\.message.event) == ["first", "second"])
        #expect(player.nextInput() == 250)
        #expect(player.takeInputsDue(now: 249).isEmpty)
        let last = player.takeInputsDue(now: 250).map { $0.message.canonicalValue() }
        #expect(last == [JSONValue.text("c")])
        #expect(player.nextInput() == nil)
        let out = Message(event: "out", value: messageValue(JSONValue.null, "o", { $0 }))
        player.send(systemPort: "sut", message: out, now: 3)
        #expect(player.sent.map(\.at) == [3])
        #expect(player.sent.map(\.systemPort) == ["sut"])
    }
}

struct FromCanonicalTests {
    /// An integer is read from the one spelling the writer gives it, at every width, and any other
    /// spelling, or a value past its type, is refused, naming where it stands.
    @Test func anIntegerIsReadFromTheTextOfItsDigitsAlone() throws {
        #expect(try FromCanonical.integer(Canonical.integer(UInt8(0)), UInt8.self, "n") == 0)
        #expect(try FromCanonical.integer(Canonical.integer(UInt64.max), UInt64.self, "n") == UInt64.max)
        #expect(try FromCanonical.integer(Canonical.integer(Int64.min), Int64.self, "n") == Int64.min)
        for spelling in ["007", "+7", "-0", "7.0", "", " 7"] {
            let reason = refusal { () throws(TraceRefusal) -> UInt32 in
                try FromCanonical.integer(.text(spelling), UInt32.self, "job_reply.code")
            }
            #expect(reason?.hasPrefix("job_reply.code: ") ?? false, "\(spelling): \(reason ?? "read")")
            #expect(holds(reason, "decimal digits"), "\(spelling): \(reason ?? "read")")
        }
        let wide = refusal { () throws(TraceRefusal) -> UInt8 in
            try FromCanonical.integer(.text("256"), UInt8.self, "n")
        }
        #expect(holds(wide, "past the range"))
        let negative = refusal { () throws(TraceRefusal) -> UInt32 in
            try FromCanonical.integer(.text("-1"), UInt32.self, "n")
        }
        #expect(holds(negative, "past the range"))
        let number = refusal { () throws(TraceRefusal) -> UInt32 in
            try FromCanonical.integer(.number("7"), UInt32.self, "n")
        }
        #expect(holds(number, "a number is not an integer"))
        // Past the range of a 128-bit integer, as the Rust reader of the same text, the digits are no
        // integer's at all.
        let past = "1" + String(repeating: "0", count: 39)
        let unreadable = refusal { () throws(TraceRefusal) -> UInt64 in
            try FromCanonical.integer(.text(past), UInt64.self, "n")
        }
        #expect(holds(unreadable, "is not the text of an integer's decimal digits"))
    }

    @Test func aBoolIsATruthValue() throws {
        #expect(try FromCanonical.boolean(Canonical.boolean(true), "b"))
        let text = refusal { () throws(TraceRefusal) -> Bool in try FromCanonical.boolean(.text("true"), "b") }
        #expect(holds(text, "a text is not a truth value"))
    }

    /// A text is read within its bound in UTF-8 bytes, not in characters.
    @Test func aTextIsReadWithinItsBound() throws {
        #expect(try FromCanonical.text(Canonical.text("abc"), 3, "t") == "abc")
        let long = refusal { () throws(TraceRefusal) -> String in
            try FromCanonical.text(Canonical.text("abcd"), 3, "t")
        }
        #expect(holds(long, "past the 3"))
        let wide = refusal { () throws(TraceRefusal) -> String in
            try FromCanonical.text(Canonical.text("\u{E9}"), 1, "t")
        }
        #expect(wide == "t: a text of 2 bytes is past the 1 its type holds")
    }

    /// A byte string is read from upper-case hex alone, the writer's spelling, and back to the same text.
    @Test func aByteStringIsReadFromUpperCaseHex() throws {
        let octets: [UInt8] = [0x00, 0xAB, 0x0F, 0xFF]
        let written = Canonical.bytes(octets)
        let read = try FromCanonical.bytes(written, 4, "b")
        #expect(read == octets)
        #expect(Canonical.bytes(read) == written)
        for spelling in ["00ab", "0", "0G"] {
            let reason = refusal { () throws(TraceRefusal) -> [UInt8] in
                try FromCanonical.bytes(.text(spelling), 4, "b")
            }
            #expect(holds(reason, "upper-case hex"), "\(spelling)")
        }
        let long = refusal { () throws(TraceRefusal) -> [UInt8] in try FromCanonical.bytes(written, 3, "b") }
        #expect(holds(long, "past the 3"))
    }

    /// A closed enum is read by its declared names; an open one by its names and by the numbers no
    /// variant declares, a declared number being the second spelling of its variant.
    @Test func anEnumIsReadByItsDeclaredName() throws {
        let declared: [(String, UInt8)] = [("red", 0), ("green", 1)]
        #expect(try FromCanonical.enumerated(.text("green"), declared, "c") == 1)
        let blue = refusal { () throws(TraceRefusal) -> UInt8 in
            try FromCanonical.enumerated(.text("blue"), declared, "c")
        }
        #expect(holds(blue, "'blue' names no"))
        func open(_ text: String) -> String? {
            refusal { () throws(TraceRefusal) -> UInt8 in
                try FromCanonical.openEnumerated(.text(text), declared, UInt8.self, "c", { $0 })
            }
        }
        #expect(try FromCanonical.openEnumerated(.text("red"), declared, UInt8.self, "c", { $0 }) == 0)
        #expect(try FromCanonical.openEnumerated(.text("7"), declared, UInt8.self, "c", { $0 }) == 7)
        #expect(holds(open("1"), "written by its name"))
        #expect(holds(open("300"), "past the range"))
        #expect(holds(open("hexagon"), "names no variant of its enum, and is no number"))
        // A name is compared by its bytes: the decomposed spelling of a declared name, which Swift's `==`
        // on a text holds equal to it, names no variant, as it names none on every other runtime.
        let accented: [(String, UInt8)] = [("caf\u{E9}", 3)]
        #expect(try FromCanonical.enumerated(.text("caf\u{E9}"), accented, "c") == 3)
        let decomposed = refusal { () throws(TraceRefusal) -> UInt8 in
            try FromCanonical.enumerated(.text("cafe\u{301}"), accented, "c")
        }
        #expect(decomposed == "c: 'cafe\u{301}' names no variant of its enum")
    }

    /// A record holds only the fields it declares, each once; a field whose type is not optional is
    /// present, and an optional one that is present as `null` is read, and refused, as its type's reader
    /// refuses `null`.
    @Test func aRecordHoldsItsDeclaredFieldsOnce() throws {
        let value = try parseJSON("{\"code\":\"0\",\"job\":\"7\"}")
        let job = try FromCanonical.record(value, ["job", "code"], "job_reply") {
            (record: JSONValue) throws(TraceRefusal) -> UInt32 in
            try FromCanonical.integer(FromCanonical.field(record, "job", "job_reply.job"), UInt32.self, "job")
        }
        #expect(job == 7)
        let readNote = { (held: JSONValue) throws(TraceRefusal) -> String in
            try FromCanonical.text(held, 8, "job_reply.note")
        }
        #expect(try FromCanonical.optionalField(value, "note", readNote) == nil)
        let stray = try parseJSON("{\"job\":\"7\",\"jobs\":\"1\"}")
        let strayRefused = refusal { () throws(TraceRefusal) -> JSONValue in
            try FromCanonical.record(stray, ["job", "code"], "job_reply", { $0 })
        }
        #expect(holds(strayRefused, "'jobs' is no field"))
        // The runtime's JSON reader refuses a member named twice, so the value that names one twice is
        // built here, as a reader of another text could build it.
        let twice = JSONValue.object([(key: "job", value: .text("7")), (key: "job", value: .text("8"))])
        let twiceRefused = refusal { () throws(TraceRefusal) -> JSONValue in
            try FromCanonical.record(twice, ["job", "code"], "job_reply", { $0 })
        }
        #expect(twiceRefused == "job_reply: the field 'job' is given twice")
        let short = try parseJSON("{\"job\":\"7\"}")
        let absent = refusal { () throws(TraceRefusal) -> JSONValue in
            try FromCanonical.field(short, "code", "job_reply.code")
        }
        #expect(absent == "job_reply.code: the field is absent, and its type is not optional")
        let written = try parseJSON("{\"note\":null}")
        let null = refusal { () throws(TraceRefusal) -> String? in
            try FromCanonical.optionalField(written, "note", readNote)
        }
        #expect(null == "job_reply.note: null is not a text")
    }

    /// A list is read within its bounds, an absent element as nil.
    @Test func aListIsReadWithinItsBounds() throws {
        let value = try parseJSON("[\"1\",null]")
        let readHeld = { (held: JSONValue) throws(TraceRefusal) -> UInt8 in
            try FromCanonical.integer(held, UInt8.self, "l[]")
        }
        let read = try FromCanonical.elements(value, 1, 2, "l") { (item: JSONValue) throws(TraceRefusal) -> UInt8? in
            try FromCanonical.nullable(item, readHeld)
        }
        #expect(read == [1, nil])
        let outside = refusal { () throws(TraceRefusal) -> [JSONValue] in
            try FromCanonical.elements(value, 3, 4, "l", { $0 })
        }
        #expect(holds(outside, "outside its bounds, 3 to 4"))
    }

    /// A union is one member named by its case; a case that holds no record is the empty object.
    @Test func aUnionIsOneMemberNamedByItsCase() throws {
        let card = Canonical.unionCase("card", Canonical.record(("amount", Canonical.integer(UInt32(1)))))
        let cases = ["card", "cash"]
        let read = try FromCanonical.unionCase(card, cases, "p")
        #expect(read.variant == "card")
        #expect(bytes(try writeCanonicalJSON(read.value)) == bytes("{\"amount\":\"1\"}"))
        let cash = try FromCanonical.unionCase(Canonical.unionCase("cash", nil), cases, "p")
        #expect(try FromCanonical.noRecord(cash.value, "p", "cash") == "cash")
        let full = refusal { () throws(TraceRefusal) -> String in try FromCanonical.noRecord(card, "p", "cash") }
        #expect(holds(full, "is the empty object"))
        let two = try parseJSON("{\"card\":{},\"cash\":{}}")
        let both = refusal { () throws(TraceRefusal) -> FromCanonical.UnionCase in
            try FromCanonical.unionCase(two, cases, "p")
        }
        #expect(holds(both, "this one has 2"))
        let cheque = refusal { () throws(TraceRefusal) -> FromCanonical.UnionCase in
            try FromCanonical.unionCase(Canonical.unionCase("cheque", nil), cases, "p")
        }
        #expect(cheque == "p: 'cheque' names no case of its union")
    }
}

struct RunnerOptionsTests {
    /// The reason `RunnerOptions.parse` refuses `args` for, or nil where it reads them.
    private func refused(_ args: [String]) -> String? {
        do throws(RunnerUsageRefusal) {
            _ = try RunnerOptions.parse(args)
            return nil
        } catch {
            return error.reason
        }
    }

    /// The command line every backend's runner takes.
    @Test func theCommandLineIsReadAsEveryRunnerReadsIt() throws {
        let options = try RunnerOptions.parse([
            "--trace-dir", "traces",
            "--log-dir", "logs",
            "--world", "ptcs,guard",
            "--module-parameter", "job=9",
            "--module-parameter", "label=a=b",
        ])
        #expect(options.traceDir == "traces")
        #expect(options.logDir == "logs")
        #expect(options.world == ["ptcs", "guard"])
        let given = [
            RunnerOptions.ModuleParameterLiteral(id: "job", literal: "9"),
            RunnerOptions.ModuleParameterLiteral(id: "label", literal: "a=b"),
        ]
        #expect(options.moduleParameters == given)
        #expect(try RunnerOptions.parse(["--log-dir", "l", "--trace-dir", "t"]).world.isEmpty)
        #expect(try RunnerOptions.parse(["--trace-dir", "t", "--log-dir", "l", "--world", ""]).world.isEmpty)
        let twice = ["--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job=1", "--module-parameter", "job=2"]
        let lines: [([String], String)] = [
            (["--log-dir", "l"], "--trace-dir is not given"),
            (["--trace-dir", "t"], "--log-dir is not given"),
            (["--trace-dir", "t", "--log-dir", "l", "--trace-dir", "u"], "--trace-dir is given twice"),
            (["--trace-dir", "t", "--log-dir", "l", "--seed", "1"], "--seed is no option"),
            (["--trace-dir"], "--trace-dir is given no value"),
            (["--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job"], "is not <id>=<value>"),
            (twice, "'job' is given twice"),
        ]
        for (line, why) in lines {
            let reason = refused(line)
            #expect(holds(reason, why), "\(why): \(reason ?? "read")")
        }
    }

    /// A module parameter's literal is read as the value of its type, and one that is no value of it is
    /// refused, naming the parameter.
    @Test func aModuleParameterIsReadAsTheSavedValueOfItsType() throws {
        #expect(try integerParameter("job", "9", UInt32.self) == 9)
        #expect(try integerParameter("wide", "18446744073709551615", UInt64.self) == UInt64.max)
        let wide = overrideRefusal { () throws(OverrideRefusal) -> UInt8 in
            try integerParameter("job", "256", UInt8.self)
        }
        #expect(wide?.hasPrefix("job: ") ?? false, "\(wide ?? "read")")
        let leading = overrideRefusal { () throws(OverrideRefusal) -> UInt32 in
            try integerParameter("job", "09", UInt32.self)
        }
        #expect(leading != nil)
        #expect(try boolParameter("armed", "true") == JSONValue.bool(true))
        let yes = overrideRefusal { () throws(OverrideRefusal) -> ModuleParameterValue in
            try boolParameter("armed", "yes")
        }
        #expect(yes == "armed: 'yes' is neither true nor false")
        #expect(textParameter("nightly") == JSONValue.text("nightly"))
        #expect(enumParameter("green") == JSONValue.text("green"))
        #expect(try bytesParameter("key", "0aFF") == SavedValues.of([0x0A, 0xFF] as [UInt8]))
        // A fullwidth digit is a digit of another block, and no hex digit.
        for literal in ["0a0", "0g", "+a", "\u{FF10}\u{FF11}"] {
            let reason = overrideRefusal { () throws(OverrideRefusal) -> ModuleParameterValue in
                try bytesParameter("key", literal)
            }
            #expect(reason == "key: '\(literal)' is not octets in hex")
        }
        #expect(undeclaredParameter("limit").reason == "the suite declares no module parameter 'limit'")
    }
}
