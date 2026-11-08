// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/membership.h — chains refined by the kinds of their entries
// (docs/adr/0065 §6 R6), which a generated C++ value holds in the arm of the
// gates on a chain's entries (docs/adr/0081, step 4). What converts to what is
// decided by the compiler, so those properties are `static_assert`s: a chain of
// the same kinds within narrower bounds converts, to a view without a copy and to
// an owning list by an explicit copy, and one of other kinds does not. What
// `make` admits and what a held chain reads are checked at run time. This target
// is built at C++17, the standard it is for.

#include "sce/forge/membership.h"

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <type_traits>
#include <vector>

#include "sce/forge/span.h"

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

using ::SCE::Forge::Chain;
using ::SCE::Forge::Kinds;
using ::SCE::Forge::Placed;
using ::SCE::Forge::Span;

/// An entry of a chain: its identifier and what it holds.
struct Entry {
    std::uint8_t id;
    std::uint8_t body;
};

constexpr bool operator==(const Entry &a, const Entry &b) {
    return a.id == b.id && a.body == b.body;
}

/// The kinds a gate names: an entry whose identifier is 7, and one whose is 9.
struct Seven {
    template <typename E> static constexpr bool matches(const E &entry) {
        return entry.id == 7;
    }
};

struct Nine {
    template <typename E> static constexpr bool matches(const E &entry) {
        return entry.id == 9;
    }
};

using Owned = std::vector<Entry>;
using Viewed = Span<const Entry>;

// The arm where the chain holds a 7 and no 9, held and viewed.
using HeldSevenNotNine = Chain<Owned, Kinds<Seven>, Kinds<Nine>, 3, 1>;
using SevenNotNine = Chain<Viewed, Kinds<Seven>, Kinds<Nine>, 3, 1>;

static_assert(std::is_convertible<HeldSevenNotNine, SevenNotNine>::value, "a held chain is viewed implicitly");
static_assert(!std::is_convertible<SevenNotNine, HeldSevenNotNine>::value, "a view is not copied implicitly");
static_assert(std::is_constructible<HeldSevenNotNine, SevenNotNine>::value, "a view is copied explicitly");
static_assert(std::is_convertible<Chain<Viewed, Kinds<Seven>, Kinds<Nine>, 2, 1>, SevenNotNine>::value,
              "a chain within a tighter bound converts");
static_assert(!std::is_constructible<Chain<Viewed, Kinds<Seven>, Kinds<Nine>, 2, 1>, SevenNotNine>::value,
              "one within a looser bound does not");
static_assert(!std::is_constructible<Chain<Viewed, Kinds<Nine>, Kinds<Seven>, 3, 1>, SevenNotNine>::value,
              "a chain of other kinds does not convert");
static_assert(!std::is_constructible<HeldSevenNotNine, Owned>::value, "a bare list is checked by make");

// Only a chain that must hold no kind has a default, the fewest entries
// value-made; a view holds no list of its own where it must hold some.
static_assert(std::is_default_constructible<Chain<Owned, Kinds<>, Kinds<Seven>, 3, 1>>::value,
              "a chain that must hold no kind holds the fewest value-made entries");
static_assert(!std::is_default_constructible<HeldSevenNotNine>::value, "one that must hold a kind has none");
static_assert(!std::is_default_constructible<Chain<Viewed, Kinds<>, Kinds<>, 3, 1>>::value,
              "a view has no fewest entries to hold");

void make_admits_exactly_the_chains_of_the_arm() {
    const Owned seven{{1, 0x10}, {7, 0x11}};
    const Owned seven_and_nine{{7, 0x11}, {9, 0x12}};
    const Owned neither{{1, 0x10}};
    const Owned none{};
    const Owned four{{7, 0}, {1, 0}, {1, 0}, {1, 0}};
    CHECK(HeldSevenNotNine::make(seven).has_value(), "a 7 and no 9 is the arm's chain");
    CHECK(!HeldSevenNotNine::make(seven_and_nine).has_value(), "a 9 is a kind the arm lacks");
    CHECK(!HeldSevenNotNine::make(neither).has_value(), "no 7 is a kind the arm holds missing");
    CHECK(!HeldSevenNotNine::make(none).has_value(), "no entry is fewer than the fewest");
    CHECK(!HeldSevenNotNine::make(four).has_value(), "four entries are past the most");
    using Neither = Chain<Owned, Kinds<>, Kinds<Seven, Nine>, 3, 1>;
    CHECK(Neither::make(neither).has_value(), "the arm of no kind holds a chain of neither");
    CHECK(!Neither::make(seven).has_value(), "and not one that holds a 7");
}

