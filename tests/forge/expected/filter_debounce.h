// SCE-MAP: filter_debounce:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_FILTER_DEBOUNCE_H
#define SCE_FORGE_FILTER_DEBOUNCE_H

#include <cstdint>
#include <sce/forge/filter.h>

namespace SCE::Generated::FilterDebounce {

struct FilterDebounce {
    SCE::Forge::Debounce<bool, 3> impl_;

    bool update(bool rawButton) {
        return impl_.update(static_cast<bool>(rawButton));
    }

    void reset() {
        impl_.reset();
    }
};

}  // namespace SCE::Generated::FilterDebounce

#endif  // SCE_FORGE_FILTER_DEBOUNCE_H
