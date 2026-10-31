// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import EcmaScriptMachines
import SceRuntime
import Testing

// What the generated machines of `sce-build/tests/fixtures/swift_ecmascript` do under their `Engine`:
// documents of `datamodel="ecmascript"` whose `<data>` initialisers and transition guards are lowered to
// Lua at build time and run by the `SceLua` engine. The four W3C documents are the oracle for the
// language; the rest hold what no W3C document asks: a guard that mixes In() with an expression, a guard
// that reads the event being handled, a guard that cannot be evaluated, and the session variables.

private func started<Policy: StatePolicy>(_ policy: Policy) -> Engine<Policy> {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    engine.initialize()
    return engine
}

// MARK: - The W3C documents

// W3C 444: `<data>` creates an ECMAScript variable, and a guard can use it, here with the increment
// `++var1`, which the lowering turns into a write and a read.
@Test func aDataVariableIsAnEcmaScriptVariableAGuardCanIncrement() {
    let engine = started(Test444Policy())
    #expect(engine.getCurrentState() == .pass)
    #expect(engine.isInFinalState)
}

// W3C 321: `_sessionid` is bound on startup.
@Test func theSessionIdIsBoundOnStartup() {
    let engine = started(Test321Policy())
    #expect(engine.getCurrentState() == .pass)
}

// W3C 279 and 550: early binding assigns the variables of a state that is never visited at load.
@Test func earlyBindingGivesAStateLocalVariableItsValueBeforeTheStateIsVisited() {
    #expect(started(Test279Policy()).getCurrentState() == .pass)
}

@Test func anExplicitEarlyBindingDoesTheSame() {
    #expect(started(Test550Policy()).getCurrentState() == .pass)
}

// MARK: - A guard that mixes In() with an expression

@Test func aGuardThatMixesInWithAnExpressionWaitsForTheStateAndHoldsTheVariable() {
    let engine = started(EcmaInGuardPolicy())
    #expect(Set(engine.getActiveStates()).isSuperset(of: [.r1a, .r2a]))

    // Region one has not moved, so In('r1b') is false and the guard is too.
    engine.processEvent(.check)
    #expect(engine.getActiveStates().contains(.r2a))

    engine.processEvent(.go)
    #expect(engine.getActiveStates().contains(.r1b))
    engine.processEvent(.check)
    #expect(engine.getActiveStates().contains(.r2done))
}

// MARK: - A guard that reads the event

@Test func aGuardReadsTheFieldsOfThePayloadOfTheEventBeingHandled() {
    let above = started(EcmaEventGuardPolicy())
    above.processEventWithMeta(.reading, metadata: EventMetadata(data: #"{"level": 9}"#))
    #expect(above.getCurrentState() == .alarm)

    let below = started(EcmaEventGuardPolicy())
    below.processEventWithMeta(.reading, metadata: EventMetadata(data: #"{"level": 1}"#))
    #expect(below.getCurrentState() == .calm)
}

@Test func aGuardReadsTheNameAndTheTypeOfTheEvent() {
    // No payload: the first guard cannot be read and is an error, so the second answers, and it
    // reads `_event.name` and `_event.type`.
    let engine = started(EcmaEventGuardPolicy())
    engine.processEvent(.reading)
    #expect(engine.getCurrentState() == .calm)
}

// MARK: - A guard that cannot be evaluated

@Test func aGuardThatCannotBeEvaluatedIsFalseAndRaisesErrorExecution() {
    let engine = started(EcmaGuardErrorPolicy())
    // The cond is not taken, and the error event it raised is what moved the machine.
    #expect(engine.getCurrentState() == .raised)
}

// MARK: - The session variables

@Test func theSessionVariablesAreTheOnesTheHostGave() throws {
    let policy = EcmaSessionPolicy()
    try policy.setSessionID("probe-1")
    #expect(started(policy).getCurrentState() == .match)
}

@Test func aSessionThatTheHostNamedDifferentlyDoesNotMatch() throws {
    let policy = EcmaSessionPolicy()
    try policy.setSessionID("another")
    #expect(started(policy).getCurrentState() == .other)
}

// MARK: - The feature flags

@Test func aMachineThatNeedsAnEngineSaysSo() {
    #expect(Test444Policy().needsScriptEngine)
    #expect(Test444Policy().needsDataModelInit)
}
