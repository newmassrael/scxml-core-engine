// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/span.h — the view a generated C++ function takes a list or a byte
// string through (docs/adr/0059, section 4). What it is made from is decided by
// the compiler, so those properties are `static_assert`s: a contiguous
// container, a temporary for a `const` view, a built-in array and a view of
// fewer qualifiers convert; a `const` container to a writable view, a container
// of a derived type and one of another element type do not. What it reads is
// checked at run time. This target is built at C++17, the standard the view is
// for; the generated functions that take one are the conformance harness's.

#include "sce/forge/span.h"

#include <array>
#include <cstdint>
#include <cstdio>
#include <string_view>
#include <type_traits>
#include <vector>

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

using ::SCE::Forge::Span;

struct Base {
    int x;
};

struct Derived : Base {
    int y;
};

static_assert(std::is_convertible_v<std::vector<int> &, Span<int>>, "a container converts to a writable view");
static_assert(std::is_convertible_v<const std::vector<int> &, Span<const int>>,
              "a const container converts to a const view");
static_assert(std::is_convertible_v<std::vector<int>, Span<const int>>, "a temporary converts to a const view");
static_assert(!std::is_convertible_v<const std::vector<int> &, Span<int>>, "a const container is not a writable view");
static_assert(!std::is_convertible_v<std::vector<int>, Span<int>>, "a temporary is not a writable view");
static_assert(std::is_convertible_v<std::array<std::uint8_t, 4> &, Span<const std::uint8_t>>, "a std::array converts");
static_assert(std::is_convertible_v<std::string_view, Span<const char>>, "a std::string_view converts");
static_assert(std::is_convertible_v<int (&)[3], Span<const int>>, "a built-in array converts");
static_assert(std::is_convertible_v<Span<int>, Span<const int>>, "a writable view is a const view");
static_assert(!std::is_convertible_v<Span<const int>, Span<int>>, "a const view is not a writable view");
static_assert(!std::is_convertible_v<std::vector<Derived> &, Span<Base>>,
              "a derived element is not read at its base's stride");
static_assert(!std::is_convertible_v<std::vector<long> &, Span<const int>>, "another element type does not convert");
static_assert(!std::is_convertible_v<int, Span<const int>>, "a value is not a view");
static_assert(std::is_trivially_copyable_v<Span<const int>>, "a view copies as its pointer and count");

constexpr int TABLE[] = {1, 2, 3};
constexpr Span<const int> TABLE_VIEW{TABLE};
static_assert(TABLE_VIEW.size() == 3 && TABLE_VIEW[2] == 3 && TABLE_VIEW.back() == 3, "a view is usable as a constant");

int sum(Span<const int> values) {
    int total = 0;
    for (const int v : values) {
        total += v;
    }
    return total;
}

void an_empty_view_views_nothing() {
    const Span<const int> none;
    CHECK(none.size() == 0, "an empty view has no element");
    CHECK(none.empty(), "an empty view is empty");
    CHECK(none.data() == nullptr, "an empty view points nowhere");
    CHECK(none.begin() == none.end(), "an empty view iterates nothing");
    CHECK(sum(Span<const int>{}) == 0, "an empty view sums to 0");
}

void a_view_reads_the_elements_it_was_made_from() {
    const std::vector<int> held = {4, 5, 6, 7};
    const Span<const int> view = held;
    CHECK(view.size() == 4, "the view counts the container's elements");
    CHECK(!view.empty(), "a view of elements is not empty");
    CHECK(view.data() == held.data(), "the view points at the container's elements, not a copy");
    CHECK(view[0] == 4 && view[3] == 7, "indexing reads the container's elements");
    CHECK(view.front() == 4 && view.back() == 7, "front and back are the first and last elements");
    CHECK(sum(held) == 22, "a container passed to a view parameter is read whole");

    const Span<const int> part(held.data() + 1, 2);
    CHECK(sum(part) == 11, "a pointer and a count view exactly those elements");

    int array[] = {1, 2, 3};
    CHECK(sum(array) == 6, "a built-in array is read whole");

    const std::array<int, 2> fixed = {10, 20};
    CHECK(sum(fixed) == 30, "a std::array is read whole");

    const std::string_view text = "abc";
    const Span<const char> letters = text;
    CHECK(letters.size() == 3 && letters[1] == 'b', "a std::string_view is viewed as its characters");
}

void a_temporary_lasts_the_call_it_is_passed_to() {
    CHECK(sum(std::vector<int>{1, 2, 3}) == 6, "a temporary container is read whole by the call");
}

void a_writable_view_writes_through() {
    std::vector<int> held = {1, 2, 3};
    const Span<int> view = held;
    view[0] = 9;
    CHECK(held[0] == 9, "a write through a view reaches the container");
    const Span<const int> reading = view;
    CHECK(reading.data() == held.data() && reading.size() == 3, "a writable view converts to the same const view");
}

}  // namespace

int main() {
    an_empty_view_views_nothing();
    a_view_reads_the_elements_it_was_made_from();
    a_temporary_lasts_the_call_it_is_passed_to();
    a_writable_view_writes_through();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("span_runtime_test: all checks passed");
    return 0;
}
