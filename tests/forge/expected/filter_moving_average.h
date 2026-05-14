// SCE-MAP: filter_moving_average:1

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_FILTER_MOVING_AVERAGE_H
#define SCE_FORGE_FILTER_MOVING_AVERAGE_H

#include <cstdint>
#include <sce/forge/filter.h>

namespace SCE::Generated::FilterMovingAverage {

struct FilterMovingAverage {
    SCE::Forge::MovingAverage<double, 5> impl_;

    double update(double rawTemp) {
        return impl_.update(static_cast<double>(rawTemp));
    }

    void reset() {
        impl_.reset();
    }
};

}  // namespace SCE::Generated::FilterMovingAverage

#endif  // SCE_FORGE_FILTER_MOVING_AVERAGE_H
