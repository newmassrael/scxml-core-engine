// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The three signal filters of the `filter` kind (SCE_FORGE.md Section 4.8). Siblings:
// `com.sce.forge.runtime.MovingAverage`/`LowPass`/`Debounce` (Kotlin),
// `sce_forge_runtime::filter` (Rust) and `scealgorithm` (Go).
//
// Each is a class: a filter is a handle to state that the next sample reads, so two holders of
// one filter see one history, as they do on every other backend.

/// Sliding-window arithmetic mean.
public final class MovingAverage {
    private let window: Int
    private var buffer: [Double]
    private var index = 0
    private var filled = false

    /// A mean over the last `window` samples; a window below 1 holds no sample and is refused.
    public init(window: Int) {
        precondition(window >= 1, "window must be >= 1")
        self.window = window
        self.buffer = [Double](repeating: 0.0, count: window)
    }

    /// The mean of the samples held, `value` among them: of the `index` so far while the window
    /// is filling, and of the whole window after.
    public func update(_ value: Double) -> Double {
        buffer[index] = value
        index = (index + 1) % window
        if !filled && index == 0 {
            filled = true
        }
        let count = filled ? window : index
        var sum = 0.0
        for i in 0..<count {
            sum += buffer[i]
        }
        return sum / Double(count)
    }

    /// Forgets every sample.
    public func reset() {
        for i in 0..<window {
            buffer[i] = 0.0
        }
        index = 0
        filled = false
    }
}

/// First-order exponential low-pass: `y[n] = alpha * x[n] + (1 - alpha) * y[n-1]`.
///
/// The first sample is the output (`y[0] = x[0]`), so the output is not pulled toward zero
/// while the filter warms up.
public final class LowPass {
    private let alpha: Double
    private var state = 0.0
    private var initialized = false

    public init(alpha: Double) {
        self.alpha = alpha
    }

    public func update(_ value: Double) -> Double {
        if !initialized {
            state = value
            initialized = true
        } else {
            state = alpha * value + (1.0 - alpha) * state
        }
        return state
    }

    /// Forgets the output: the next sample is the first again.
    public func reset() {
        state = 0.0
        initialized = false
    }
}

/// The output latches to a new value only after `window` consecutive identical samples; until
/// the window has filled, the most recent input passes through.
public final class Debounce<T: Equatable> {
    private let window: Int
    private var buffer: [T?]
    private var index = 0
    private var filled = false
    private var output: T?

    /// A latch over `window` samples; a window below 1 holds no sample and is refused.
    public init(window: Int) {
        precondition(window >= 1, "window must be >= 1")
        self.window = window
        self.buffer = [T?](repeating: nil, count: window)
    }

    public func update(_ value: T) -> T {
        buffer[index] = value
        index = (index + 1) % window
        if !filled && index == 0 {
            filled = true
        }
        if filled {
            let first = buffer[0]
            var stable = true
            for i in 1..<window where buffer[i] != first {
                stable = false
                break
            }
            if stable {
                output = first
            }
        } else {
            output = value
        }
        // `output` is set on every call: to `value` while the window fills, and to the value the
        // window agreed on once it has, so it is never absent here.
        return SceOptional.proven(output)
    }

    /// Forgets every sample and the latched output.
    public func reset() {
        for i in 0..<window {
            buffer[i] = nil
        }
        index = 0
        filled = false
        output = nil
    }
}
