// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4, docs/adr/0098 -- Swift AOT path: a value an `<invoke>` hands its child, whose name is
// the id of a top-level `<data>` of the child, is that `<data>`'s initial value in place of its own.
// The `<data>`'s own expression is not evaluated, a state's `<data>` never takes a handed value,
// early- or late-bound, and a name no top-level `<data>` declares is not added. The child's
// top-level `<script>` and its initial `<onentry>` read what it took.
//
// Fixture: sce-build/tests/fixtures/swift_ecmascript/ecma_invoke_seeds.scxml, whose comment says
// what each variable of its two inline children holds. Each child is started here alone, handed
// its values by `acceptSeeds` as its parent would hand them, and its variables are read; the
// parent is also run whole, so the values its `<invoke>` hands are the ones the children take.
//
// Regeneration: scripts/regen_swift_ecmascript.sh

import EcmaScriptMachines
import SceLua
import SceRuntime
import Testing

private typealias Parent = EcmaInvokeSeedsPolicy
private typealias EarlyChild = EcmaInvokeSeedsSceSynthInvokeInvEarlyPolicy
private typealias LateChild = EcmaInvokeSeedsSceSynthInvokeInvLatePolicy

/// What the parent's `inv_early` hands: every name it evaluated, the two the child declares no
/// top-level `<data>` for (`kept`, a state's, and `stray`, nobody's) among them.
private let earlyHanded: [String: LuaSeed] = [
    "failing": .scalar(.string("handed")),
    "counted": .scalar(.integer(7)),
    "obj": .json(#"{"name":"box","items":["a","b"]}"#),
    "kept": .scalar(.string("handed")),
    "stray": .scalar(.string("handed")),
]

/// What the parent's `inv_late` hands: `handed`, a top-level `<data>`, and `kept`, a state's.
private let lateHanded: [String: LuaSeed] = [
    "handed": .scalar(.string("handed")),
    "kept": .scalar(.string("handed")),
]

/// The machine `policy` describes, started on a manual clock. A start refused for its script session
/// (docs/adr/0096) is recorded as an issue of the test that asked.
private func started<Policy: StatePolicy>(_ policy: Policy) -> Engine<Policy> {
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    do {
        try engine.initialize()
    } catch {
        Issue.record("the machine's start was refused: \(error)")
    }
    return engine
}

/// Whether the Lua expression `expression` holds in the child's session: a number is compared there,
/// so an integer and a float of one value are one answer.
private func holds(_ read: (String) -> LuaValue?, _ expression: String) -> Bool {
    read(expression) == .boolean(true)
}

// One suite a stem, so two stems that name a test alike do not redeclare one function of the module.
@Suite struct AChildTakesAHandedValueInPlaceOfItsData {
    /// The control: a child handed nothing evaluates its own values, so the fixture's own expressions
    /// do raise and do have their effect. Without it the cases below could pass over a fixture whose
    /// expressions never fail.
    @Test func aChildHandedNothingEvaluatesItsOwnValues() {
        let child = EarlyChild()
        _ = started(child)
        let read = child.readScriptValue
        #expect(holds(read, "errors == 1"), "the own expression of `failing` raised error.execution once: errors=\(String(describing: read("errors")))")
        #expect(holds(read, "effects == 1"), "the own expression of `counted` had its effect: effects=\(String(describing: read("effects")))")
        #expect(read("type(failing)") == .string("nil"), "a <data> whose expression failed is left undefined")
        #expect(read("kept") == .string("own"))
        #expect(read("objWhole") == .boolean(false))
    }

    @Test func aHandedValueIsDeclaredInPlaceOfTheOwnValueWhichIsNotEvaluated() {
        let child = EarlyChild()
        child.acceptSeeds(earlyHanded)
        _ = started(child)
        let read = child.readScriptValue
        #expect(read("failing") == .string("handed"), "failing=\(String(describing: read("failing")))")
        #expect(read("counted") == .integer(7), "counted=\(String(describing: read("counted")))")
        #expect(
            holds(read, "errors == 0"),
            "the own expression of `failing` raised although a value took its place: errors=\(String(describing: read("errors")))")
        #expect(
            holds(read, "effects == 0"),
            "the own expression of `counted` had its effect although a value took its place: effects=\(String(describing: read("effects")))")
    }

    @Test func aStatesDataTakesNoHandedValue() {
        let early = EarlyChild()
        early.acceptSeeds(earlyHanded)
        _ = started(early)
        #expect(early.readScriptValue("kept") == .string("own"), "an early-bound state <data> took a handed value")

        let late = LateChild()
        late.acceptSeeds(lateHanded)
        _ = started(late)
        #expect(late.readScriptValue("kept") == .string("own"), "a late-bound state <data> took a handed value")
        #expect(late.readScriptValue("handed") == .string("handed"), "the late-bound child's top-level <data> took its value")
    }

    @Test func aNameNoTopLevelDataDeclaresIsNotAdded() {
        let child = EarlyChild()
        child.acceptSeeds(earlyHanded)
        _ = started(child)
        #expect(child.readScriptValue("type(stray)") == .string("nil"), "a handed name the child does not declare became a variable")
    }

    @Test func anObjectArrivesWhole() {
        let child = EarlyChild()
        child.acceptSeeds(earlyHanded)
        _ = started(child)
        #expect(child.readScriptValue("objWhole") == .boolean(true), "the top-level script did not find the object's shape")
    }

    @Test func theTopLevelScriptAndTheInitialOnentryReadTheHandedValue() {
        let child = EarlyChild()
        child.acceptSeeds(earlyHanded)
        _ = started(child)
        #expect(child.readScriptValue("seenByScript") == .string("handed"), "the top-level <script> ran before the value was declared")
        #expect(child.readScriptValue("seenByEntry") == .string("handed"), "the initial <onentry> ran before the value was declared")
    }

    /// The values are the start's that took them: a later start of the same machine begins from the
    /// document's own values.
    @Test func aHandedValueIsTakenByOneStart() throws {
        let child = EarlyChild()
        child.acceptSeeds(earlyHanded)
        let engine = started(child)
        #expect(child.readScriptValue("failing") == .string("handed"))

        engine.stop()
        try engine.initialize()
        let read = child.readScriptValue
        #expect(read("type(failing)") == .string("nil"), "a second start took the first start's value: failing=\(String(describing: read("failing")))")
        #expect(holds(read, "effects == 1"), "a second start skipped the own expression: effects=\(String(describing: read("effects")))")
    }

    /// A handed value the session cannot hold is that `<data>`'s initialization error: one
    /// error.execution, the `<data>` left as one whose own expression failed, and the others taken.
    @Test func aHandedValueTheSessionCannotHoldIsTheDatasInitializationError() {
        var handed = earlyHanded
        handed["obj"] = .json("{")
        let child = EarlyChild()
        child.acceptSeeds(handed)
        _ = started(child)
        let read = child.readScriptValue
        #expect(holds(read, "errors == 1"), "errors=\(String(describing: read("errors")))")
        #expect(read("type(obj)") == .string("nil"), "obj=\(String(describing: read("obj")))")
        #expect(read("failing") == .string("handed"), "the other values were still taken")
    }

    /// The parent hands every name it evaluated, and each child reports what it took.
    @Test func theParentHandsEveryNameAndEachChildTakesWhatItDeclares() {
        let engine = started(Parent())
        for _ in 0..<50 where !engine.isInFinalState {
            engine.tick()
        }
        #expect(
            engine.terminalState == .pass,
            "ended in \(String(describing: engine.terminalState)). failOwnValueRaised / failOwnValueEvaluated: a handed top-level <data> evaluated its own expression. failStateDataTookTheValue / failLateStateDataTookTheValue: a state's <data> took a handed value. failUnmatchedNameAdded: a name no top-level <data> declares became a variable. failObjectNotWhole: the object lost its shape. failHandedValueLost / failLateHandedValueLost: a top-level <data> kept its own value. failStartReadTheOwnValue: the top-level <script> or the initial <onentry> ran before the value was declared."
        )
    }
}
