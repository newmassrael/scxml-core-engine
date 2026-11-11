// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import StaticDatamodelMachines

// A Swift algorithm that takes a union reads each case's fields where a test of its tag proves the case, and one that
// returns a union hands back the one it was given or the case it builds (docs/adr/0092, Landing order step 5), run
// here over the committed units `AlgorithmShapeExtent.swift` and `AlgorithmShapeGrown.swift`, generated from
// `sce-build/tests/fixtures/union/` beside the union `UnionShapeMinimal`, the enum of its tag and the records its
// cases hold.

private let circle = UnionShapeMinimal.circle(ShapeRadiusMinimalPayload(radius: 9))
private let rect = UnionShapeMinimal.rect(ShapeRectMinimalPayload(width: 4, height: 11))
private let wide = UnionShapeMinimal.rect(ShapeRectMinimalPayload(width: 12, height: 5))

@Test func aDotHasNoExtent() {
    #expect(AlgorithmShapeExtent.algorithmShapeExtent(UnionShapeMinimal.dot) == 0)
}

@Test func aCircleReadsItsRadiusAsItsExtent() {
    #expect(AlgorithmShapeExtent.algorithmShapeExtent(circle) == 9)
}

@Test func aRectReadsTheLongerOfItsTwoSides() {
    #expect(AlgorithmShapeExtent.algorithmShapeExtent(rect) == 11)
    #expect(AlgorithmShapeExtent.algorithmShapeExtent(wide) == 12)
}

@Test func aDotIsGrownIntoACircleOfTheRadiusGiven() {
    let grown = AlgorithmShapeGrown.algorithmShapeGrown(UnionShapeMinimal.dot, 6)
    #expect(grown == UnionShapeMinimal.circle(ShapeRadiusMinimalPayload(radius: 6)))
    #expect(AlgorithmShapeExtent.algorithmShapeExtent(grown) == 6)
}

@Test func aShapeThatIsNotADotIsReturnedAsItIs() {
    #expect(AlgorithmShapeGrown.algorithmShapeGrown(circle, 3) == circle)
    #expect(AlgorithmShapeGrown.algorithmShapeGrown(rect, 3) == rect)
}