void a_held_chain_reads_as_the_list_it_holds() {
    const Owned entries{{1, 0x10}, {7, 0x11}};
    const auto view = SevenNotNine::make(Viewed(entries.data(), entries.size()));
    CHECK(view.has_value(), "the entries are the arm's chain");
    if (!view) {
        return;
    }
    CHECK(view->size() == 2 && !view->empty() && (*view)[1].body == 0x11, "a view reads the entries in place");
    std::size_t bodies = 0;
    for (const Entry &entry : *view) {
        bodies += entry.body;
    }
    CHECK(bodies == 0x21, "it iterates the entries in order");

    const HeldSevenNotNine held(*view);
    CHECK(held == *view && held.inner().data() != entries.data(), "an explicit copy holds the same entries");
    const SevenNotNine again = held;
    CHECK(again == held && again.inner().data() == held.inner().data(), "a view of a held chain views it");

    const Chain<Owned, Kinds<>, Kinds<Seven>, 3, 1> fewest{};
    CHECK(fewest.size() == 1 && fewest[0].id == 0, "the default holds the fewest entries, value-made");
}

// An arm whose kinds an absent chain also has holds the chain optionally, and
// keeps the list it holds alike, present or not.
static_assert(
    std::is_same<::SCE::Forge::Detail::InnerRead<const std::optional<HeldSevenNotNine> &>, const Owned &>::value,
    "an optional chain read by reference is kept as its list by reference");
static_assert(std::is_same<::SCE::Forge::Detail::InnerRead<std::optional<SevenNotNine>>, Viewed>::value,
              "one read by value is kept as a copy of its list");
static_assert(std::is_same<::SCE::Forge::Detail::InnerRead<const HeldSevenNotNine &>,
                           ::SCE::Forge::Detail::InnerRead<const std::optional<HeldSevenNotNine> &>>::value,
              "the arms that hold it optionally and plainly keep one list");

void an_absent_chain_holds_no_entry_of_any_kind() {
    const std::optional<Owned> absent{};
    const std::optional<Owned> present{Owned{{1, 0x10}, {7, 0x11}}};
    CHECK(!::SCE::Forge::Detail::first_of_kind<Seven>(absent).has_value(), "an absent chain holds no 7");
    CHECK(::SCE::Forge::Detail::first_of_kind<Seven>(present) == std::optional<std::size_t>{1},
          "a present one holds its 7 where it stands");
    CHECK(!::SCE::Forge::Detail::first_of_kind<Nine>(present).has_value(), "and no 9 it does not hold");
}

using HeldPlaced = Placed<Owned, Seven, 3>;
using ViewedPlaced = Placed<Viewed, Seven, 3>;

static_assert(std::is_convertible<HeldPlaced, ViewedPlaced>::value, "held others are viewed implicitly");
static_assert(!std::is_convertible<ViewedPlaced, HeldPlaced>::value, "viewed others are not copied implicitly");
static_assert(std::is_constructible<HeldPlaced, ViewedPlaced>::value, "viewed others are copied explicitly");
static_assert(!std::is_constructible<Placed<Viewed, Nine, 3>, ViewedPlaced>::value,
              "a chain that names another kind does not convert");

void make_places_the_named_entry_among_the_others() {
    const Owned two{{1, 0x10}, {2, 0x20}};
    const Owned with_seven_after{{1, 0x10}, {7, 0x70}};
    CHECK(HeldPlaced::make(two, 0).has_value(), "the named entry may stand first");
    CHECK(HeldPlaced::make(two, 2).has_value(), "or last");
    CHECK(!HeldPlaced::make(two, 3).has_value(), "but not past the others");
    CHECK(!HeldPlaced::make(Owned{{1, 0}, {2, 0}, {3, 0}}, 0).has_value(),
          "three others and the named entry are past the most");
    CHECK(HeldPlaced::make(with_seven_after, 1).has_value(), "an entry of the kind may follow the named one");
    CHECK(!HeldPlaced::make(with_seven_after, 2).has_value(), "but not precede it: the named entry is then that one");

    const auto placed = HeldPlaced::make(two, 1);
    CHECK(placed && placed->size() == 3 && placed->place() == 1 && placed->rest() == two,
          "it holds the others and where the named entry stands");
    if (!placed) {
        return;
    }
    const ViewedPlaced viewed = *placed;
    CHECK(viewed == *placed && viewed.rest().data() == placed->rest().data(), "a view views the others");
    CHECK(*placed != *HeldPlaced::make(two, 0), "the same others with the named entry elsewhere differ");

    const HeldPlaced alone{};
    CHECK(alone.size() == 1 && alone.place() == 0 && alone.rest().empty(), "the default is the named entry alone");
}

}  // namespace

int main() {
    make_admits_exactly_the_chains_of_the_arm();
    a_held_chain_reads_as_the_list_it_holds();
    an_absent_chain_holds_no_entry_of_any_kind();
    make_places_the_named_entry_among_the_others();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("membership_runtime_test: all checks passed");
    return 0;
}
