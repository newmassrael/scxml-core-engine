// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/bounded.h — a list held to a lower and an upper bound (docs/adr/0065
// §6 R1, docs/adr/0077 §3), which a generated C++ value presents a byte string
// through (docs/adr/0081, step 4). What converts to what is decided by the
// compiler, so those properties are `static_assert`s: a list within narrower
// bounds converts, to a view without a copy and to an owning list by an
// explicit copy, and one within looser bounds does not. What `make` admits and
// what a held list reads are checked at run time. This target is built at
// C++17, the standard it is for.

#include "sce/forge/bounded.h"

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <string>
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

using ::SCE::Forge::Bounded;
using ::SCE::Forge::Bytes;
using ::SCE::Forge::Text;

using Owned = Bounded<std::vector<std::uint8_t>, 8>;
using OwnedAtLeastTwo = Bounded<std::vector<std::uint8_t>, 8, 2>;

// A view within narrower bounds converts implicitly; one within looser bounds
// does not convert at all.
static_assert(std::is_convertible<Bytes<4>, Bytes<8>>::value, "a tighter upper bound converts");
static_assert(std::is_convertible<Bytes<8, 2>, Bytes<8>>::value, "a tighter lower bound converts");
static_assert(!std::is_constructible<Bytes<8>, Bytes<16>>::value, "a looser upper bound does not");
static_assert(!std::is_constructible<Bytes<8, 2>, Bytes<8>>::value, "a looser lower bound does not");

// An owning list is viewed without a copy; a view is held by an explicit copy.
static_assert(std::is_convertible<Owned, Bytes<8>>::value, "an owned list is viewed implicitly");
static_assert(std::is_nothrow_constructible<Bytes<8>, const Owned &>::value, "viewing an owned list throws nothing");
static_assert(!std::is_convertible<Bytes<8>, Owned>::value, "a view is not copied implicitly");
static_assert(std::is_constructible<Owned, Bytes<8>>::value, "a view is copied explicitly");

// A list holds no element past its bound: a bare list is no `Bounded`, and the
// only door is `make`.
static_assert(!std::is_constructible<Owned, std::vector<std::uint8_t>>::value, "a bare list is checked by make");
static_assert(!std::is_constructible<Bytes<8>, std::vector<std::uint8_t>>::value, "a bare list is checked by make");

// A view holds no list of its own, so where it must hold some it has no default.
static_assert(std::is_default_constructible<Bytes<8>>::value, "an empty view lies within a bound of no fewest");
static_assert(!std::is_default_constructible<Bytes<8, 2>>::value, "a view has no fewest elements to hold");
static_assert(std::is_default_constructible<OwnedAtLeastTwo>::value, "an owning list holds the fewest itself");

void make_admits_exactly_the_lists_within_the_bounds() {
    const std::vector<std::uint8_t> none{};
    const std::vector<std::uint8_t> one{1};
    const std::vector<std::uint8_t> eight{1, 2, 3, 4, 5, 6, 7, 8};
    const std::vector<std::uint8_t> nine{1, 2, 3, 4, 5, 6, 7, 8, 9};
    CHECK(Bytes<8>::make(none).has_value(), "no element lies within a bound of no fewest");
    CHECK(Bytes<8>::make(eight).has_value(), "the most lies within the bound");
    CHECK(!Bytes<8>::make(nine).has_value(), "one past the most does not");
    CHECK((!Bytes<8, 2>::make(one).has_value()), "one short of the fewest does not");
    CHECK((Bytes<8, 2>::make(eight).has_value()), "a list between the fewest and the most does");
    CHECK(!Owned::make(nine).has_value(), "an owning list is held to the same bound");
}

void a_held_list_reads_as_the_list_it_holds() {
    const std::vector<std::uint8_t> bytes{7, 8, 9};
    const std::optional<Bytes<8>> view = Bytes<8>::make(bytes);
    CHECK(view.has_value(), "three bytes lie within eight");
    if (!view) {
        return;
    }
    CHECK(view->size() == 3 && !view->empty() && view->data() == bytes.data(), "a view views the list");
    CHECK((*view)[0] == 7 && (*view)[2] == 9, "it reads the elements by index");
    std::size_t sum = 0;
    for (const std::uint8_t b : *view) {
        sum += b;
    }
    CHECK(sum == 24, "it iterates the elements in order");
    CHECK(*view == bytes && bytes == *view && !(*view != bytes), "it compares with the list it holds");

    const Owned owned(*view);
    CHECK(owned == *view && owned.inner() == bytes, "an explicit copy holds the same elements");
    CHECK(owned.data() != bytes.data(), "and holds them in a list of its own");
    const Bytes<8> again = owned;
    CHECK(again.data() == owned.data(), "a view of an owned list views it without a copy");

    const std::vector<std::uint8_t> other{7, 8, 10};
    CHECK((*view != other && *view != std::vector<std::uint8_t>{7, 8}), "a different list differs");
}

