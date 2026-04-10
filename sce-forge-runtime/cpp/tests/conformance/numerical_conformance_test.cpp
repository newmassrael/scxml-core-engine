// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-language numerical conformance harness (C++ half).
//
// The fixture headers included below are produced at CMake configure time by
// the generate_forge_fixtures custom target, which invokes sce-codegen on
// the SCXML files under tests/forge/resources/ and writes the C++ output
// into ${CMAKE_CURRENT_BINARY_DIR}/generated. That directory is added to
// this translation unit's include path by CMakeLists.txt. No committed C++
// goldens are consumed — the single source of truth is the SCXML and the
// codegen.
//
// The generated fixtures are exercised against the reference vectors in
// tests/forge/conformance/numerical_reference.json (path provided via the
// REFERENCE_JSON_PATH compile-time define), the same file used by the Rust,
// Python, Kotlin, and Go conformance tests.

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include "filter_debounce.h"
#include "filter_moving_average.h"
#include "interpolation_1d_linear.h"
#include "interpolation_2d_bilinear.h"
#include "observer_coolant.h"

using json = nlohmann::json;

namespace {

int g_failures = 0;

#define CHECK(cond, ...)                               \
    do {                                               \
        if (!(cond)) {                                 \
            std::printf("FAIL: ");                     \
            std::printf(__VA_ARGS__);                  \
            std::printf("\n");                         \
            ++g_failures;                              \
        }                                              \
    } while (0)

void assert_close(double actual, double expected, double tol, const char *label) {
    double diff = std::fabs(actual - expected);
    CHECK(diff <= tol,
          "%s: actual=%.17g expected=%.17g diff=%.17g tol=%.17g",
          label,
          actual,
          expected,
          diff,
          tol);
}

json load_reference() {
    std::ifstream in(REFERENCE_JSON_PATH);
    if (!in.is_open()) {
        std::fprintf(stderr,
                     "cannot open reference JSON at %s\n",
                     REFERENCE_JSON_PATH);
        std::exit(2);
    }
    json ref;
    in >> ref;
    return ref;
}

void test_interpolation_1d_linear(const json &ref, double tol) {
    const auto &spec = ref["pure_functions"]["interpolation_1d_linear"];
    for (const auto &c : spec["cases"]) {
        auto rpm = c["args"][0].get<std::uint16_t>();
        auto expected = c["expected"].get<double>();
        double actual =
            SCE::Generated::Interpolation1dLinear::Interpolation1dLinear::lookup(rpm);
        char label[128];
        std::snprintf(label, sizeof(label), "interpolation_1d_linear(%u)",
                      static_cast<unsigned>(rpm));
        assert_close(actual, expected, tol, label);
    }
}

void test_interpolation_2d_bilinear(const json &ref, double tol) {
    const auto &spec = ref["pure_functions"]["interpolation_2d_bilinear"];
    for (const auto &c : spec["cases"]) {
        auto rpm = c["args"][0].get<std::uint16_t>();
        auto load = c["args"][1].get<std::uint8_t>();
        auto expected = c["expected"].get<double>();
        double actual =
            SCE::Generated::Interpolation2dBilinear::Interpolation2dBilinear::lookup(
                rpm, load);
        char label[128];
        std::snprintf(label, sizeof(label),
                      "interpolation_2d_bilinear(%u, %u)",
                      static_cast<unsigned>(rpm),
                      static_cast<unsigned>(load));
        assert_close(actual, expected, tol, label);
    }
}

void test_filter_moving_average(const json &ref, double tol) {
    const auto &spec = ref["stateful_filters"]["filter_moving_average"];
    SCE::Generated::FilterMovingAverage::FilterMovingAverage filter{};
    int i = 0;
    for (const auto &step : spec["sequence"]) {
        auto input = step["input"].get<double>();
        auto expected = step["expected"].get<double>();
        double actual = filter.update(input);
        char label[128];
        std::snprintf(label, sizeof(label),
                      "filter_moving_average step %d input=%.17g",
                      i, input);
        assert_close(actual, expected, tol, label);
        ++i;
    }
}

void test_filter_debounce(const json &ref) {
    const auto &spec = ref["stateful_filters"]["filter_debounce"];
    SCE::Generated::FilterDebounce::FilterDebounce filter{};
    int i = 0;
    for (const auto &step : spec["sequence"]) {
        auto input = step["input"].get<bool>();
        auto expected = step["expected"].get<bool>();
        bool actual = filter.update(input);
        CHECK(actual == expected,
              "filter_debounce step %d input=%d: actual=%d expected=%d",
              i,
              static_cast<int>(input),
              static_cast<int>(actual),
              static_cast<int>(expected));
        ++i;
    }
}

const char *coolant_tag_name(
    SCE::Generated::ObserverCoolant::ForgeDomain::Tag tag) {
    switch (tag) {
    case SCE::Generated::ObserverCoolant::ForgeDomain::EMIT_WARNING:
        return "EMIT_WARNING";
    case SCE::Generated::ObserverCoolant::ForgeDomain::CLEAR_WARNING:
        return "CLEAR_WARNING";
    case SCE::Generated::ObserverCoolant::ForgeDomain::EMERGENCY_SHUTDOWN:
        return "EMERGENCY_SHUTDOWN";
    }
    return "unknown";
}

void test_observer_coolant(const json &ref) {
    const auto &spec = ref["observers"]["observer_coolant"];
    SCE::Generated::ObserverCoolant::ObserverCoolant observer{};
    int i = 0;
    for (const auto &step : spec["sequence"]) {
        auto input = step["input"].get<double>();
        std::vector<std::string> expected_events;
        for (const auto &e : step["expected_events"]) {
            expected_events.push_back(e.get<std::string>());
        }
        auto queue = observer.update(input);
        std::vector<std::string> actual_events;
        for (std::size_t j = 0; j < queue.size(); ++j) {
            actual_events.emplace_back(coolant_tag_name(queue[j].tag));
        }
        if (actual_events != expected_events) {
            std::printf(
                "FAIL: observer_coolant step %d input=%.17g: got [",
                i, input);
            for (std::size_t j = 0; j < actual_events.size(); ++j) {
                if (j) std::printf(", ");
                std::printf("%s", actual_events[j].c_str());
            }
            std::printf("] expected [");
            for (std::size_t j = 0; j < expected_events.size(); ++j) {
                if (j) std::printf(", ");
                std::printf("%s", expected_events[j].c_str());
            }
            std::printf("]\n");
            ++g_failures;
        }
        ++i;
    }
}

} // namespace

int main() {
    json ref = load_reference();
    double tol = ref["float_tolerance"].get<double>();

    test_interpolation_1d_linear(ref, tol);
    test_interpolation_2d_bilinear(ref, tol);
    test_filter_moving_average(ref, tol);
    test_filter_debounce(ref);
    test_observer_coolant(ref);

    if (g_failures > 0) {
        std::printf("FAILED: %d assertion(s)\n", g_failures);
        return 1;
    }
    std::printf("OK: all conformance tests passed\n");
    return 0;
}
