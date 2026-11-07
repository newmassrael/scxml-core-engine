// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/law.h — a list whose entries say their inputs by a law (docs/adr/0065
// §6 R8), which a generated C++ value holds a list of such entries in
// (docs/adr/0081, step 4). What converts to what is decided by the compiler, so
// those properties are `static_assert`s: a `Lawful` of one law converts to one of
// the same law over a list its own converts to, to a view without a copy and to
// an owning list by an explicit copy, and never to one of another law. What
// `make` admits, what start a list made by default follows from, and what a held
// list reads are checked at run time. This target is built at C++17, the
// standard it is for.

#include "sce/forge/bounded.h"
#include "sce/forge/law.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <tuple>
#include <type_traits>
#include <utility>
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

using ::SCE::Forge::AfterKey;
using ::SCE::Forge::Bounded;
using ::SCE::Forge::FixedStart;
using ::SCE::Forge::Lawful;
using ::SCE::Forge::Laws;
using ::SCE::Forge::List;
using ::SCE::Forge::Uniform;

// An entry of a kind, saying one input bound to every entry (`bound`) and one
// supplied from the entry before (`after`); an entry that says nothing of an
// input holds no value for it.
struct Entry {
    std::uint8_t kind;
    std::optional<bool> bound;
    std::optional<bool> after;

    bool operator==(const Entry &other) const {
        return kind == other.kind && bound == other.bound && after == other.after;
    }
};

struct SaysBound {
    static std::optional<bool> says(const Entry &entry) {
        return entry.bound;
    }
};

struct SaysAfter {
    static std::optional<bool> says(const Entry &entry) {
        return entry.after;
    }
};

// The kind the entry after which says `after` set.
struct IsMarker {
    static bool matches(const Entry &entry) {
        return entry.kind == 4;
    }
};

using Bound = Uniform<SaysBound>;
using After = AfterKey<SaysAfter, IsMarker>;
using Both = Laws<Bound, After>;

using Owned = Bounded<std::vector<Entry>, 4>;
using Viewed = List<Entry, 4>;
// Named, so that a check's condition holds no comma of a template's arguments.
using BoundList = Lawful<Owned, Bound>;
using AfterList = Lawful<Owned, After>;
using BothList = Lawful<Owned, Both>;

static_assert(std::is_same<Bound::Start, bool>::value, "a bound flag's start is the flag");
static_assert(std::is_same<After::Start, FixedStart>::value, "the entry before has a fixed start");
static_assert(std::is_same<Both::Start, std::tuple<bool, FixedStart>>::value, "laws run together start from each");

// A lawful owned list is viewed without a copy; a view is held by an explicit
// copy; neither converts to a list of another law.
static_assert(std::is_convertible<Lawful<Owned, Bound>, Lawful<Viewed, Bound>>::value,
              "an owned lawful list is viewed implicitly");
static_assert(std::is_nothrow_constructible<Lawful<Viewed, Bound>, const Lawful<Owned, Bound> &>::value,
              "viewing an owned lawful list throws nothing");
static_assert(!std::is_convertible<Lawful<Viewed, Bound>, Lawful<Owned, Bound>>::value,
              "a lawful view is not copied implicitly");
static_assert(std::is_constructible<Lawful<Owned, Bound>, Lawful<Viewed, Bound>>::value,
              "a lawful view is copied explicitly");
static_assert(!std::is_constructible<Lawful<Viewed, After>, Lawful<Owned, Bound>>::value,
              "a list held to one law is not one held to another");

// A list holds no entry its law refuses: a bare list is no `Lawful`, and the
// only door with a start of the caller's is `make`.
static_assert(!std::is_constructible<Lawful<Owned, Bound>, Owned>::value, "a bare list is checked by make");
static_assert(!std::is_constructible<Lawful<Owned, Bound>, bool, Owned>::value, "a bare list is checked by make");

Owned owned(std::vector<Entry> entries) {
    return *Owned::make(std::move(entries));
}

