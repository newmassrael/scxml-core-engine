/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */

/*
 * SCE Forge — runtime types for Level-2 procedure state machines (C11 backend).
 *
 * Generated procedure code uses these typedefs as the public API for
 * service handlers, helper closures, and run results. Mirrors the
 * cpp/Rust/Kotlin/Go/Python procedure runtimes so a 6-backend cross-
 * language conformance suite can compare byte-equal dispatched payloads
 * for any given fixture.
 *
 * Memory policy (RFC §5.J.2 F1, see `sce-forge-runtime/c/CMakeLists.txt`):
 *   - No heap. The bytes container is a stack-bounded fixed-size struct
 *     with capacity `SCE_FORGE_BYTES_DEFAULT_MAX` (= 256 today). All
 *     instances copy by value; helpers and handlers receive const
 *     pointers and write through out-parameters. The cap is the
 *     contract; per-slot caps from `sce:max-size` annotations apply at
 *     runtime as length checks via the `error.execution` raise path
 *     (RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B4).
 *   - No threads, no I/O, no globals. All state lives in a state struct
 *     passed explicitly through `<snake>_execute(handler, helpers..., args)`.
 */

#ifndef SCE_FORGE_PROCEDURE_H
#define SCE_FORGE_PROCEDURE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce/forge/limits.h"

/* Bytes container — fixed-cap stack-only buffer. */
typedef struct {
    uint8_t data[SCE_FORGE_BYTES_DEFAULT_MAX];
    size_t len;
} sce_forge_bytes_t;

/* Service request dispatched by `<send sce:service=...>`. */
typedef struct {
    const char *service;            /* sce:service — always present */
    bool has_subfunc;
    const char *subfunc;            /* sce:subfunc — valid when has_subfunc */
    bool has_addr;
    const char *addr;               /* sce:addr resolved to string */
    bool has_payload;
    sce_forge_bytes_t payload;      /* sce:payload bytes — valid when has_payload */
} sce_forge_procedure_service_request_t;

/* Service response. `data` populates `_event.data` for subsequent
 * `<assign>` actions; `success` selects the `ok` vs `fail` event. */
typedef struct {
    bool success;
    sce_forge_bytes_t data;
} sce_forge_procedure_service_response_t;

/* Service handler signature. The handler receives the request and an
 * opaque `user_data` slot for the caller's context (e.g. transport
 * client). Must be set via `<snake>_execute()` before the procedure
 * runs; a NULL handler is a programmer error and the generated code
 * falls through to an uncompleted result. */
typedef sce_forge_procedure_service_response_t (*sce_forge_procedure_service_handler_t)(
    const sce_forge_procedure_service_request_t *req,
    void *user_data);

/* Done data parameter (one row of `<donedata><param>...`). The
 * generated code emits one static const array per `<final>` state
 * with done data; `<snake>_execute()` returns a pointer + count
 * pair. */
typedef struct {
    const char *name;
    const char *value;
} sce_forge_procedure_done_param_t;

/* Run result, mirroring cpp ProcedureRunResult. `done_data` points at a
 * static const array owned by the generated code; the caller must not
 * free it. */
typedef struct {
    bool completed;
    const char *final_state;        /* "" when not completed */
    const sce_forge_procedure_done_param_t *done_data;
    size_t done_data_count;
} sce_forge_procedure_run_result_t;

/* Internal-event raise outcome from execute_transition_actions.
 * `raised = false` ⇒ normal flow; `raised = true` ⇒ run loop pumps
 * `event` back through process_transition for the source state. */
typedef struct {
    bool raised;
    int event;
} sce_forge_procedure_raise_t;

#endif  /* SCE_FORGE_PROCEDURE_H */
