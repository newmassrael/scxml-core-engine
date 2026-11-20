// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE C11 runtime: the `_event.data` of an `error.*` event a machine raises
// for itself, the record SCE_MESH.md section 10.7.1 pins (docs/adr/0106,
// #111): `errorName` ("execution" or "communication"), `reason` (the code)
// and `detail` (the sentence the raise site wrote).
//
// The code is a member of the enum generated from
// sce:std/engine/error_reason.scxml into sce/generated
// (tools/codegen/trees/runtime-c.toml), `ErrorReason_t`, spelled by its
// declared-name accessor, so the document is the one spelling of it.
//
// A machine has no allocator to build the record with where the error is
// raised, so the event carries the code beside the sentence (one byte of the
// event record, `error_reason`), and the record is written where `_event.data`
// is handed to a data model: the script engine's binding writes it into
// storage of the engine's own (`sce/error_data_text.h`). This header is what a
// machine's core reads of the code, and includes nothing hosted
// (docs/adr/0110, D1).
//
// Header-only, C11, and consumable from C++ (the gate in
// `cxx_consumption_gate.cmake`).

#ifndef SCE_ERROR_DATA_H
#define SCE_ERROR_DATA_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce/generated/error_reason.h"

#ifdef __cplusplus
extern "C" {
#endif

/* The `error_reason` of an event no engine error raised: every other event,
   and an `error.execution` a host raised with data of its own (a host that
   declines to start an `<invoke>` owns the shape of its refusal). No member of
   the vocabulary has the value 0, so `error_reason_from_underlying` answers
   false for it. */
#define SCE_ERROR_REASON_NONE 0u

/* The first value of the communication half of the vocabulary: 1 to 63 are
   raised as `error.execution`, 64 to 127 as `error.communication`. */
#define SCE_ERROR_COMMUNICATION_REASONS_FROM 64u

/* The `errorName` of an `error.*` event raised for `reason`: read from the
   code's value range, so a code and the event it is raised as cannot disagree. */
static inline const char *sce_error_name(ErrorReason_t reason) {
    return error_reason_to_underlying(reason) < SCE_ERROR_COMMUNICATION_REASONS_FROM ? "execution" : "communication";
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_ERROR_DATA_H */
