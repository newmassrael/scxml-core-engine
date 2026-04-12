// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_FILTER_LOW_PASS_H
#define SCE_FORGE_FILTER_LOW_PASS_H

#include <cstdint>
#include <sce/forge/filter.h>

namespace SCE::Generated::FilterLowPass {

struct FilterLowPass {
    sce::forge::LowPass<double> impl_{static_cast<double>(0.1)};

    double update(double rawSignal) {
        return impl_.update(static_cast<double>(rawSignal));
    }

    void reset() {
        impl_.reset();
    }
};

}  // namespace SCE::Generated::FilterLowPass

#endif  // SCE_FORGE_FILTER_LOW_PASS_H
