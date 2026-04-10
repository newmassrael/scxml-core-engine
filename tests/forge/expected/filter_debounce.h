// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_FILTER_DEBOUNCE_H
#define SCE_FORGE_FILTER_DEBOUNCE_H

#include <cstdint>
#include <array>

namespace SCE::Generated::FilterDebounce {

struct FilterDebounce {
    bool stableValue_{};
    bool candidate_{};
    size_t count_ = 0;
    bool initialized_ = false;

    bool update(bool rawButton) {
        if (!initialized_) {
            stableValue_ = static_cast<bool>(rawButton);
            candidate_ = stableValue_;
            count_ = 1;
            initialized_ = true;
            return stableValue_;
        }
        if (static_cast<bool>(rawButton) == candidate_) {
            count_++;
            if (count_ >= 3) {
                stableValue_ = candidate_;
            }
        } else {
            candidate_ = static_cast<bool>(rawButton);
            count_ = 1;
        }
        return stableValue_;
    }

    void reset() {
        stableValue_ = {};
        candidate_ = {};
        count_ = 0;
        initialized_ = false;
    }
};

}  // namespace SCE::Generated::FilterDebounce

#endif  // SCE_FORGE_FILTER_DEBOUNCE_H