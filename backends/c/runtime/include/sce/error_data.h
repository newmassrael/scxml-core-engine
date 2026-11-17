// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// storage of the engine's own, through the writer every saved state is written
// with, so its escapes are the ones every engine writes and the text is the one
// the C++ engines' `SCE::Common::errorData` builds (sce/include/common/ErrorData.h).
//
// Header-only, C11, and consumable from C++ (the gate in
// `cxx_consumption_gate.cmake`).

#ifndef SCE_ERROR_DATA_H
#define SCE_ERROR_DATA_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce/generated/error_reason.h"
#include "sce/saved_json.h"

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

/* Write the `_event.data` of an `error.*` event raised for `reason` into `w`, as
   JSON text: `{"errorName":...,"reason":...,"detail":...}`, `detail` (NULL for
   none) escaped as every engine escapes a JSON string. A writer over no buffer
   measures the text, so a caller sizes its storage with one call and writes
   with a second. */
static inline void sce_error_data_write(sce_jw_t *w, ErrorReason_t reason, const char *detail) {
    const char *name = error_reason_declared_name(reason);
    sce_jw_begin_object(w);
    sce_jw_key(w, "errorName");
    sce_jw_string(w, sce_error_name(reason));
    sce_jw_key(w, "reason");
    /* A value no member names is not one the vocabulary declares; the enum can
       hold it only through a cast, and the record then names no code. */
    sce_jw_string(w, name != NULL ? name : "");
    sce_jw_key(w, "detail");
    sce_jw_string(w, detail != NULL ? detail : "");
    sce_jw_end_object(w);
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_ERROR_DATA_H */
