// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_LOOKUP_ENGINE_STATUS_H
#define SCE_FORGE_LOOKUP_ENGINE_STATUS_H

#include <cstdint>

namespace SCE::Generated::LookupEngineStatus {

enum class Status { STOP, RUNNING, FAULT };

inline Status lookupStatus(uint8_t engSta) {
    switch (engSta) {
        case 0x07:
            return Status::FAULT;
        case 0x03:
            return Status::RUNNING;
        case 0x00:
        case 0x01:
        case 0x02:
            return Status::STOP;
        default: return Status::STOP;
    }
}

}  // namespace SCE::Generated::LookupEngineStatus

#endif  // SCE_FORGE_LOOKUP_ENGINE_STATUS_H
