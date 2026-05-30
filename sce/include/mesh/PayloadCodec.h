// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh PayloadCodec — wire-stable payload encoding discriminator.
//
// Identifies how MeshEnvelope.data bytes were encoded so the receiver can
// dispatch to the matching decoder. Values are IMMUTABLE once shipped; see
// SCE_MESH.md Section 13 Phase 3.5 for the two-layer wire design (CBOR
// envelope + per-event payload codec). Serialized into envelope key 4 as
// CBOR uint8. CloudEvents v1.0 'datacontenttype' equivalent.

#pragma once

#include <cstdint>

namespace SCE::Mesh {

enum class PayloadCodec : uint8_t {
    None  = 0,  // payload absent (FireForget control messages)
    Json  = 1,  // default; §scxml-5.10 _event.data
    Cbor  = 2,  // structured, smaller than JSON
    Typed = 3,  // codegen-emitted binary using event schema
    Raw   = 4,  // user-supplied encoder (escape hatch)
};

static_assert(static_cast<uint8_t>(PayloadCodec::None)  == 0, "PayloadCodec wire value changed");
static_assert(static_cast<uint8_t>(PayloadCodec::Json)  == 1, "PayloadCodec wire value changed");
static_assert(static_cast<uint8_t>(PayloadCodec::Cbor)  == 2, "PayloadCodec wire value changed");
static_assert(static_cast<uint8_t>(PayloadCodec::Typed) == 3, "PayloadCodec wire value changed");
static_assert(static_cast<uint8_t>(PayloadCodec::Raw)   == 4, "PayloadCodec wire value changed");

}  // namespace SCE::Mesh
