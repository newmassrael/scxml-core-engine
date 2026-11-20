// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE C11 runtime: the JSON text of the `_event.data` of an `error.*` event a
// machine raises for itself (docs/adr/0106, #111), the record SCE_MESH.md
// section 10.7.1 pins: `errorName`, `reason` and `detail`.
//
// A machine's core carries the code beside the sentence (`sce/error_data.h`)
// and never writes the record as text (docs/adr/0110, D1): the text is written
// where `_event.data` is handed to a data model that reads JSON, which is a
// script engine's binding, through the writer every saved state is written
// with, so its escapes are the ones every engine writes and the text is the one
// the C++ engines' `SCE::Common::errorData` builds
// (sce/include/common/ErrorData.h). That writer is hosted, and so is this
// header: a machine includes it only where it has a script engine.
//
// Header-only, C11, and consumable from C++.

#ifndef SCE_ERROR_DATA_TEXT_H
#define SCE_ERROR_DATA_TEXT_H

#include "sce/error_data.h"
#include "sce/saved_json.h"

#ifdef __cplusplus
extern "C" {
#endif

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

#endif /* SCE_ERROR_DATA_TEXT_H */
