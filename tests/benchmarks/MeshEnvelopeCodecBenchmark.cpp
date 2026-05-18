// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Drain hot-path allocation cost benchmark — closes the measurement
// half of mesh_open_issues.md Issue 1.
//
// Issue 1's defendable claim: every wire envelope decoded by
// `ShmChannel::drainWith` populates `MeshEnvelope.type: std::string`
// from CBOR text bytes via tinycbor's `cbor_value_copy_text_string`
// API, which performs a heap allocation when the value exceeds
// glibc's std::string SSO threshold (15 bytes on x86_64 libstdc++).
// Event names shorter than the threshold land in the inline buffer —
// zero allocations per event. Longer names trigger one alloc per
// event, plus a paired free at envelope destruction.
//
// Three measurement points distinguish where cost actually lives:
//
//   A. `BM_DecodeShortName` — decode envelope whose `type` field is
//      a 10-character name ("user.click"). Total path: tinycbor
//      walk + SSO-fits string copy + the other CBOR field walks.
//      No `type`-field heap alloc.
//
//   B. `BM_DecodeLongName` — decode envelope whose `type` field is
//      a 30-character name ("application.module.event.subscription.ack").
//      Same path as (A) plus one heap alloc + matching free for `type`.
//      The (B - A) delta is the cost the inline_string<N> migration
//      would save per event.
//
//   C. `BM_RoundtripShort` / `BM_RoundtripLong` — encode then decode.
//      Captures the full producer-consumer cycle so the absolute
//      drain-budget percentage stays anchored against realistic
//      end-to-end cost rather than an isolated decode that ignores
//      encode-side allocations.
//
// Threshold gating decision recorded in the closing entry of
// mesh_open_issues.md (Issue 1 — drain hot-path allocation):
//   (B - A) < 100 ns        → close + document
//   (B - A) 100..500 ns     → document only, no fix
//   (B - A) > 500 ns or > 5% of drain budget → migrate to inline buffer
//
// The "drain budget" reference point is the wire-21 SM dispatch
// chain measured by `Wire21SenderBenchmark.cpp` — typical sender
// hits ~150 ns/op there, so a per-event 500 ns alloc cost on the
// receive side would dominate a steady wire-21 stream.

#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/PatternKind.h"
#include "mesh/PayloadCodec.h"

#include <benchmark/benchmark.h>

#include <array>
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

namespace {

// Short name: 10 chars — fits glibc libstdc++ SSO threshold (15 bytes).
// Decoder writes into the inline buffer; no heap activity for `type`.
constexpr const char *kShortName = "user.click";

// Long name: 44 chars — exceeds SSO. Decoder allocates a heap buffer
// for `type` per envelope, freed at envelope destruction.
constexpr const char *kLongName =
    "application.module.event.subscription.ack";

::SCE::Mesh::MeshEnvelope makeEnvelope(const char *type_name) {
    ::SCE::Mesh::MeshEnvelope env;
    env.id = {{0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
               0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10}};
    env.source = "bench_sender";
    env.type = type_name;
    env.pattern = ::SCE::Mesh::PatternKind::FireForget;
    env.datacontenttype = ::SCE::Mesh::PayloadCodec::None;
    return env;
}

}  // namespace

// ─── A. Decode-only, short `type` (SSO, no alloc on `type`) ────────────
static void BM_DecodeShortName(benchmark::State &state) {
    const auto env = makeEnvelope(kShortName);
    const auto wire = ::SCE::Mesh::encodeEnvelope(env);
    for (auto _ : state) {
        ::SCE::Mesh::MeshEnvelope out;
        const bool ok =
            ::SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), out);
        bool ok_local = ok;
        const auto *out_ptr = &out;
        benchmark::DoNotOptimize(ok_local);
        benchmark::DoNotOptimize(out_ptr);
    }
}
BENCHMARK(BM_DecodeShortName);

// ─── B. Decode-only, long `type` (heap alloc on `type`) ───────────────
static void BM_DecodeLongName(benchmark::State &state) {
    const auto env = makeEnvelope(kLongName);
    const auto wire = ::SCE::Mesh::encodeEnvelope(env);
    for (auto _ : state) {
        ::SCE::Mesh::MeshEnvelope out;
        const bool ok =
            ::SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), out);
        bool ok_local = ok;
        const auto *out_ptr = &out;
        benchmark::DoNotOptimize(ok_local);
        benchmark::DoNotOptimize(out_ptr);
    }
}
BENCHMARK(BM_DecodeLongName);

// ─── C. Roundtrip — encode + decode, short `type` ────────────────────
static void BM_RoundtripShort(benchmark::State &state) {
    const auto env = makeEnvelope(kShortName);
    for (auto _ : state) {
        const auto wire = ::SCE::Mesh::encodeEnvelope(env);
        ::SCE::Mesh::MeshEnvelope out;
        const bool ok =
            ::SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), out);
        bool ok_local = ok;
        const auto *out_ptr = &out;
        benchmark::DoNotOptimize(ok_local);
        benchmark::DoNotOptimize(out_ptr);
    }
}
BENCHMARK(BM_RoundtripShort);

// ─── D. Roundtrip — encode + decode, long `type` ─────────────────────
static void BM_RoundtripLong(benchmark::State &state) {
    const auto env = makeEnvelope(kLongName);
    for (auto _ : state) {
        const auto wire = ::SCE::Mesh::encodeEnvelope(env);
        ::SCE::Mesh::MeshEnvelope out;
        const bool ok =
            ::SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), out);
        bool ok_local = ok;
        const auto *out_ptr = &out;
        benchmark::DoNotOptimize(ok_local);
        benchmark::DoNotOptimize(out_ptr);
    }
}
BENCHMARK(BM_RoundtripLong);

BENCHMARK_MAIN();
