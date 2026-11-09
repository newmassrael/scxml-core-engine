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

#include "sce/forge/algorithm.h"

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

// The places a generated machine holds a text, a byte string and a list in
// (docs/adr/0086, section 7.4) are the owning forms, under the names the
// generator spells. Each is lent read only wherever the list it holds is
// expected, and a text is viewed wherever a `std::string_view` is; nothing
// converts into one but its own `make`, `literal` and a widening from a
// tighter bound.
using HeldText = ::SCE::Forge::OwnedText<4>;
using HeldBytes = ::SCE::Forge::OwnedBytes<3>;
using HeldList = ::SCE::Forge::OwnedList<std::uint16_t, 2>;
static_assert(std::is_same<HeldText, Bounded<std::string, 4>>::value, "a held text is an owning text");
static_assert(std::is_same<HeldBytes, Bounded<std::vector<std::uint8_t>, 3>>::value, "a held byte string too");
static_assert(std::is_same<HeldList::value_type, std::uint16_t>::value, "a list names the type of its element");
static_assert(std::is_convertible<const HeldText &, const std::string &>::value, "a held text is lent as a string");
static_assert(std::is_convertible<const HeldText &, std::string_view>::value, "and viewed as one");
static_assert(std::is_convertible<const HeldList &, const std::vector<std::uint16_t> &>::value,
              "a held list is lent as the vector it holds");
static_assert(!std::is_convertible<const Text<4> &, const std::string &>::value, "a view lends no string");
static_assert(!std::is_constructible<HeldText, std::string>::value, "a bare string is checked by make");
static_assert(!std::is_constructible<HeldText, const char *>::value, "and so is a pointer to text");
static_assert(std::is_convertible<::SCE::Forge::OwnedText<2>, HeldText>::value, "a tighter bound widens");
static_assert(!std::is_constructible<::SCE::Forge::OwnedText<2>, HeldText>::value, "a looser one does not");

void a_held_place_starts_at_a_literal_the_compiler_judges() {
    const HeldText text = HeldText::literal("beta");
    CHECK(text == "beta" && "beta" == text && text != "bet" && text.size() == 4, "a text literal is held whole");
    const HeldText zero = HeldText::literal("a\0b");
    CHECK(zero.size() == 3 && zero[1] == '\0', "a zero inside a literal is one of its bytes");
    const HeldBytes bytes = HeldBytes::literal({0x01, 0xFF});
    CHECK((bytes == std::vector<std::uint8_t>{0x01, 0xFF}), "a byte-string literal holds each byte");
    const std::string &lent = text;
    const std::string_view viewed = text;
    CHECK(lent == "beta" && viewed == "beta" && std::string(text) == "beta", "a held text reads as the string");
}

void a_held_list_grows_only_within_its_bound() {
    const std::uint16_t seven = 7;
    const std::uint16_t eight = 8;
    const std::uint16_t nine = 9;
    HeldList list;
    CHECK(list.push(seven) && list.push(eight), "two elements lie within a bound of two");
    CHECK(!list.push(nine), "a third is refused");
    CHECK(list.size() == 2 && list[0] == seven && list[1] == eight, "and the list is as it was before it");
    const std::vector<std::uint16_t> &lent = list;
    CHECK((lent.size() == 2 && lent == std::vector<std::uint16_t>{seven, eight}), "it is lent as the vector it holds");
    list.clear();
    CHECK(list.empty() && list.push(nine) && list.size() == 1, "an emptied list takes elements again");

    const auto widened = ::SCE::Forge::widenedMap<HeldText>(
        ::SCE::Forge::OwnedList<::SCE::Forge::OwnedText<2>, 2>::literal(
            {::SCE::Forge::OwnedText<2>::literal("ab"), ::SCE::Forge::OwnedText<2>::literal("c")}),
        [](const ::SCE::Forge::OwnedText<2> &element) -> HeldText { return element; });
    const ::SCE::Forge::OwnedList<HeldText, 3> place = widened;
    CHECK(place.size() == 2 && place[0] == "ab" && place[1] == "c", "a list widens element by element");
}