void a_bound_flag_is_said_alike_by_every_entry() {
    const Owned agree = owned({Entry{1, true, std::nullopt}, Entry{2, true, std::nullopt}});
    const auto set = BoundList::make(true, agree);
    CHECK(set.has_value() && set->start() && set->size() == 2, "entries that say the flag set follow it set");
    CHECK(!BoundList::make(false, agree).has_value(), "entries that say it set do not follow it clear");

    const Owned silent = owned({Entry{1, std::nullopt, std::nullopt}, Entry{2, false, std::nullopt}});
    CHECK(BoundList::make(false, silent).has_value(), "an entry that says nothing disagrees with no state");
    CHECK(!BoundList::make(true, silent).has_value(), "one entry that says otherwise breaks the law");

    CHECK(BoundList::make(true, Owned{}).has_value(), "an empty list follows any start");
}

void an_input_from_the_entry_before_is_set_only_right_after_its_kind() {
    const Owned chain =
        owned({Entry{4, std::nullopt, false}, Entry{3, std::nullopt, true}, Entry{3, std::nullopt, false}});
    CHECK(AfterList::make(FixedStart{}, chain).has_value(), "set right after the marker, clear elsewhere");

    const Owned first_set = owned({Entry{3, std::nullopt, true}});
    CHECK(!AfterList::make(FixedStart{}, first_set).has_value(), "the first entry has no entry before it");

    const Owned stays_set =
        owned({Entry{4, std::nullopt, false}, Entry{3, std::nullopt, true}, Entry{3, std::nullopt, true}});
    CHECK(!AfterList::make(FixedStart{}, stays_set).has_value(), "only the entry right after the marker says it set");

    const Owned adjacent =
        owned({Entry{4, std::nullopt, false}, Entry{4, std::nullopt, true}, Entry{3, std::nullopt, true}});
    CHECK(AfterList::make(FixedStart{}, adjacent).has_value(), "a marker after a marker sets the next");
}

void laws_run_together_hold_every_entry_to_each() {
    const Owned both = owned({Entry{4, true, false}, Entry{3, true, true}});
    const auto held = BothList::make(std::make_tuple(true, FixedStart{}), both);
    CHECK(held.has_value() && std::get<0>(held->start()), "entries that follow both laws are held to both");
    CHECK(!BothList::make(std::make_tuple(false, FixedStart{}), both).has_value(),
          "an entry that breaks one law breaks the two");
}

void a_lawful_list_reads_as_the_list_it_holds() {
    const Owned entries = owned({Entry{1, true, std::nullopt}, Entry{2, true, std::nullopt}});
    const auto held = BoundList::make(true, entries);
    CHECK(held.has_value(), "two entries that say the flag set follow it set");
    if (!held) {
        return;
    }
    const Lawful<Viewed, Bound> viewed = *held;
    CHECK(viewed.size() == 2 && viewed[1].kind == 2 && viewed.start(), "a view reads the entries and the start");
    std::size_t walked = 0;
    for (const Entry &entry : viewed) {
        walked += entry.kind;
    }
    CHECK(walked == 3, "a view is walked in order");
    const Lawful<Owned, Bound> copied(viewed);
    CHECK(copied == *held, "a copy of a view holds the same entries and start");
    const auto other = BoundList::make(true, owned({Entry{1, true, std::nullopt}}));
    CHECK(other.has_value() && *other != *held, "lists of other entries differ");
}

void a_list_made_by_default_follows_the_start_its_first_entry_says() {
    const Lawful<Owned, Bound> empty;
    CHECK(empty.empty() && !empty.start(), "an empty list starts clear");
    using AtLeastOne = Bounded<std::vector<Entry>, 4, 1>;
    const Lawful<AtLeastOne, After> one;
    CHECK(one.size() == 1 && one.start() == FixedStart{}, "a list that must hold an entry holds a default one");
}

}  // namespace

int main() {
    a_bound_flag_is_said_alike_by_every_entry();
    an_input_from_the_entry_before_is_set_only_right_after_its_kind();
    laws_run_together_hold_every_entry_to_each();
    a_lawful_list_reads_as_the_list_it_holds();
    a_list_made_by_default_follows_the_start_its_first_entry_says();
    if (failures != 0) {
        std::fprintf(stderr, "law_runtime_test: %d failure(s)\n", failures);
        return 1;
    }
    std::printf("law_runtime_test: ok\n");
    return 0;
}
