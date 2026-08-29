// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What one run-time rewrite call costs, over the shared ECMA-262 table.
//
// The C++ half of the per-call price in docs/SCE_LUA_TRANSLATION_SEAM.md.
// Its Rust counterpart is sce-build/examples/lowering_per_call.rs, and the
// two are meant to be read from one run of
// scripts/measure-lowering-per-call.sh, which puts them on ONE host inside
// ONE load window. A cross-machine comparison is not a comparison: the
// first attempt at this measurement timed C++ locally and Rust on the
// build machine and had to be thrown away.
//
// WHY THERE ARE TWO FIXTURES, AND WHY THE COLD ONE IS THE COST.
//
// EcmaScriptToLuaTransformer keeps three mutable memo caches inside itself
// — generalCache_, guardCache_, scriptCache_ (EcmaScriptToLuaTransformer.h)
// — so a probe that reuses one instance measures a hash lookup and reports
// a two-digit figure. That is a FLOOR, not a cost, in exactly the way an
// unexported cdylib's 380 KB was a floor and not a size. Constructing the
// transformer inside the timed region is what makes every call a miss, and
// a miss is what a caller pays the first time it sees an expression.
//
// Both are reported on purpose. The gap between them IS the memo, and the
// document's comparison against the Rust frontend turns on that: the
// frontend has no cache, so it must be compared against the cold column.
// Reporting only the warm number is the mistake this file exists to make
// impossible to repeat — it has been walked into three times on this axis.
//
// NOTHING HERE ASSERTS A BOUND. This machine is shared between sessions
// and the same 21 gates have been measured at 529s and 1161s, so a timing
// assertion would be a flake generator. What the file buys by existing is
// that the figure in the document has a command behind it.

#include "scripting/EcmaScriptToLuaTransformer.h"

#include <benchmark/benchmark.h>
#include <fstream>
#include <nlohmann/json.hpp>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

using Context = SCE::EcmaScriptToLuaTransformer::ExpressionContext;

struct Case {
    std::string source;
    Context context;
};

// The shared table, read once. This is the same file the LoweredEcma262
// ratchet and the Rust half read, so the two halves of the measurement
// are over the same population by construction rather than by a comment
// claiming they are.
const std::vector<Case> &cases() {
    static const std::vector<Case> loaded = [] {
        std::ifstream in(SCE_ECMA262_CASES_PATH);
        if (!in) {
            throw std::runtime_error("cannot open the shared ECMA-262 table: " SCE_ECMA262_CASES_PATH);
        }
        nlohmann::json table;
        in >> table;

        std::vector<Case> out;
        for (const auto &entry : table.at("cases")) {
            // `condition` is what a transition guard carries, and it is the
            // context that adds truthiness wrapping — a different cache and
            // a different amount of work, so the split has to be kept.
            const bool guard = entry.value("form", std::string{}) == "condition";
            out.push_back(Case{entry.at("source").get<std::string>(), guard ? Context::Guard : Context::General});
        }

        // A benchmark over an empty set reports a very good number. The
        // table held 98 cases when this bound was written.
        if (out.size() < 50) {
            throw std::runtime_error("only " + std::to_string(out.size()) +
                                     " case(s) read from the shared table — the corpus walk is broken");
        }
        return out;
    }();
    return loaded;
}

// COLD: a fresh transformer per call, so every call misses the memo.
// This is the number a caller pays for an expression it has not seen.
void BM_RewriteCold(benchmark::State &state) {
    const auto &table = cases();
    for (auto _ : state) {
        for (const auto &c : table) {
            SCE::EcmaScriptToLuaTransformer transformer;
            benchmark::DoNotOptimize(transformer.transform(c.source, c.context));
        }
    }
    state.SetItemsProcessed(static_cast<int64_t>(state.iterations()) * static_cast<int64_t>(table.size()));
}

// WARM: one transformer for the whole run, so from the second pass on
// every call is a memo hit. Reported so the gap can be attributed to the
// cache rather than to the algorithm.
void BM_RewriteWarm(benchmark::State &state) {
    const auto &table = cases();
    SCE::EcmaScriptToLuaTransformer transformer;
    // Prime it outside the timed region: a warm figure that includes the
    // first, cold pass is neither of the two numbers this file reports.
    for (const auto &c : table) {
        benchmark::DoNotOptimize(transformer.transform(c.source, c.context));
    }
    for (auto _ : state) {
        for (const auto &c : table) {
            benchmark::DoNotOptimize(transformer.transform(c.source, c.context));
        }
    }
    state.SetItemsProcessed(static_cast<int64_t>(state.iterations()) * static_cast<int64_t>(table.size()));
}

BENCHMARK(BM_RewriteCold);
BENCHMARK(BM_RewriteWarm);

}  // namespace
