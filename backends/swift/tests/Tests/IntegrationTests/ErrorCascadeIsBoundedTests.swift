// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: a handler that answers its own failure with the same failure is stopped, and
// the host is told. Swift AOT path.
//
// Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python / Kotlin channels).
//
// Regeneration: scripts/regen_swift_integration.sh

import IntegrationMachines
import SceLua
import SceRuntime
import Testing

/// The ceiling the engine applies, spelled here rather than read back from it. A test that asked
/// the engine for its own limit would agree with any limit, including one an edit moved by three
/// orders of magnitude.
private let maxLinks: Int64 = 100

/// The fixture counts handler runs with `<assign>`, so this is an ECMAScript-datamodel machine.
private func started() -> (
    policy: ErrorCascadeIsBoundedPolicy, engine: Engine<ErrorCascadeIsBoundedPolicy>
) {
    let policy = ErrorCascadeIsBoundedPolicy()
    let engine = Engine(policy: policy)
    startRecordingARefusal(engine)
    return (policy, engine)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct ErrorCascadeIsBounded {
    /// The axis: the chain is cut at a known link, and the count says it was.
    @Test func aHandlerThatCannotHandleItsErrorIsStopped() {
        let (policy, engine) = started()
        #expect(
            engine.errorCascadeEvents == 0,
            "nothing has been refused before the machine has done anything")

        engine.processEvent(.spin)

        #expect(
            policy.readScriptValue("runs") == .integer(maxLinks),
            "`runaway`'s handler must run exactly as many times as the engine allows links in a chain: fewer means the document was cut off early, more means the ceiling moved"
        )
        #expect(
            policy.readScriptValue("ticks") == .integer(maxLinks),
            "every link's handler also raises the author's own `tick`, and every one of them must be delivered: an engine that counted those as links would refuse at half the depth"
        )
        #expect(
            engine.errorCascadeEvents == 1,
            "the handler's <assign> failed again on the last allowed link, and the error it raised is the one the engine refused to queue"
        )
        #expect(
            engine.lastErrorCascadeEvent == .errorExecution,
            "a count alone does not name the repair: error.execution is a handler whose own content fails"
        )
        #expect(
            engine.getCurrentState() == .runaway,
            "the handler is targetless, so nothing here may move the machine"
        )
    }

    /// The other half, and the one that makes the count mean something: a chain that ends by
    /// itself must pass through untouched.
    @Test func aChainThatEndsOnItsOwnIsNotRefused() {
        let (policy, engine) = started()

        engine.processEvent(.settle)

        #expect(
            policy.readScriptValue("repairs") == .integer(3),
            "`settling`'s handler repairs three times and then its `repairs < 3` guard stops matching: three links is what a real repair strategy looks like, and the engine must not have interrupted it"
        )
        #expect(
            engine.errorCascadeEvents == 0,
            "nothing was refused: the chain ended on the document's own terms; a ceiling that fired here would report every document that fails often as one that cannot stop failing"
        )
        #expect(
            engine.lastErrorCascadeEvent == nil, "nothing was refused, so there is no last one to name")
        #expect(
            engine.unhandledErrorEvents == 1,
            "the fourth error found no matching transition once the guard closed, which is the ordinary clause"
        )
    }

    /// A single failure with nobody to answer it is not a chain.
    @Test func oneErrorNobodyAnsweredIsNotAChain() {
        let (_, engine) = started()

        for _ in 0..<5 { engine.processEvent(.boom) }

        #expect(
            engine.unhandledErrorEvents == 5,
            "five failures, none of them answered: the clause's own case")
        #expect(
            engine.errorCascadeEvents == 0,
            "no handler ran, so no handler raised anything: a count keyed off how OFTEN a document fails would already be at five here"
        )
    }

    /// Cutting the chain must not cost the document the states that work.
    @Test func theMachineStillAnswersAfterItsChainIsCut() {
        let (policy, engine) = started()

        engine.processEvent(.spin)
        #expect(
            engine.errorCascadeEvents == 1,
            "precondition: this test is about what happens AFTER a refusal")

        engine.processEvent(.poke)

        #expect(
            policy.readScriptValue("pokes") == .integer(1),
            "`runaway` answers `poke` with a targetless transition, and it ran: an engine that stopped the machine to end the chain would leave the host with a dead document instead of a bounded one"
        )
        #expect(
            engine.errorCascadeEvents == 1,
            "`poke` raises nothing, so the count that was already there is all there is: the refusal is a fact about the past, not a mode"
        )
    }

    /// The depth is a property of the chain, not of the machine's whole life.
    @Test func aSecondChainStartsFromZero() {
        let (policy, engine) = started()

        engine.processEvent(.spin)
        engine.processEvent(.reset)
        #expect(
            engine.getCurrentState() == .idle, "`reset` is the fixture's way back out of the chain")

        engine.processEvent(.spin)

        #expect(
            policy.readScriptValue("runs") == .integer(2 * maxLinks),
            "the second entry into `runaway` must buy the document a full chain again: a depth carried across the drains would stop this one at its first link"
        )
        #expect(
            engine.errorCascadeEvents == 2,
            "two chains, two refusals: a count that saturates at one would read as a machine that recovered"
        )
    }
}
