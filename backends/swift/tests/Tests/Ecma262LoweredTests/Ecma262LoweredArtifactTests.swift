// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A Lua-lowered SWIFT artifact, compiled and RUN, answers ECMA-262.
//
// The twin of `backends/kotlin/lowered-ecma262/.../LoweredEcma262ArtifactTest.kt`, and it closes
// the gap that file's header names: until a generated artifact is compiled with the build-time
// lowering in it and driven to its final state, "the frontend answers the shared table" is a
// statement about `sce-build` plus somebody else's Lua. `EcmaScriptSemanticsTests` (in `SceLua`)
// hands the engine the lowered Lua of a committed table; this runs a machine.
//
// What is measured is one generated machine (`scripts/regen_swift_ecma262_lowered.sh`) that asks
// every case of the shared table as a `cond=` or an `expr=`. Swift has one route into its Lua
// session, the Lua the frontend lowered at build time, so there is no source-passing control
// artifact to compare it with, and what Kotlin reads off two artifacts is read here off one.
//
// The population is the shared table, in full. The expectations are read from it and are not in
// the machine: a fixture that carried its own answers would be compared against itself.
//
// Held in BOTH directions, as Kotlin's is: a case the artifact gets wrong without being named in
// `tests/ecmascript/swift_lowered_artifact_defects.json` is red, and so is a case named there that
// it now answers correctly, which is what lets the list stay at zero honestly.

import Ecma262LoweredMachines
import Foundation
import SceLua
import SceRuntime
import Testing

// MARK: - What is read

/// A JSON value, decoded so that a boolean and a number are two different answers: `true` and `1`
/// are exactly what a Lua-shaped translation confuses.
private enum Json: Decodable, Equatable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([Json])
    case object([String: Json])

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([Json].self) {
            self = .array(value)
        } else {
            self = .object(try container.decode([String: Json].self))
        }
    }
}

private struct TableCase: Decodable {
    let source: String
    let clause: String
    let form: String
    let expect: [String: Json]
}

private struct Table: Decodable {
    let cases: [TableCase]
}

private struct Defect: Decodable {
    let source: String
    let clause: String
}

private struct Defects: Decodable {
    let defects: [Defect]
}

// MARK: - The protocol of the fixture

/// Spelled in `tools/generate_lowered_ecma262_fixture.py` and repeated here and not imported, because
/// the two files are the two halves of the gate: a harness that read its protocol out of the
/// generator could not disagree with it.
private let condHeld = 1.0
private let condNotHeld = 2.0
private let unevaluated = "<unevaluated>"
private let controlRefused = "ctlRefused"
private let controlEvaluable = "ctlEvaluable"

/// The floor: a table that shrank to nothing would score the artifact perfectly. The number of
/// `MIN_CASES` in the fixture generator, in `ecma262_scoreboard_contract` and in the Kotlin twin.
private let minCases = 55

/// The ceiling on the defects ledger. An exclusion list with no ceiling is a way to make any lane
/// green; every entry is a defect someone is expected to remove.
private let maxDefects = 3

private let casesPath = "tests/ecmascript/ecma262_semantics.json"
private let defectsPath = "tests/ecmascript/swift_lowered_artifact_defects.json"

/// The repository root, found from this file's own path:
/// `<root>/backends/swift/tests/Tests/Ecma262LoweredTests/<this file>`.
private func repositoryRoot(file: String = #filePath) -> URL {
    var url = URL(fileURLWithPath: file)
    for _ in 0..<6 {
        url.deleteLastPathComponent()
    }
    return url
}

private func read<T: Decodable>(_ type: T.Type, _ path: String) throws -> T {
    let url = repositoryRoot().appendingPathComponent(path)
    return try JSONDecoder().decode(type, from: Data(contentsOf: url))
}

private struct Case {
    /// Index into the table's `cases`, which is the fixture's own key: state `dN` asks case N.
    let index: Int
    let source: String
    let clause: String
    let asCondition: Bool
    let expect: [String: Json]
    let codegenDefect: Bool

    var label: String { "[\(source)] (\(clause))" }
}

