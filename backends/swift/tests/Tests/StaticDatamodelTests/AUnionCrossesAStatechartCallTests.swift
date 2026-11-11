// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// A statechart hands an algorithm a union and takes one back (docs/adr/0092, section 6.1 item 7): each call maps
// the machine's own sum into the enum the union's unit declares, case by case, and back. `shape` is held in
// the machine's own `StatechartShapeCrossingUnionShapeMinimalUnion` and starts as a circle of radius 2; on `grow` it
// is handed to `AlgorithmShapeGrown`, which returns any shape that is not a dot as it is, and its extent is asked of
// `AlgorithmShapeExtent`, which reads the radius of a circle.
//
// Fixture: sce-build/tests/fixtures/union/statechart_shape_crossing.scxml

private func started() -> (StatechartShapeCrossingPolicy, Engine<StatechartShapeCrossingPolicy>) {
    let policy = StatechartShapeCrossingPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    engine.initialize()
    return (policy, engine)
}

@Test func theMachineHoldsItsShapeAsTheCaseTheDocumentSetsBeforeAnyCall() {
    let (policy, _) = started()
    #expect(policy.shape == .circle(StatechartShapeCrossingRadiusRecord(radius: 2)))
    #expect(policy.extent == 0)
}

@Test func aShapeCrossesToTheAlgorithmAndBackInTheCaseItWasIn() {
    let (policy, engine) = started()
    engine.raiseExternalByName("grow", eventData: "")
    engine.step()
    #expect(policy.shape == .circle(StatechartShapeCrossingRadiusRecord(radius: 2)))
}

@Test func theExtentTheAlgorithmReadsIsOfTheCaseTheMachineHoldsAcrossTheCall() {
    let (policy, engine) = started()
    engine.raiseExternalByName("grow", eventData: "")
    engine.step()
    #expect(policy.extent == 2)
}