void a_held_place_is_written_only_within_its_bound() {
    ::SCE::Forge::AlgorithmFailure fits;
    const HeldText within = ::SCE::Forge::Checked::held<HeldText>(fits, std::string("beta"));
    CHECK(!fits.failed() && within == "beta", "a text within the bound is held");
    const HeldText widened = ::SCE::Forge::Checked::held<HeldText>(fits, ::SCE::Forge::OwnedText<2>::literal("ab"));
    CHECK(!fits.failed() && widened == "ab", "a held text of another bound is copied as the text it is");

    ::SCE::Forge::AlgorithmFailure past;
    const HeldText refused = ::SCE::Forge::Checked::held<HeldText>(past, "gamma");
    CHECK(past.failed() && past.error() == ::SCE::Forge::AlgorithmError::CapacityExceeded,
          "a text past the bound is a capacity failure");
    CHECK(refused.empty(), "and nothing of it is held, never a part cut to fit");

    ::SCE::Forge::AlgorithmFailure notText;
    const HeldText broken = ::SCE::Forge::Checked::held<HeldText>(notText, std::string("\xC3"));
    CHECK(notText.failed() && broken.empty(), "a text that is not UTF-8 has no place either");

    ::SCE::Forge::AlgorithmFailure listFits;
    const auto list = ::SCE::Forge::Checked::heldList<2>(listFits, std::vector<std::uint16_t>{1, 2});
    CHECK(!listFits.failed() && list.size() == 2, "an answered list within the bound is held");
    ::SCE::Forge::AlgorithmFailure listPast;
    const auto long_list = ::SCE::Forge::Checked::heldList<2>(listPast, std::vector<std::uint16_t>{1, 2, 3});
    CHECK(listPast.failed() && long_list.empty(), "one past the bound is refused whole");
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

// A range a source presents for a list no bound refines is any begin and end
// pair whose elements convert to the element type; walking one that throws is
// told apart from one that does not.
using ::SCE::Forge::Detail::IsNothrowRange;
using ::SCE::Forge::Detail::IsNothrowRangeOf;
using ::SCE::Forge::Detail::IsRange;
using ::SCE::Forge::Detail::IsRangeOf;

/// A range whose read of an element may throw.
struct ThrowingRange {
    struct Iterator {
        const std::string *at;

        bool operator!=(const Iterator &other) const noexcept {
            return at != other.at;
        }

        Iterator &operator++() noexcept {
            ++at;
            return *this;
        }

        const std::string &operator*() const {
            return *at;
        }
    };

    Iterator begin() const noexcept {
        return Iterator{nullptr};
    }

    Iterator end() const noexcept {
        return Iterator{nullptr};
    }
};

static_assert(IsRange<std::vector<std::string>>::value, "a vector is a range");
static_assert(IsRange<Text<8>>::value, "a bounded view is a range");
static_assert(!IsRange<int>::value, "a number is no range");
static_assert(IsRangeOf<std::vector<std::string>, std::string_view>::value, "texts are viewed as text");
static_assert(!IsRangeOf<std::vector<int>, std::string_view>::value, "numbers are not text");
static_assert(!IsRangeOf<int, int>::value, "a number is no range of numbers");
static_assert(IsNothrowRange<std::vector<std::string>>::value, "walking a vector throws nothing");
static_assert(IsNothrowRangeOf<std::vector<std::string>, std::string_view>::value,
              "viewing each text of a vector throws nothing");
static_assert(IsRangeOf<std::vector<const char *>, std::string>::value, "a pointer to text converts to a string");
static_assert(!IsNothrowRangeOf<std::vector<const char *>, std::string>::value, "copying each into a string can throw");
static_assert(IsRangeOf<ThrowingRange, std::string_view>::value, "a range of any iterator is a range");
static_assert(!IsNothrowRange<ThrowingRange>::value, "a read that can throw is seen");

}  // namespace

int main() {
    make_admits_exactly_the_lists_within_the_bounds();
    a_held_list_reads_as_the_list_it_holds();
    make_admits_a_text_only_of_well_formed_utf8();
    a_default_holds_the_fewest_elements_the_bounds_admit();
    make_admits_exactly_two_lists_of_one_count();
    a_held_place_starts_at_a_literal_the_compiler_judges();
    a_held_list_grows_only_within_its_bound();
    a_held_place_is_written_only_within_its_bound();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("bounded_runtime_test: all checks passed");
    return 0;
}
