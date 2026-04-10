// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_FILTER_MOVING_AVERAGE_H
#define SCE_FORGE_FILTER_MOVING_AVERAGE_H

#include <cstdint>
#include <array>

namespace SCE::Generated::FilterMovingAverage {

struct FilterMovingAverage {
    std::array<double, 5> buffer_{};
    size_t index_ = 0;
    bool filled_ = false;

    double update(double rawTemp) {
        buffer_[index_] = static_cast<double>(rawTemp);
        index_ = (index_ + 1) % 5;
        if (!filled_ && index_ == 0) filled_ = true;
        size_t count = filled_ ? 5 : index_;
        double sum = 0;
        for (size_t i = 0; i < count; i++) sum += buffer_[i];
        return sum / static_cast<double>(count);
    }

    void reset() {
        buffer_ = {};
        index_ = 0;
        filled_ = false;
    }
};

}  // namespace SCE::Generated::FilterMovingAverage

#endif  // SCE_FORGE_FILTER_MOVING_AVERAGE_H