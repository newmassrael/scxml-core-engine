// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — filter
// Three signal-filter class templates matching SCE_FORGE.md §4.8:
//   MovingAverage<T, Window>  — sliding window arithmetic mean
//   LowPass<T>                — first-order exponential smoothing
//   Debounce<T, Window>       — require N consecutive identical samples
//
// All filters keep state in a fixed-size internal buffer (no heap).

#pragma once

#include <array>
#include <cstddef>

namespace SCE::Forge {

template <typename T, std::size_t Window> class MovingAverage {
public:
    static_assert(Window >= 1, "MovingAverage window must be >= 1");

    T update(T input) noexcept {
        buffer_[index_] = input;
        index_ = (index_ + 1) % Window;
        if (!filled_ && index_ == 0) {
            filled_ = true;
        }
        const std::size_t count = filled_ ? Window : index_;
        T sum = T{};
        for (std::size_t i = 0; i < count; ++i) {
            sum += buffer_[i];
        }
        return sum / static_cast<T>(count);
    }

    void reset() noexcept {
        buffer_ = {};
        index_ = 0;
        filled_ = false;
    }

private:
    std::array<T, Window> buffer_{};
    std::size_t index_ = 0;
    bool filled_ = false;
};

/// First-order exponential low-pass: y[n] = alpha * x[n] + (1 - alpha) * y[n-1].
/// On the first sample, y[0] = x[0] (no warm-up bias toward zero).
template <typename T> class LowPass {
public:
    explicit LowPass(T alpha) noexcept : alpha_(alpha) {}

    T update(T input) noexcept {
        if (!initialized_) {
            state_ = input;
            initialized_ = true;
        } else {
            state_ = alpha_ * input + (T{1} - alpha_) * state_;
        }
        return state_;
    }

    void reset() noexcept {
        state_ = T{};
        initialized_ = false;
    }

private:
    T alpha_;
    T state_ = T{};
    bool initialized_ = false;
};

/// Output latches to a new value only after `Window` consecutive identical
/// samples. Until the buffer fills, the most recent input passes through.
template <typename T, std::size_t Window> class Debounce {
public:
    static_assert(Window >= 1, "Debounce window must be >= 1");

    T update(T input) noexcept {
        buffer_[index_] = input;
        index_ = (index_ + 1) % Window;
        if (!filled_ && index_ == 0) {
            filled_ = true;
        }

        if (filled_) {
            bool stable = true;
            for (std::size_t i = 1; i < Window; ++i) {
                if (buffer_[i] != buffer_[0]) {
                    stable = false;
                    break;
                }
            }
            if (stable) {
                output_ = buffer_[0];
            }
        } else {
            output_ = input;
        }
        return output_;
    }

    void reset() noexcept {
        buffer_ = {};
        index_ = 0;
        filled_ = false;
        output_ = T{};
    }

private:
    std::array<T, Window> buffer_{};
    std::size_t index_ = 0;
    bool filled_ = false;
    T output_ = T{};
};

}  // namespace SCE::Forge
