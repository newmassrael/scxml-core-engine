// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reader and the writer of the canonical form a test configuration's
// header generates for each record its system ports take
// (docs/SCE_TTCN3_LANE.md §10.3, §10.5; docs/SCE_TEST_PROFILE.md §12.5),
// compiled here over records of every form C++ holds: order_taken (a list of
// records with a closed enum, an optional text and integer, a record with a
// truth value, a byte string), job_status (a list of integers, optional
// integers) and form_extras (an enum of an open set, a list of optional
// elements, a 64-bit integer, a list with a lower count, an optional list and
// a list of lists). The headers are generated from
// sce-build/tests/fixtures/test_profile/trace_forms/ (tests/CMakeLists.txt).
//
// The twin of backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs,
// with the same values and the same refusals, but for the union the Rust
// configuration's order_placed holds, which C++ does not lower: order_taken
// holds every other form order_placed does, and a byte string in the union's
// place. A value written by `canonical<Record>` and read back by
// `read<Record>` is the value it was, through the canonical text; each
// malformed spelling of a form is refused, naming where it stands; and a trace
// of one record of each type replays to a log that holds each value as the
// trace does.

#include <gtest/gtest.h>

#include <cstdint>
#include <limits>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "replays_every_unionless_form.h"
#include "saved/SavedJson.h"
#include "test/Suite.h"
#include "test/Trace.h"
#include "test/TraceRefusal.h"
#include "unionless_forms_configuration.h"