private func population() throws -> [Case] {
    let table = try read(Table.self, casesPath)
    let defects = Set(try read(Defects.self, defectsPath).defects.map { "\($0.source)\u{0}\($0.clause)" })
    let cases = table.cases.enumerated().map { index, row in
        Case(
            index: index,
            source: row.source,
            clause: row.clause,
            asCondition: row.form == "condition",
            expect: row.expect,
            codegenDefect: defects.contains("\(row.source)\u{0}\(row.clause)")
        )
    }
    // An entry that excuses nothing is a claim no lane can fault, and the way an exclusion list
    // rots: the "remove it when the artifact answers it" direction can never fire on a case that is
    // not in the population.
    let known = Set(cases.map { "\($0.source)\u{0}\($0.clause)" })
    let unmatched = defects.subtracting(known)
    #expect(unmatched.isEmpty, "\(unmatched.count) entry/entries in \(defectsPath) name no case in \(casesPath)")
    return cases
}

// MARK: - The run

private enum Reading: String {
    /// `answers.rN` absent: the case's state was never entered. Never a pass.
    case notReached
    /// The slot still holds the sentinel: the engine refused the expression.
    case refused
    /// Reached, and the slot is absent: the expression evaluated to nil, which the engine's JSON
    /// encoding omits. The table spells it `{"empty": true}`.
    case empty
    /// The slot holds a value.
    case value
}

private struct Answer {
    let reading: Reading
    var value: Json?
    /// Could the engine evaluate the case's expression at all? For a condition this is the probe's
    /// answer and it is load-bearing: a guard the engine refused evaluates to false (§scxml-5.9.1),
    /// so without it every case whose ECMA-262 answer is `false` would pass on an expression that
    /// could not be parsed.
    var evaluable = false

    var isValue: Bool { reading == .value }
}

private struct Recorded {
    let completed: Bool
    let answers: [String: Json]?
    /// Why the artifact's start was refused (docs/adr/0096), nil for one that started.
    var refusal: String? = nil
}

/// Drive the artifact to its final state and read the answers back. The read is one fixed
/// expression and reads the recording; every answer in it was computed before this call is made.
/// A start refused for its script session ran nothing, so there is nothing to read.
private func drive() -> Recorded {
    let policy = Ecma262LoweredPolicy()
    let engine = Engine(policy: policy)
    do {
        try engine.initialize()
    } catch {
        return Recorded(completed: false, answers: nil, refusal: String(describing: error))
    }
    let completed = engine.runUntilCompletion(timeout: .seconds(3), pollInterval: .milliseconds(10))
    guard case .string(let text)? = policy.readScriptValue("JSON.stringify(answers)"),
        let answers = try? JSONDecoder().decode([String: Json].self, from: Data(text.utf8))
    else {
        return Recorded(completed: completed, answers: nil)
    }
    return Recorded(completed: completed, answers: answers)
}

private let recorded = drive()

private func probeSaysEvaluable(_ answers: [String: Json], _ c: Case) -> Bool {
    // The probe assignment yielded nil, which the engine's JSON encoding omits: it evaluated, and
    // produced nothing to hold.
    guard let probe = answers["v\(c.index)"] else {
        return answers["r\(c.index)"] != nil
    }
    if case .string(let text) = probe {
        return text != unevaluated
    }
    return true
}

private func readAnswer(_ answers: [String: Json], _ c: Case) -> Answer {
    guard answers["r\(c.index)"] != nil else {
        return Answer(reading: .notReached)
    }
    guard let slot = answers["d\(c.index)"] else {
        return Answer(reading: .empty, value: nil, evaluable: !c.asCondition)
    }
    if case .string(let text) = slot, text == unevaluated {
        return Answer(reading: .refused)
    }
    return Answer(
        reading: .value, value: slot,
        evaluable: c.asCondition ? probeSaysEvaluable(answers, c) : true)
}

/// Does one recorded answer agree with ECMA-262? Everything is compared by type as well as by
/// value: `0`, `false` and `""` are three different answers, and truthiness is exactly where a
/// Lua-shaped translation confuses them.
private func agrees(_ answer: Answer, _ c: Case) -> Bool {
    if c.asCondition {
        guard answer.isValue, case .number(let held)? = answer.value else { return false }
        // A refusal is never an answer about the language, however well it matches the expectation.
        guard answer.evaluable else { return false }
        guard case .bool(let wanted)? = c.expect["bool"] else { return false }
        return held == (wanted ? condHeld : condNotHeld)
    }
    if c.expect["empty"] != nil {
        // The one shape that is an ABSENCE: a pass only when the case was reached and the slot went
        // from the sentinel to nothing.
        return answer.reading == .empty
    }
    guard answer.isValue, let value = answer.value else { return false }
    if case .bool(let wanted)? = c.expect["bool"] {
        return value == .bool(wanted)
    }
    if case .number(let wanted)? = c.expect["number"] {
        if case .number(let got) = value { return got == wanted }
        return false
    }
    if case .string(let wanted)? = c.expect["string"] {
        return value == .string(wanted)
    }
    return false
}

