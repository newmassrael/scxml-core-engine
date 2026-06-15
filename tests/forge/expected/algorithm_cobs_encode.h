// SCE-MAP: algorithm_cobs_encode:32

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: free function in `namespace SCE::Generated::<Name>`. No
// STL containers, no exceptions. `bytes` lowers to `std::span<const
// std::uint8_t>` (RFC §synth-5-J-5 emitter table).

#pragma once
#ifndef SCE_FORGE_ALGORITHM_COBS_ENCODE_H
#define SCE_FORGE_ALGORITHM_COBS_ENCODE_H

#include <cstddef>
#include <cstdint>
#include <vector>
#include <span>
namespace SCE::Generated::AlgorithmCobsEncode {

inline std::vector<std::uint8_t> algorithm_cobs_encode(std::span<const std::uint8_t> data) {
    uint16_t n = (data).size();
    std::vector<std::uint8_t> out;
    uint16_t p = 0;
    bool done = false;
    while (done == false) {
        uint16_t q = p;
        while (q < n && q - p < 254 && data[q] != 0) {
            q = q + 1;
        }
        uint16_t run = q - p;
        uint8_t code = run + 1;
        out.push_back(static_cast<std::uint8_t>(code));
        uint16_t k = p;
        while (k < q) {
            out.push_back(static_cast<std::uint8_t>(data[k]));
            k = k + 1;
        }
        if (q >= n) {
            done = true;
        } else {
            if (run < 254) {
                p = q + 1;
                if (p >= n) {
                    uint8_t last = 1;
                    out.push_back(static_cast<std::uint8_t>(last));
                    done = true;
                }
            } else {
                p = q;
            }
        }
    }
    return out;
}

}  // namespace SCE::Generated::AlgorithmCobsEncode

#endif  // SCE_FORGE_ALGORITHM_COBS_ENCODE_H
