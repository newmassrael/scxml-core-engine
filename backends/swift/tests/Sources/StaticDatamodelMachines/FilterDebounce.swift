// SCE-GENERATED — DO NOT EDIT
// source-hash: 88c3d475be4b704e681e435f769b6c1e874023c075725ce0d093a310a81ad11d
// SCE-MAP: filter_debounce.scxml:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// A signal filter: a class whose `update` reads one sample and answers the filtered one, and
// whose `reset` forgets what it has seen. It holds its history, so each holder of one instance
// shares it. The runtime's filter is named by its module, so a document named for one of the
// runtime's classes (`moving_average`) cannot hide it.

import SceForgeRuntime

public final class FilterDebounce {
    private let sceImpl = SceForgeRuntime.Debounce<Bool>(window: 3)

    public init() {}

    public func update(_ rawButton: Bool) -> Bool {
        sceImpl.update(Bool(rawButton))
    }

    public func reset() {
        sceImpl.reset()
    }
}