private func controlReading(_ answers: [String: Json], _ slot: String) -> String {
    switch answers[slot] {
    case nil: return "<absent>"
    case .string(let text)?: return text
    case .number(let number)?: return String(number)
    case .bool(let flag)?: return String(flag)
    case let other?: return String(describing: other)
    }
}

// MARK: - The tests

@Test func theArtifactReachedItsFinalStateAndRecordedItsAnswers() throws {
    #expect(recorded.refusal == nil, "the artifact's start was refused: \(recorded.refusal ?? "")")
    #expect(
        recorded.completed,
        "the artifact did not reach its final state, so the answers would be a partial run reported as a full one")
    #expect(
        recorded.answers != nil,
        "the artifact's `answers` object could not be read back: no session, or the engine refused JSON.stringify(answers)")
}

/// The population is the shared table in full, and large enough to mean something. A floor and not
/// an exact count: cases are added to the table and the lane must not have to be edited for that.
@Test func thePopulationIsTheSharedTableInFull() throws {
    let cases = try population()
    #expect(cases.count >= minCases, "the shared table joined \(cases.count) case(s), under the floor of \(minCases)")
    let defects = try read(Defects.self, defectsPath).defects
    #expect(defects.count <= maxDefects, "\(defectsPath) names \(defects.count) defect(s), over the ceiling of \(maxDefects)")
}

/// The refusal probe reported BOTH outcomes on this run. `agrees` refuses a condition verdict whose
/// probe says the engine could not evaluate the expression, so a probe stuck on one answer decides
/// the whole measurement: stuck on "refused" makes every condition a divergence, stuck on
/// "evaluated" makes a genuine refusal read as the answer `false`.
@Test func theRefusalProbeDistinguishedBothOutcomes() throws {
    let answers = try #require(recorded.answers)
    let refused = controlReading(answers, controlRefused)
    let evaluable = controlReading(answers, controlEvaluable)
    #expect(
        refused == unevaluated,
        "the refusal control read '\(refused)', not the unevaluated sentinel: the probe is not reporting refusals")
    #expect(
        evaluable != unevaluated,
        "the evaluable control read the unevaluated sentinel over a literal: the probe is stuck on refusal")
}

/// BUILD-TIME lowering answers the language, held in both directions.
@Test func buildTimeLoweringAnswersEcma262ExceptWhereADefectIsNamed() throws {
    let answers = try #require(recorded.answers)
    var unexpected: [String] = []
    var fixed: [String] = []
    for c in try population() {
        let answer = readAnswer(answers, c)
        let ok = agrees(answer, c)
        if c.codegenDefect {
            // The day the generator stops mishandling the case it starts passing, and the entry has
            // to go; without this direction the list could only grow.
            if ok {
                fixed.append("\(c.label): named in \(defectsPath) as a code-generation defect, and answered correctly")
            }
            continue
        }
        if !ok {
            let probe = c.asCondition && !answer.evaluable ? " (probe: the engine could not evaluate it)" : ""
            unexpected.append("\(c.label): expected \(c.expect), read \(answer.reading.rawValue) \(String(describing: answer.value))\(probe)")
        }
    }
    #expect(
        unexpected.isEmpty,
        "\(unexpected.count) case(s) the artifact answers differently from ECMA-262 without \(defectsPath) naming them:\n  \(unexpected.joined(separator: "\n  "))")
    #expect(
        fixed.isEmpty,
        "\(fixed.count) case(s) are named as defects and answered correctly. Remove the entry:\n  \(fixed.joined(separator: "\n  "))")
}

/// The census, printed so a GREEN run states what it measured: a number that only exists on a red
/// run is a number nobody can cite from a green one. The gate lifts this line out of the log.
@Test func theRunReportsWhatItMeasured() throws {
    let answers = try #require(recorded.answers)
    let cases = try population()
    print(
        "LoweredEcma262Swift census: cases=\(cases.count) "
            + "codegen-defects=\(cases.filter { $0.codegenDefect }.count) "
            + "lowered-control-refused=\(controlReading(answers, controlRefused)) "
            + "lowered-control-evaluable=\(controlReading(answers, controlEvaluable))")
}
