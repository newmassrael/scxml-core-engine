// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_FILTER_LOW_PASS_H
#define SCE_FORGE_FILTER_LOW_PASS_H

#include <cstdint>
#include <array>

namespace SCE::Generated::FilterLowPass {

struct FilterLowPass {
    double prev_ = {};
    bool initialized_ = false;

    double update(double rawSignal) {
        if (!initialized_) {
            prev_ = static_cast<double>(rawSignal);
            initialized_ = true;
            return prev_;
        }
        prev_ = static_cast<double>(0.1) * static_cast<double>(rawSignal) + (1.0 - static_cast<double>(0.1)) * prev_;
        return prev_;
    }

    void reset() {
        prev_ = {};
        initialized_ = false;
    }
};

}  // namespace SCE::Generated::FilterLowPass

#endif  // SCE_FORGE_FILTER_LOW_PASS_H