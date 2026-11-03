// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceForgeRuntime
import XCTest

@testable import StaticDatamodelMachines

// The procedures `sce-codegen generate -l swift` writes (`scripts/regen_static_datamodel_swift.sh`):
// a class that adopts the runtime's `ProcedureStateMachine`, whose run loop takes it from the
// initial state through its service sends to a final state. The units are the documents
// `tests/forge/resources/procedure_startup_check.scxml` (a decision tree over two inputs) and
// `procedure_security_access.scxml` (service sends, a helper, an assign, a bytes cap, donedata).
final class ProcedureUnitTests: XCTestCase {
    // A pure decision tree names no event and sends nothing: it runs by its guards.
    func testADecisionTreeEndsInTheFinalStateItsGuardsPick() {
        func run(voltage: Float, temperature: Float) -> ProcedureRunResult {
            let procedure = ProcedureStartupCheck()
            procedure.setVoltage(voltage)
            procedure.setTemperature(temperature)
            return procedure.runToCompletion()
        }
        XCTAssertEqual(run(voltage: 12.0, temperature: 50.0).finalState, "success")
        XCTAssertEqual(run(voltage: 10.0, temperature: 50.0).finalState, "fail_voltage")
        XCTAssertEqual(run(voltage: 12.0, temperature: 90.0).finalState, "fail_overtemp")
        XCTAssertTrue(run(voltage: 12.0, temperature: 50.0).completed)
    }

    func testTheBoundsOfAGuardAreInclusiveWhereTheDocumentSaysSo() {
        let procedure = ProcedureStartupCheck()
        procedure.setVoltage(11.5)
        procedure.setTemperature(79.9)
        XCTAssertEqual(procedure.runToCompletion().finalState, "success")
    }

    private struct Session {
        var requests: [ProcedureServiceRequest] = []
        var seedAnswer = "abc"
        var failing: Set<String> = []
        var seedRequests = 0

        mutating func answer(_ request: ProcedureServiceRequest) -> ProcedureServiceResponse {
            requests.append(request)
            let name = request.service + (request.subfunc ?? "")
            if request.subfunc == "0x01" { seedRequests += 1 }
            if failing.contains(name) { return ProcedureServiceResponse(success: false) }
            return ProcedureServiceResponse(success: true, data: request.subfunc == "0x01" ? seedAnswer : "")
        }
    }

    private func run(_ session: inout Session, address: UInt32 = 42) -> ProcedureRunResult {
        var state = session
        let result = ProcedureSecurityAccess.execute(
            { request in state.answer(request) },
            { seed in Array(seed.reversed()) },
            address
        )
        session = state
        return result
    }

    func testAProcedureSendsEachServiceItsRequestAndEndsInDone() {
        var session = Session()
        let result = run(&session)
        XCTAssertEqual(result.finalState, "done")
        XCTAssertEqual(result.doneData, ["result": "success"])
        XCTAssertEqual(
            session.requests,
            [
                ProcedureServiceRequest(service: "TesterPresent", addr: "42"),
                ProcedureServiceRequest(service: "SecurityAccess", subfunc: "0x01"),
                // The payload is what the helper made of the seed the service answered.
                ProcedureServiceRequest(
                    service: "SecurityAccess", subfunc: "0x02", payload: Array("cba".utf8)),
            ])
    }

    func testAnAddressIsSentAsDecimalText() {
        var session = Session()
        _ = run(&session, address: 4_000_000_000)
        XCTAssertEqual(session.requests.first?.addr, "4000000000")
    }

    func testAFailedFirstServiceEndsInTheErrorState() {
        var session = Session()
        session.failing = ["TesterPresent"]
        let result = run(&session)
        XCTAssertEqual(result.finalState, "error")
        XCTAssertEqual(result.doneData, ["result": "failure"])
        XCTAssertEqual(session.requests.count, 1)
    }

    // `retry` is entered on a failed seed request and goes back to it while the count is below the
    // maximum of three, so the seed is requested four times before the procedure gives up.
    func testARetryIsBoundedByTheCountTheAssignKeeps() {
        var session = Session()
        session.failing = ["SecurityAccess0x01"]
        let result = run(&session)
        XCTAssertEqual(result.finalState, "error")
        XCTAssertEqual(session.seedRequests, 4)
    }

    // A seed longer than the 64 bytes its variable holds is not stored: the assign raises
    // `error.execution`, which no transition of the source state handles, so the procedure ends
    // short of a final state.
    func testAValueOverTheBytesCapRaisesAnErrorAndEndsTheRun() {
        var session = Session()
        session.seedAnswer = String(repeating: "x", count: 65)
        let result = run(&session)
        XCTAssertFalse(result.completed)
        XCTAssertEqual(result.finalState, "")
        XCTAssertEqual(result.doneData, [:])
    }

    func testAValueAtTheBytesCapIsStored() {
        var session = Session()
        session.seedAnswer = String(repeating: "x", count: 64)
        XCTAssertEqual(run(&session).finalState, "done")
    }

    // `crossfile_procedure_filter.scxml`: a procedure that imports a low-pass filter holds one, builds
    // it with the class, and asks it for the next sample in the transition to its final state.
    func testAProcedureThatHoldsAFilterRunsToItsFinalStateThroughIt() {
        let procedure = CrossfileProcedureFilter()
        procedure.setRawSample(1.5)
        let result = procedure.runToCompletion()
        XCTAssertTrue(result.completed)
        XCTAssertEqual(result.finalState, "done")
        XCTAssertEqual(result.doneData, ["result": "success"])
    }
}
