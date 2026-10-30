// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: filter_low_pass.scxml:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A signal filter: a class whose `update` reads one sample and answers the filtered one, and
// whose `reset` forgets what it has seen. It holds its history, so each holder of one instance
// shares it. The runtime's filter is named by its module, so a document named for one of the
// runtime's classes (`moving_average`) cannot hide it.

import SceForgeRuntime

public final class FilterLowPass {
    private let sceImpl = SceForgeRuntime.LowPass(alpha: 0.1)

    public init() {}

    public func update(_ rawSignal: Double) -> Double {
        Double(sceImpl.update(Double(rawSignal)))
    }

    public func reset() {
        sceImpl.reset()
    }
}
