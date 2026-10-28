// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// The scalar core of `datamodel="sce-static"`: a variable is a property of the policy,
// and guards, `<assign>`, `<if>`, `<elseif>`, `<log>`, `<raise>` and `In()` are native.
//
// Each scenario is replayed by one line, `replay("<scenario>", <machine>())`, which is
// what `every_scenario_is_replayed_on_every_backend_that_lowers_its_machine` reads
// this file for.

// MARK: - The machines, with the variables their scenarios state

private func counter() -> Machine {
    let policy = StaticCounterPolicy()
    return drive(
        policy,
        variables: [
            "count": { integer(policy.vCount) },
            "ready": { .bool(policy.vReady) },
        ]
    )
}

private func eventArrival() -> Machine {
    let policy = StaticEventArrivalPolicy()
    return drive(
        policy,
        variables: [
            "requests": { integer(policy.requests) },
            "specials": { integer(policy.specials) },
        ]
    )
}

private func eventWildcard() -> Machine {
    let policy = StaticEventWildcardPolicy()
    return drive(
        policy,
        variables: [
            "requests": { integer(policy.requests) },
            "strays": { integer(policy.strays) },
        ]
    )
}

private func overflow() -> Machine {
    let policy = StaticOverflowPolicy()
    return drive(
        policy,
        variables: [
            "level": { integer(policy.level) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func stringCapacity() -> Machine {
    let policy = StaticStringCapacityPolicy()
    return drive(
        policy,
        variables: [
            "title": { .text(policy.title) },
            "body": { .text(policy.body) },
            "copied": { integer(policy.copied) },
            "measured": { integer(policy.measured) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func blockEnds() -> Machine {
    let policy = StaticBlockEndsPolicy()
    return drive(
        policy,
        variables: [
            "a": { integer(policy.a) },
            "b": { integer(policy.b) },
            "afterAssign": { integer(policy.afterAssign) },
            "thenRan": { integer(policy.thenRan) },
            "elseRan": { integer(policy.elseRan) },
            "afterIf": { integer(policy.afterIf) },
            "inBranch": { integer(policy.inBranch) },
            "afterBranch": { integer(policy.afterBranch) },
            "afterOk": { integer(policy.afterOk) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func history() -> Machine {
    let policy = StaticHistoryPolicy()
    return drive(policy, variables: ["resumed": { integer(policy.resumed) }])
}

private func timers() -> Machine {
    let policy = StaticTimersPolicy()
    return drive(policy, variables: ["trace": { integer(policy.trace) }])
}

private func sendDelay() -> Machine {
    let policy = StaticSendDelayPolicy()
    return drive(
        policy,
        variables: [
            "wait": { integer(policy.wait) },
            "beats": { integer(policy.beats) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func sendEvent() -> Machine {
    let policy = StaticSendEventPolicy()
    return drive(
        policy,
        variables: [
            "pings": { integer(policy.pings) },
            "pongs": { integer(policy.pongs) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func cancelExpr() -> Machine {
    let policy = StaticCancelExprPolicy()
    return drive(
        policy,
        variables: [
            "a_fired": { integer(policy.aFired) },
            "b_fired": { integer(policy.bFired) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func sendIdLocation() -> Machine {
    let policy = StaticSendIdlocationPolicy()
    return drive(
        policy,
        variables: [
            "first_fired": { integer(policy.firstFired) },
            "second_fired": { integer(policy.secondFired) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func sendTarget() -> Machine {
    let policy = StaticSendTargetPolicy()
    return drive(
        policy,
        variables: [
            "landed": { integer(policy.landed) },
            "refused": { integer(policy.refused) },
        ]
    )
}

private func sendType() -> Machine {
    let policy = StaticSendTypePolicy()
    return drive(
        policy,
        variables: [
            "landed": { integer(policy.landed) },
            "refused": { integer(policy.refused) },
        ]
    )
}

// MARK: - The scenarios

// A counter counts to its flag and lets go: guards, `<assign>`, `<if>` and `<elseif>`
// are native, and `In()` asks the machine's own configuration.
@Test func theCounterCountsToItsFlagAndLetsGo() {
    replay("static_counter", counter())
}

@Test func theCounterStopsAtItsBoundAndRefusesGo() {
    replay("static_counter_bound", counter())
}

// An event arrives by name from outside the document, so the names it can arrive
// under are not the ones the document writes: a name that extends one the document
// has is that event, and one it does not is dropped (§scxml-3.12.1).
@Test func anEventArrivesUnderANameTheDocumentDoesNotWrite() {
    replay("static_event_arrival", eventArrival())
}

@Test func anEventArrivesUnderANameOnlyTheWildcardTakes() {
    replay("static_event_wildcard", eventWildcard())
}

// A checked integer operation that overflows fails instead of wrapping: the
// statement writes nothing, `error.execution` is raised, and the block ends.
@Test func anOverflowingOperationFailsInsteadOfWrapping() {
    replay("static_overflow", overflow())
}

// A string variable holds no more than its bound, counted in UTF-8 bytes: the text
// that does not fit is refused, not cut.
@Test func aStringHoldsNoMoreThanItsBound() {
    replay("static_string_capacity", stringCapacity())
}

// A failing statement ends its block, and only its block: what stands after it in the
// same block does not run, and the next block does (§scxml-4.9).
@Test func aFailingStatementEndsItsBlockAndOnlyItsBlock() {
    replay("static_block_ends", blockEnds())
}

// A history remembers what its parent held when it was left, and a machine that
// resumes through it finds the variables it kept.
@Test func aHistoryResumesWhereItsParentWasLeft() {
    replay("static_history", history())
}

// MARK: - Sends and cancels

// A delayed `<send>` still waiting is part of a machine's state: armed on entering a
// state, delivered when its time comes, and removed by the `<cancel>` that names it.
@Test func aDelayedSendIsDeliveredWhenItsTimeComesAndACancelRemovesIt() {
    replay("static_timers", timers())
}

@Test func aCancelStopsTheTimersOfTheStateItLeaves() {
    replay("static_timers_stop", timers())
}

// The delay of a `<send>` is a string the machine computes, read as the CSS2 time it
// must be: a value that is no time, or an operation that fails, is refused.
@Test func aSendWaitsForADelayTheMachineComputes() {
    replay("static_send_delay", sendDelay())
}

@Test func aSendDeliversTheEventTheMachineNames() {
    replay("static_send_event", sendEvent())
}

@Test func aCancelNamesTheSendItRemovesByAComputedId() {
    replay("static_cancel_expr", cancelExpr())
}

@Test func aSendHandsItsIdToTheVariableItNames() {
    replay("static_send_idlocation", sendIdLocation())
}

@Test func aSendGoesOnlyByARouteTheDocumentDeclares() {
    replay("static_send_target", sendTarget())
}

@Test func aSendOfAComputedTypeGoesByAProcessorTheDocumentDeclares() {
    replay("static_send_type", sendType())
}
