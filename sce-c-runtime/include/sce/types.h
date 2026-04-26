// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE C11 runtime — cross-fixture defaults.
//
// RFC §5.J.1 (watching-zenoh downstream MCU backend). The C++ AOT
// runtime in `sce/include/static/StaticExecutionEngine.h` is a class
// template — the compiler instantiates it once per generated state
// machine, so every concrete data layout (event queue, event
// metadata, transition tables) lives inside the generated translation
// unit. The C11 backend mirrors that pattern: jinja2 macros under
// `tools/codegen/templates/c/helpers/*.jinja2` inline equivalent
// algorithm bodies into each generated `_sm.c`, and per-fixture data
// types are declared inside the same translation unit.
//
// As a result this header carries only what is genuinely cross-fixture
// — capacity defaults that a generated machine references via the
// preprocessor. Each can be overridden at build time per fixture
// (`-DSCE_MAX_EVENTS=N`, etc.).

#ifndef SCE_TYPES_H
#define SCE_TYPES_H

#ifndef SCE_MAX_EVENTS
#define SCE_MAX_EVENTS 32
#endif

#ifndef SCE_MAX_ID_LEN
#define SCE_MAX_ID_LEN 64
#endif

#ifndef SCE_MAX_INVOKES
#define SCE_MAX_INVOKES 4
#endif

#ifndef SCE_MAX_PARALLEL_REGIONS
#define SCE_MAX_PARALLEL_REGIONS 4
#endif

// W3C SCXML 5.10 _event.data — generated event records reserve a fixed
// payload slot so the entire EventWithMetadata is value-typed and queue
// storage is statically reservable. The default sizes datamodel-less and
// short JSON payloads; fixtures that carry longer SCXML <send> data
// override per build.
#ifndef SCE_MAX_DATA_LEN
#define SCE_MAX_DATA_LEN 256
#endif

// W3C SCXML 5.10 _event.type — bounded by the spec's enumerated values
// ("internal", "external", "platform"). 16 bytes leaves headroom for any
// future additions without requiring a per-fixture override.
#ifndef SCE_MAX_EVENT_TYPE_LEN
#define SCE_MAX_EVENT_TYPE_LEN 16
#endif

// Static helpers (state hierarchy queries, history filters, …) are
// emitted unconditionally so generated code stays compilable as fixtures
// climb the W3C category ladder. The flat / datamodel-less subset used
// by the simplest fixtures does not call every helper, so wrap each one
// with this attribute to keep `-Wall -Wunused-function` clean. Resolves
// to nothing on toolchains that lack the GNU attribute, where the
// warning is unlikely to be treated as an error anyway.
#if defined(__GNUC__) || defined(__clang__)
#define SCE_C_UNUSED __attribute__((unused))
#else
#define SCE_C_UNUSED
#endif

#endif  // SCE_TYPES_H