// A text is a list of `char` and converts as a byte string does: an owned
// text is viewed without a copy, and a view is held by an explicit copy.
using OwnedText = Bounded<std::string, 8>;
static_assert(std::is_convertible<OwnedText, Text<8>>::value, "an owned text is viewed implicitly");
static_assert(!std::is_convertible<Text<8>, OwnedText>::value, "a text view is not copied implicitly");
static_assert(std::is_constructible<OwnedText, Text<8>>::value, "a text view is copied explicitly");
static_assert(!std::is_constructible<Text<8>, std::string_view>::value, "a bare text is checked by make");

void make_admits_a_text_only_of_well_formed_utf8() {
    CHECK(Text<8>::make(std::string_view("abc")).has_value(), "ASCII is UTF-8");
    CHECK(Text<8>::make(std::string_view("\xC3\xA9t\xC3\xA9")).has_value(), "two-byte sequences are UTF-8");
    CHECK(!Text<8>::make(std::string_view("\xC3")).has_value(), "a sequence cut short is not");
    CHECK(!Text<8>::make(std::string_view("\xFF")).has_value(), "a byte no sequence begins with is not");
    CHECK(!Text<8>::make(std::string_view("\xED\xA0\x80")).has_value(), "a surrogate is not");
    CHECK(!Text<2>::make(std::string_view("abc")).has_value(), "a text past its bound is not made either");
    CHECK(Text<8>::make(std::string_view()).has_value(), "an empty text is UTF-8");

    const std::string held("caf\xC3\xA9");
    const auto view = Text<8>::make(held);
    CHECK(view.has_value() && *view == held && view->size() == 5, "a text compares with the string it views");
    if (!view) {
        return;
    }
    const OwnedText owned(*view);
    CHECK(owned == held && owned.inner() == held && owned.data() != held.data(),
          "an explicit copy holds the same text in a string of its own");
}

void a_default_holds_the_fewest_elements_the_bounds_admit() {
    const Bytes<8> empty{};
    CHECK(empty.empty() && empty.size() == 0, "a view of no fewest is empty");
    const OwnedAtLeastTwo fewest{};
    CHECK(fewest.size() == 2 && fewest[0] == 0 && fewest[1] == 0, "an owning list holds the fewest, value-made");
}

// Two lists of one count convert as each list does, which keeps each count:
// owning lists are viewed without a copy, and views are held by an explicit
// copy. Two bare lists are no `Aligned`; the only door is `make`.
using ::SCE::Forge::Aligned;
using OwnedPair = Aligned<Owned, Owned>;
using ViewedPair = Aligned<Bytes<8>, Bytes<8>>;
static_assert(std::is_convertible<OwnedPair, ViewedPair>::value, "two owned lists are viewed implicitly");
static_assert(!std::is_convertible<ViewedPair, OwnedPair>::value, "two views are not copied implicitly");
static_assert(std::is_constructible<OwnedPair, ViewedPair>::value, "two views are copied explicitly");
static_assert(!std::is_constructible<Aligned<Bytes<4>, Bytes<4>>, ViewedPair>::value,
              "two lists within looser bounds do not convert");
static_assert(!std::is_constructible<ViewedPair, Bytes<8>, Bytes<8>>::value, "two bare lists are checked by make");

void make_admits_exactly_two_lists_of_one_count() {
    const std::vector<std::uint8_t> three{1, 2, 3};
    const std::vector<std::uint8_t> also_three{4, 5, 6};
    const std::vector<std::uint8_t> two{4, 5};
    const auto first = Bytes<8>::make(three);
    const auto second = Bytes<8>::make(also_three);
    const auto shorter = Bytes<8>::make(two);
    CHECK(first && second && shorter, "each list lies within its bound");
    if (!first || !second || !shorter) {
        return;
    }
    CHECK(!ViewedPair::make(*first, *shorter).has_value(), "two lists of different counts are not one count");
    const auto pair = ViewedPair::make(*first, *second);
    CHECK(pair.has_value(), "two lists of one count are");
    if (!pair) {
        return;
    }
    CHECK(pair->size() == 3 && !pair->empty(), "it holds the count of each");
    CHECK(pair->first() == three && pair->second() == also_three, "it reads each list as it was made");

    const OwnedPair owned(*pair);
    CHECK(owned == *pair && owned.first().data() != three.data(), "an explicit copy holds the same lists");
    const ViewedPair again = owned;
    CHECK(again == owned && again.first().data() == owned.first().data(), "a view of it views them");
    CHECK(owned != *ViewedPair::make(*second, *first), "two pairs of different lists differ");

    const ViewedPair empty{};
    CHECK(empty.empty() && empty.first().empty() && empty.second().empty(), "the default holds two empty lists");
}

}  // namespace

int main() {
    make_admits_exactly_the_lists_within_the_bounds();
    a_held_list_reads_as_the_list_it_holds();
    make_admits_a_text_only_of_well_formed_utf8();
    a_default_holds_the_fewest_elements_the_bounds_admit();
    make_admits_exactly_two_lists_of_one_count();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("bounded_runtime_test: all checks passed");
    return 0;
}
