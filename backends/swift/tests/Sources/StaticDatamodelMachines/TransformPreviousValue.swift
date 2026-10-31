// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: transform_previous_value.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the final class `TransformPreviousValue` for each, a
// file of the module that calls them. A call is qualified by the class
// (`TransformPreviousValue.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.
//
// A document that reads a field through `previous()` keeps the values it reads from one
// activation to the next, so the unit is a class that holds them: its functions are still static,
// each taking the kept values as trailing parameters, and `update` is one
// activation, which computes every output from this activation's inputs and the kept values and
// only then replaces each kept value by what its field holds now.

public final class TransformPreviousValue {

    public static func computeTotal(_ sample: Int32, _ pressed: Bool, _ previous_sample: Int32, _ previous_pressed: Bool, _ previous_total: Int32) -> Int32 {
        previous_total + sample
    }

    public static func computeDelta(_ sample: Int32, _ pressed: Bool, _ previous_sample: Int32, _ previous_pressed: Bool, _ previous_total: Int32) -> Int32 {
        sample - previous_sample
    }

    public static func computeRising(_ sample: Int32, _ pressed: Bool, _ previous_sample: Int32, _ previous_pressed: Bool, _ previous_total: Int32) -> Bool {
        pressed && !previous_pressed
    }

    private var previous_sample: Int32 = 16
    private var previous_pressed: Bool = false
    private var previous_total: Int32 = 0

    public init() {}

    /// Every kept value back to its field's `sce:initial`.
    public func reset() {
        self.previous_sample = 16
        self.previous_pressed = false
        self.previous_total = 0
    }

    /// One activation.
    public func update(_ sample: Int32, _ pressed: Bool) -> TransformPreviousValueOutputs {
        let total = TransformPreviousValue.computeTotal(sample, pressed, self.previous_sample, self.previous_pressed, self.previous_total)
        let delta = TransformPreviousValue.computeDelta(sample, pressed, self.previous_sample, self.previous_pressed, self.previous_total)
        let rising = TransformPreviousValue.computeRising(sample, pressed, self.previous_sample, self.previous_pressed, self.previous_total)
        self.previous_sample = sample
        self.previous_pressed = pressed
        self.previous_total = total
        return TransformPreviousValueOutputs(total: total, delta: delta, rising: rising)
    }
}

/// Every output of one activation.
public struct TransformPreviousValueOutputs: Equatable, Sendable {
    public let total: Int32
    public let delta: Int32
    public let rising: Bool

    public init(total: Int32, delta: Int32, rising: Bool) {
        self.total = total
        self.delta = delta
        self.rising = rising
    }
}
