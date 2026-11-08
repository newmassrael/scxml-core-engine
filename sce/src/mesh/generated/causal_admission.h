// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_admission.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CAUSAL_ADMISSION_H
#define SCE_FORGE_CAUSAL_ADMISSION_H

#include <cstdint>
#include <optional>

namespace SCE::Generated::CausalAdmission {

enum class CausalAdmission : uint8_t {
    Drop = 0,
    Deliver = 1,
    Hold = 2,
};

/// The carrier value this holds.
constexpr uint8_t to_underlying(CausalAdmission value) noexcept {
    return static_cast<uint8_t>(value);
}

/// The name the document declares for this value, or `nullptr` for a value
/// no variant names — one an `enum class` can hold whether or not its set is
/// open, since it holds every carrier value.
constexpr const char *declared_name(CausalAdmission value) noexcept {
    switch (value) {
    case CausalAdmission::Drop:
        return "drop";
    case CausalAdmission::Deliver:
        return "deliver";
    case CausalAdmission::Hold:
        return "hold";
    }
    return nullptr;
}

/// The variant `raw` declares, or `std::nullopt` — the declared set is
/// closed, so a value outside it is not a value of this type.
constexpr std::optional<CausalAdmission> from_underlying(uint8_t raw) noexcept {
    switch (raw) {
    case 0:
        return CausalAdmission::Drop;
    case 1:
        return CausalAdmission::Deliver;
    case 2:
        return CausalAdmission::Hold;
    default:
        return std::nullopt;
    }
}

}  // namespace SCE::Generated::CausalAdmission

#endif  // SCE_FORGE_CAUSAL_ADMISSION_H