namespace {

using SCE::Saved::JsonValue;
using SCE::Saved::parseJson;
using SCE::Saved::writeCanonical;
using SCE::Test::Trace;
using SCE::Test::TraceInput;
using SCE::Test::TraceRefusal;
using SCE::Test::Verdict;

namespace Forms = SCE::Generated::UnionlessFormsConfiguration;

using Order = SCE::Generated::OrderTaken::OrderTakenPayload;
using OrderLine = SCE::Generated::OrderLine::OrderLinePayload;
using Customer = SCE::Generated::OrderCustomer::OrderCustomerPayload;
using Color = SCE::Generated::PaintColor::PaintColor;
using Status = SCE::Generated::JobStatus::JobStatusPayload;
using Extras = SCE::Generated::FormExtras::FormExtrasPayload;
using Shape = SCE::Generated::ShapeKindOpen::ShapeKindOpen;

// The types the schema units hold a bounded text and list in (docs/adr/0086,
// section 7.4).
using Lines = decltype(Order::lines);
using Sku = decltype(OrderLine::sku);
using Note = decltype(Order::note)::value_type;
using FullName = decltype(Customer::full_name);
using Codes = decltype(Status::codes);
using StatusNote = decltype(Status::note)::value_type;
using Slots = decltype(Extras::slots);
using Flags = decltype(Extras::flags);
using Spare = decltype(Extras::spare)::value_type;
using Grid = decltype(Extras::grid);
using Row = Grid::value_type;

/// The writer of a record's canonical form, and its reader.
template <typename V> using Writer = JsonValue (*)(const V &);
template <typename V> using Reader = V (*)(const JsonValue &);

/// `list` held in `Held`, the type of a place that holds its bound; a list
/// past it fails the test, since every value below lies within its place's
/// bound.
template <typename Held, typename List> Held held(List list) {
    auto made = Held::make(std::move(list));
    EXPECT_TRUE(made.has_value()) << "the value lies within its place's bound";
    return made.value_or(Held{});
}

/// A line of an order.
OrderLine line(const std::string &sku, std::uint16_t quantity, Color color) {
    OrderLine made{};
    made.sku = held<Sku>(sku);
    made.quantity = quantity;
    made.color = color;
    return made;
}

/// An order of two lines, one with a text of a character of two bytes and an
/// escaped quote, the widest order id, a note and no priority, paid with the
/// voucher `voucher`.
Order order(std::vector<std::uint8_t> voucher) {
    Order made{};
    made.order_id = std::numeric_limits<std::uint32_t>::max();
    made.lines = held<Lines>(std::vector<OrderLine>{
        line("A-1", 2, Color::Green), line("\"q\" \xC3\xA9", std::numeric_limits<std::uint16_t>::max(), Color::Blue)});
    made.note = Note::literal("ring twice");
    made.priority = std::nullopt;
    made.customer.full_name = FullName::literal("Ada");
    made.customer.vip = true;
    made.voucher_code = std::move(voucher);
    return made;
}

/// The vouchers the order is paid with: three octets, and none.
std::vector<std::vector<std::uint8_t>> vouchers() {
    return {{0x00, 0xab, 0xff}, {}};
}

std::vector<Status> statuses() {
    Status first{};
    first.elapsed_ms = 0;
    first.note = std::nullopt;
    first.codes = Codes{};
    first.tag = std::numeric_limits<std::uint8_t>::max();
    first.retry = std::nullopt;
    first.count = 7;
    Status second{};
    second.elapsed_ms = 12;
    second.note = StatusNote::literal("late");
    second.codes = Codes::literal({3, 9, 5, 0});
    second.tag = std::nullopt;
    second.retry = std::uint8_t{0};
    second.count = 0;
    return {first, second};
}

/// The extras: a declared variant and a number no variant declares, a list
/// with an absent element, the least and the greatest 64-bit integer, a list
/// at its lower count and at its bound, an optional list present and absent,
/// and a list of lists with an empty one.
std::vector<Extras> extras() {
    Extras first{};
    first.shape = Shape::Circle;
    first.slots = held<Slots>(std::vector<std::optional<std::uint16_t>>{std::uint16_t{1}, std::nullopt,
                                                                        std::numeric_limits<std::uint16_t>::max()});
    first.least = std::numeric_limits<std::int64_t>::min();
    first.flags = Flags::literal({true});
    first.spare = Spare::literal({0, 255});
    first.grid = held<Grid>(std::vector<Row>{Row::literal({1, 2}), Row{}});
    Extras second{};
    second.shape = SCE::Generated::ShapeKindOpen::from_underlying(9);
    second.slots = Slots{};
    second.least = std::numeric_limits<std::int64_t>::max();
    second.flags = Flags::literal({false, true});
    second.spare = std::nullopt;
    second.grid = Grid{};
    return {first, second};
}

/// `value` written by `write` in its canonical text, read back from that text
/// by `read`, and written again: the same text, so the reader holds every
/// field the writer wrote, as the writer wrote it.
template <typename V> void roundTrip(const V &value, Writer<V> write, Reader<V> read) {
    const std::string written = writeCanonical(write(value));
    EXPECT_EQ(writeCanonical(write(read(parseJson(written)))), written);
}

/// docs/SCE_TEST_PROFILE.md §12.5: a value of each form reads back from the
/// text its writer gives as the value it was.
TEST(ARecord, ReadsBackAValueOfEachFormFromItsCanonicalText) {
    for (const auto &voucher : vouchers()) {
        roundTrip<Order>(order(voucher), &Forms::canonicalOrderTaken, &Forms::readOrderTaken);
    }
    for (const Status &status : statuses()) {
        roundTrip<Status>(status, &Forms::canonicalJobStatus, &Forms::readJobStatus);
    }
    for (const Extras &extra : extras()) {
        roundTrip<Extras>(extra, &Forms::canonicalFormExtras, &Forms::readFormExtras);
    }
    const Extras read = Forms::readFormExtras(Forms::canonicalFormExtras(extras()[1]));
    EXPECT_EQ(SCE::Generated::ShapeKindOpen::to_underlying(read.shape), 9) << "a number no variant declares";
    EXPECT_EQ(read.least, std::numeric_limits<std::int64_t>::max());
    const Order paid = Forms::readOrderTaken(Forms::canonicalOrderTaken(order(vouchers()[0])));
    EXPECT_EQ(paid.voucher_code, (std::vector<std::uint8_t>{0x00, 0xab, 0xff}));
    ASSERT_EQ(paid.lines.size(), 2u);
    EXPECT_EQ(paid.lines[1].sku.inner(), "\"q\" \xC3\xA9");
    EXPECT_FALSE(paid.priority.has_value());
}

/// The reason `read` refuses the canonical text of `value` with `from`
/// replaced once by `to` for.
template <typename V>
std::string refused(const V &value, Writer<V> write, Reader<V> read, const std::string &from, const std::string &to) {
    std::string text = writeCanonical(write(value));
    const std::size_t at = text.find(from);
    EXPECT_NE(at, std::string::npos) << "`" << from << "` in " << text;
    if (at == std::string::npos) {
        return "";
    }
    text.replace(at, from.size(), to);
    const JsonValue changed = parseJson(text);
    try {
        (void)read(changed);
    } catch (const TraceRefusal &refusal) {
        return refusal.reason();
    }
    ADD_FAILURE() << text << " is read";
    return "";
}

/// docs/SCE_TTCN3_LANE.md §10.2: a malformed spelling of each form is refused,
/// naming the place it stands at: the record's type, each field by its id and
/// `[]` for an element of a list.
TEST(ARecord, RefusesAMalformedSpellingOfEachFormByName) {
    const Order paid = order(vouchers()[0]);
    const auto ofOrder = [&paid](const std::string &from, const std::string &to) {
        return refused<Order>(paid, &Forms::canonicalOrderTaken, &Forms::readOrderTaken, from, to);
    };
    const Extras extra = extras()[0];
    const auto ofExtras = [&extra](const std::string &from, const std::string &to) {
        return refused<Extras>(extra, &Forms::canonicalFormExtras, &Forms::readFormExtras, from, to);
    };
    const Status status = statuses()[0];
    const auto ofStatus = [&status](const std::string &from, const std::string &to) {
        return refused<Status>(status, &Forms::canonicalJobStatus, &Forms::readJobStatus, from, to);
    };
    const std::string longName = "\"full_name\":\"" + std::string(25, 'x') + "\"";
    const std::vector<std::pair<std::string, std::string>> cases = {
        {ofOrder("\"order_id\":\"4294967295\"", "\"order_id\":\"007\""),
         "order_taken.order_id: '007' is not the text of an integer's decimal digits"},
        {ofOrder("\"quantity\":\"2\"", "\"quantity\":\"65536\""),
         "order_taken.lines[].quantity: 65536 is past the range of its type"},
        {ofOrder("\"vip\":true", "\"vip\":\"true\""), "order_taken.customer.vip: a text is not a truth value"},
        {ofOrder("\"full_name\":\"Ada\"", longName),
         "order_taken.customer.full_name: a text of 25 bytes is past the 24 its type holds"},
        {ofOrder("\"color\":\"green\"", "\"color\":\"purple\""),
         "order_taken.lines[].color: 'purple' names no variant of its enum"},
        {ofOrder("\"note\":\"ring twice\"", "\"note\":null"), "order_taken.note: null is not a text"},
        {ofOrder("{\"customer\":", "{\"rush\":\"1\",\"customer\":"), "order_taken: 'rush' is no field of its record"},
        {ofOrder("\"order_id\":\"4294967295\",", ""),
         "order_taken.order_id: the field is absent, and its type is not optional"},
        {ofOrder("\"lines\":[", "\"lines\":[{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
                                "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"
                                "{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},"),
         "order_taken.lines: a list of 5 elements is outside its bounds, 0 to 4"},
        {ofOrder("\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"00abff\""),
         "order_taken.voucher_code: '00abff' is not octets in upper-case hex, two digits each"},
        {ofOrder("\"voucher_code\":\"00ABFF\"", "\"voucher_code\":\"0011223344\""),
         "order_taken.voucher_code: a byte string of 5 octets is past the 4 its type holds"},
        {ofStatus("\"tag\":\"255\"", "\"tag\":\"256\""), "job_status.tag: 256 is past the range of its type"},
        {ofStatus("\"codes\":[]", "\"codes\":[\"x\"]"),
         "job_status.codes[]: 'x' is not the text of an integer's decimal digits"},
        {ofExtras("\"shape\":\"circle\"", "\"shape\":\"1\""),
         "form_extras.shape: 1 is the number of a declared variant, which is written by its name"},
        {ofExtras("\"shape\":\"circle\"", "\"shape\":\"hexagon\""),
         "form_extras.shape: 'hexagon' names no variant of its enum, and is no number"},
        {ofExtras("\"slots\":[\"1\",null,", "\"slots\":[\"1\",\"x\","),
         "form_extras.slots[]: 'x' is not the text of an integer's decimal digits"},
        {ofExtras("\"flags\":[true]", "\"flags\":[]"),
         "form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2"},
        {ofExtras("\"grid\":[[\"1\",\"2\"]", "\"grid\":[[\"1\",\"2\",\"3\"]"),
         "form_extras.grid[]: a list of 3 elements is outside its bounds, 0 to 2"},
        {ofExtras("\"least\":\"-9223372036854775808\"", "\"least\":\"-9223372036854775809\""),
         "form_extras.least: -9223372036854775809 is past the range of its type"},
        {ofExtras("\"spare\":[\"0\",\"255\"]", "\"spare\":null"),
         "form_extras.spare: null is not a list, which is an array"},
    };
    for (const auto &[reason, expected] : cases) {
        EXPECT_EQ(reason, expected);
    }
}

/// docs/SCE_TTCN3_LANE.md §10.5: a trace of one record of each type the
/// system port takes, two at the start and one later, replays to the
/// listener, which takes one a round and passes; the log holds each value as
/// the trace does.
TEST(ARecord, ATraceOfEveryFormReplaysToTheValuesItHolds) {
    const auto input = [](SCE::Test::Instant at, const std::string &type, JsonValue value) {
        TraceInput made;
        made.at = at;
        made.port = "sut";
        made.messageType = type;
        made.value = std::move(value);
        return made;
    };
    Trace trace;
    trace.suite = "forms_suite";
    trace.testCase = SCE::Generated::ReplaysEveryUnionlessForm::NAME;
    trace.inputs.push_back(input(0, "order_taken", Forms::canonicalOrderTaken(order(vouchers()[0]))));
    trace.inputs.push_back(input(0, "job_status", Forms::canonicalJobStatus(statuses()[0])));
    trace.inputs.push_back(input(250, "form_extras", Forms::canonicalFormExtras(extras()[0])));
    const std::string text = trace.canonicalText();
    SCE::Test::Suite suite;
    suite.name = "forms_suite";
    suite.executes.push_back(
        SCE::Test::Execute{SCE::Generated::ReplaysEveryUnionlessForm::testCase(), std::nullopt, nullptr});
    const auto results =
        SCE::Test::runSuiteWithTraces(suite, {}, [&text](const std::string &) { return Trace::parse(text); });
    ASSERT_EQ(results.size(), 1u);
    EXPECT_EQ(results[0].result.verdict, Verdict::Pass);
    EXPECT_EQ(results[0].result.endedAt, 250);
    const std::string log = results[0].canonicalLog("forms_suite");
    for (const TraceInput &given : trace.inputs) {
        const std::string received = "\"kind\":\"message-received\",\"port\":\"sut\",\"type\":\"" + given.messageType +
                                     "\",\"value\":" + writeCanonical(given.value);
        EXPECT_NE(log.find(received), std::string::npos) << "`" << received << "` in " << log;
    }
}

}  // namespace
