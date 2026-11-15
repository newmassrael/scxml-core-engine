// SCE-GENERATED — DO NOT EDIT
// source-hash: 9744814bbcba25096b3a8566c61a417ad0df1ffce3ba2396694e4204f8dd4a7d
// SCE-MAP: filter_moving_average.scxml:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A signal filter: a class whose `update` reads one sample and answers the filtered one, and
// whose `reset` forgets what it has seen. It holds its history, so each holder of one instance
// shares it. The runtime's filter is named by its module, so a document named for one of the
// runtime's classes (`moving_average`) cannot hide it.

import SceForgeRuntime

public final class FilterMovingAverage {
    private let sceImpl = SceForgeRuntime.MovingAverage(window: 5)

    public init() {}

    public func update(_ rawTemp: Double) -> Double {
        Double(sceImpl.update(Double(rawTemp)))
    }

    public func reset() {
        sceImpl.reset()
    }
}
