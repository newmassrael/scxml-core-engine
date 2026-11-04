// SCE-GENERATED — DO NOT EDIT
// source-hash: 896f057b8d7ef2f44417d80bfa021e20b7ff6af25c9f5a861b7dde3958837be5
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
