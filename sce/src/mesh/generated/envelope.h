// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: envelope.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_ENVELOPE_H
#define SCE_FORGE_ENVELOPE_H

#include <cstdint>
#include <limits>
#include <optional>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#include "causal_entry.h"
#include "pattern_kind.h"
#include "payload_codec.h"
#include "rpc_status.h"
#include "sce/forge/cbor.h"
#include "sce/forge/codec.h"

namespace SCE::Generated::Envelope {

/// The entries a value of `Envelope` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_id {
void id() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.id())) -> decltype(s.id()) {
    return s.id();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(id(s))) -> decltype(id(s)) {
    return id(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_id

/// The `id` element of a source.
inline constexpr detail_id::Read id{};

namespace detail_source {
void source() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.source())) -> decltype(s.source()) {
    return s.source();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(source(s))) -> decltype(source(s)) {
    return source(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_source

/// The `source` element of a source.
inline constexpr detail_source::Read source{};

namespace detail_event_type {
void event_type() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.event_type()))
    -> decltype(s.event_type()) {
    return s.event_type();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(event_type(s))) -> decltype(event_type(s)) {
    return event_type(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_event_type

/// The `event_type` element of a source.
inline constexpr detail_event_type::Read event_type{};

namespace detail_pattern {
void pattern() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.pattern())) -> decltype(s.pattern()) {
    return s.pattern();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(pattern(s))) -> decltype(pattern(s)) {
    return pattern(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_pattern

/// The `pattern` element of a source.
inline constexpr detail_pattern::Read pattern{};

namespace detail_datacontenttype {
void datacontenttype() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.datacontenttype()))
    -> decltype(s.datacontenttype()) {
    return s.datacontenttype();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(datacontenttype(s)))
    -> decltype(datacontenttype(s)) {
    return datacontenttype(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_datacontenttype

/// The `datacontenttype` element of a source.
inline constexpr detail_datacontenttype::Read datacontenttype{};

namespace detail_data {
void data() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.data())) -> decltype(s.data()) {
    return s.data();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(data(s))) -> decltype(data(s)) {
    return data(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_data

/// The `data` element of a source.
inline constexpr detail_data::Read data{};

namespace detail_subject {
void subject() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.subject())) -> decltype(s.subject()) {
    return s.subject();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(subject(s))) -> decltype(subject(s)) {
    return subject(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_subject

/// The `subject` element of a source.
inline constexpr detail_subject::Read subject{};

namespace detail_correlation_id {
void correlation_id() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.correlation_id()))
    -> decltype(s.correlation_id()) {
    return s.correlation_id();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(correlation_id(s)))
    -> decltype(correlation_id(s)) {
    return correlation_id(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_correlation_id

/// The `correlation_id` element of a source.
inline constexpr detail_correlation_id::Read correlation_id{};

namespace detail_reply_to {
void reply_to() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.reply_to())) -> decltype(s.reply_to()) {
    return s.reply_to();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(reply_to(s))) -> decltype(reply_to(s)) {
    return reply_to(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_reply_to

/// The `reply_to` element of a source.
inline constexpr detail_reply_to::Read reply_to{};

namespace detail_invoke_id {
void invoke_id() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.invoke_id()))
    -> decltype(s.invoke_id()) {
    return s.invoke_id();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(invoke_id(s))) -> decltype(invoke_id(s)) {
    return invoke_id(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_invoke_id

/// The `invoke_id` element of a source.
inline constexpr detail_invoke_id::Read invoke_id{};

namespace detail_rpc_status {
void rpc_status() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.rpc_status()))
    -> decltype(s.rpc_status()) {
    return s.rpc_status();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(rpc_status(s))) -> decltype(rpc_status(s)) {
    return rpc_status(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_rpc_status

/// The `rpc_status` element of a source.
inline constexpr detail_rpc_status::Read rpc_status{};

namespace detail_rpc_error_message {
void rpc_error_message() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.rpc_error_message()))
    -> decltype(s.rpc_error_message()) {
    return s.rpc_error_message();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(rpc_error_message(s)))
    -> decltype(rpc_error_message(s)) {
    return rpc_error_message(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_rpc_error_message

/// The `rpc_error_message` element of a source.
inline constexpr detail_rpc_error_message::Read rpc_error_message{};

namespace detail_deadline_unix_ms {
void deadline_unix_ms() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.deadline_unix_ms()))
    -> decltype(s.deadline_unix_ms()) {
    return s.deadline_unix_ms();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(deadline_unix_ms(s)))
    -> decltype(deadline_unix_ms(s)) {
    return deadline_unix_ms(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_deadline_unix_ms

/// The `deadline_unix_ms` element of a source.
inline constexpr detail_deadline_unix_ms::Read deadline_unix_ms{};

namespace detail_sequence_no {
void sequence_no() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.sequence_no()))
    -> decltype(s.sequence_no()) {
    return s.sequence_no();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(sequence_no(s)))
    -> decltype(sequence_no(s)) {
    return sequence_no(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_sequence_no

/// The `sequence_no` element of a source.
inline constexpr detail_sequence_no::Read sequence_no{};

namespace detail_routing_id {
void routing_id() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.routing_id()))
    -> decltype(s.routing_id()) {
    return s.routing_id();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(routing_id(s))) -> decltype(routing_id(s)) {
    return routing_id(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_routing_id

/// The `routing_id` element of a source.
inline constexpr detail_routing_id::Read routing_id{};

namespace detail_parallel_id {
void parallel_id() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.parallel_id()))
    -> decltype(s.parallel_id()) {
    return s.parallel_id();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(parallel_id(s)))
    -> decltype(parallel_id(s)) {
    return parallel_id(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_parallel_id

/// The `parallel_id` element of a source.
inline constexpr detail_parallel_id::Read parallel_id{};

namespace detail_region_id {
void region_id() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.region_id()))
    -> decltype(s.region_id()) {
    return s.region_id();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(region_id(s))) -> decltype(region_id(s)) {
    return region_id(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_region_id

/// The `region_id` element of a source.
inline constexpr detail_region_id::Read region_id{};

namespace detail_child_session_id {
void child_session_id() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.child_session_id()))
    -> decltype(s.child_session_id()) {
    return s.child_session_id();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(child_session_id(s)))
    -> decltype(child_session_id(s)) {
    return child_session_id(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_child_session_id

/// The `child_session_id` element of a source.
inline constexpr detail_child_session_id::Read child_session_id{};

namespace detail_causal_epoch {
void causal_epoch() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.causal_epoch()))
    -> decltype(s.causal_epoch()) {
    return s.causal_epoch();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(causal_epoch(s)))
    -> decltype(causal_epoch(s)) {
    return causal_epoch(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_causal_epoch

/// The `causal_epoch` element of a source.
inline constexpr detail_causal_epoch::Read causal_epoch{};

namespace detail_causal_vector {
void causal_vector() = delete;

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.causal_vector()))
    -> decltype(s.causal_vector()) {
    return s.causal_vector();
}

template <class S>
constexpr auto read(const S &s, ::SCE::Forge::PreferFree) noexcept(noexcept(causal_vector(s)))
    -> decltype(causal_vector(s)) {
    return causal_vector(s);
}

struct Read {
    template <class S>
    constexpr auto operator()(const S &s) const noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_causal_vector

/// The `causal_vector` element of a source.
inline constexpr detail_causal_vector::Read causal_vector{};
}  // namespace source

/// Whether `S` presents every entry of `Envelope`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void> struct is_source : std::false_type {};

template <class S>
struct is_source<
    S,
    std::void_t<
        decltype(source::id(std::declval<const S &>())), decltype(source::source(std::declval<const S &>())),
        decltype(source::event_type(std::declval<const S &>())), decltype(source::pattern(std::declval<const S &>())),
        decltype(source::datacontenttype(std::declval<const S &>())), decltype(source::data(std::declval<const S &>())),
        decltype(source::subject(std::declval<const S &>())),
        decltype(source::correlation_id(std::declval<const S &>())),
        decltype(source::reply_to(std::declval<const S &>())), decltype(source::invoke_id(std::declval<const S &>())),
        decltype(source::rpc_status(std::declval<const S &>())),
        decltype(source::rpc_error_message(std::declval<const S &>())),
        decltype(source::deadline_unix_ms(std::declval<const S &>())),
        decltype(source::sequence_no(std::declval<const S &>())),
        decltype(source::routing_id(std::declval<const S &>())),
        decltype(source::parallel_id(std::declval<const S &>())),
        decltype(source::region_id(std::declval<const S &>())),
        decltype(source::child_session_id(std::declval<const S &>())),
        decltype(source::causal_epoch(std::declval<const S &>())),
        decltype(source::causal_vector(std::declval<const S &>()))>>
    : std::conjunction<
          std::is_convertible<decltype(source::id(std::declval<const S &>())), ::SCE::Forge::Bytes<16, 16>>,
          std::is_convertible<decltype(source::source(std::declval<const S &>())), ::SCE::Forge::Text<262144>>,
          std::is_convertible<decltype(source::event_type(std::declval<const S &>())), ::SCE::Forge::Text<262144>>,
          std::is_convertible<decltype(source::pattern(std::declval<const S &>())),
                              SCE::Generated::PatternKind::PatternKind>,
          std::is_convertible<decltype(source::datacontenttype(std::declval<const S &>())),
                              SCE::Generated::PayloadCodec::PayloadCodec>,
          std::is_convertible<decltype(source::data(std::declval<const S &>())), ::SCE::Forge::Bytes<16777216>>,
          std::is_convertible<decltype(source::subject(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Text<262144>>>,
          std::is_convertible<decltype(source::correlation_id(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Bytes<16, 16>>>,
          std::is_convertible<decltype(source::reply_to(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Text<262144>>>,
          std::is_convertible<decltype(source::invoke_id(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Bytes<16, 16>>>,
          std::is_convertible<decltype(source::rpc_status(std::declval<const S &>())),
                              std::optional<SCE::Generated::RpcStatus::RpcStatus>>,
          std::is_convertible<decltype(source::rpc_error_message(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Text<262144>>>,
          std::is_convertible<decltype(source::deadline_unix_ms(std::declval<const S &>())), std::optional<uint64_t>>,
          std::is_convertible<decltype(source::sequence_no(std::declval<const S &>())), std::optional<uint64_t>>,
          std::is_convertible<decltype(source::routing_id(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Bytes<16, 16>>>,
          std::is_convertible<decltype(source::parallel_id(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Text<262144>>>,
          std::is_convertible<decltype(source::region_id(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Text<262144>>>,
          std::is_convertible<decltype(source::child_session_id(std::declval<const S &>())),
                              std::optional<::SCE::Forge::Text<262144>>>,
          std::is_convertible<decltype(source::causal_epoch(std::declval<const S &>())), std::optional<uint64_t>>,
          std::is_convertible<decltype(source::causal_vector(std::declval<const S &>())),
                              std::optional<::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>>>> {
};

/// Whether reading every entry of a source `S` throws nothing.
template <class S, class = void> struct is_nothrow_source : std::false_type {};

template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<
          noexcept(
              ::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<16, 16>>(source::id(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Text<262144>>(
              source::source(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Text<262144>>(
              source::event_type(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<SCE::Generated::PatternKind::PatternKind>(
              source::pattern(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<SCE::Generated::PayloadCodec::PayloadCodec>(
              source::datacontenttype(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<16777216>>(
              source::data(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<262144>>>(
              source::subject(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Bytes<16, 16>>>(
              source::correlation_id(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<262144>>>(
              source::reply_to(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Bytes<16, 16>>>(
              source::invoke_id(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<SCE::Generated::RpcStatus::RpcStatus>>(
              source::rpc_status(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<262144>>>(
              source::rpc_error_message(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<uint64_t>>(
              source::deadline_unix_ms(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<uint64_t>>(
              source::sequence_no(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Bytes<16, 16>>>(
              source::routing_id(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<262144>>>(
              source::parallel_id(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<262144>>>(
              source::region_id(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<262144>>>(
              source::child_session_id(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<std::optional<uint64_t>>(
              source::causal_epoch(std::declval<const S &>()))) &&
          noexcept(::SCE::Forge::Detail::converts_to<
                   std::optional<::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>>>(
              source::causal_vector(std::declval<const S &>())))> {};

/// The records the lists of `Envelope` hold, each in the payload type its
/// event schema's own unit declares and written as one CBOR map of every
/// field, each under its id as text (docs/adr/0086, section 9.2).
struct Records {
    /// Write one record of `causal_entry` (`record:CausalEntry`) as one map of every
    /// field, the keys in the order RFC 8949 §4.2.1 sorts their encodings: a
    /// shorter key first.
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError>
    encode_record_causal_entry(::SCE::Forge::SceSink &w,
                               const SCE::Generated::CausalEntry::CausalEntryPayload &record) noexcept {
        namespace cbor = ::SCE::Forge::Cbor;
        if (auto _e = cbor::write_map_head(w, 3); _e) {
            return _e;
        }
        if (auto _e = cbor::write_text(w, "hi"); _e) {
            return _e;
        }
        if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(record.hi)); _e) {
            return _e;
        }
        if (auto _e = cbor::write_text(w, "lo"); _e) {
            return _e;
        }
        if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(record.lo)); _e) {
            return _e;
        }
        if (auto _e = cbor::write_text(w, "seq"); _e) {
            return _e;
        }
        if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(record.seq)); _e) {
            return _e;
        }
        return std::nullopt;
    }

    /// Read one record of `causal_entry` from `c`: its keys in any order, a key the
    /// schema does not declare skipped whole; a key given twice or a field
    /// missing is `std::nullopt`.
    static std::optional<SCE::Generated::CausalEntry::CausalEntryPayload>
    decode_record_causal_entry(::SCE::Forge::SceCursor &c) {
        namespace cbor = ::SCE::Forge::Cbor;
        const auto count = cbor::read_map_len(c);
        if (!count) {
            return std::nullopt;
        }
        std::optional<uint64_t> held_hi;
        std::optional<uint64_t> held_lo;
        std::optional<uint64_t> held_seq;
        for (std::uint64_t i = 0; i < *count; ++i) {
            const auto key = cbor::read_text(c, std::nullopt);
            if (!key) {
                return std::nullopt;
            }
            if (*key == "hi") {
                // A key given twice is not one map entry.
                if (held_hi.has_value()) {
                    return std::nullopt;
                }
                held_hi = cbor::read_uint(c);
                if (!held_hi) {
                    return std::nullopt;
                }
            } else if (*key == "lo") {
                // A key given twice is not one map entry.
                if (held_lo.has_value()) {
                    return std::nullopt;
                }
                held_lo = cbor::read_uint(c);
                if (!held_lo) {
                    return std::nullopt;
                }
            } else if (*key == "seq") {
                // A key given twice is not one map entry.
                if (held_seq.has_value()) {
                    return std::nullopt;
                }
                held_seq = cbor::read_uint(c);
                if (!held_seq) {
                    return std::nullopt;
                }
            } else if (!cbor::skip(c)) {
                return std::nullopt;
            }
        }
        if (!held_hi.has_value()) {
            return std::nullopt;
        }
        if (!held_lo.has_value()) {
            return std::nullopt;
        }
        if (!held_seq.has_value()) {
            return std::nullopt;
        }
        SCE::Generated::CausalEntry::CausalEntryPayload record;
        record.hi = *held_hi;
        record.lo = *held_lo;
        record.seq = *held_seq;
        return record;
    }
};

/// The encode of `Envelope` over any source of it: the map head counting
/// the entries present, then each in ascending key order, every head in its
/// shortest form (RFC 8949 §4.2.1). It refuses nothing; what it returns is
/// the sink's own failure, and it throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError>
    encode_source(const S &src, ::SCE::Forge::SceSink &w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value,
                      "S presents every entry of Envelope: id, source, event_type, pattern, datacontenttype, data, "
                      "subject, correlation_id, reply_to, invoke_id, rpc_status, rpc_error_message, deadline_unix_ms, "
                      "sequence_no, routing_id, parallel_id, region_id, child_session_id, causal_epoch, causal_vector");
        namespace cbor = ::SCE::Forge::Cbor;
        const auto &_r_id = source::id(src);
        const ::SCE::Forge::Bytes<16, 16> _e_id = _r_id;
        const auto &_r_source = source::source(src);
        const ::SCE::Forge::Text<262144> _e_source = _r_source;
        const auto &_r_event_type = source::event_type(src);
        const ::SCE::Forge::Text<262144> _e_event_type = _r_event_type;
        const auto &_r_pattern = source::pattern(src);
        const SCE::Generated::PatternKind::PatternKind _e_pattern = _r_pattern;
        const auto &_r_datacontenttype = source::datacontenttype(src);
        const SCE::Generated::PayloadCodec::PayloadCodec _e_datacontenttype = _r_datacontenttype;
        const auto &_r_data = source::data(src);
        const ::SCE::Forge::Bytes<16777216> _e_data = _r_data;
        const auto &_r_subject = source::subject(src);
        const std::optional<::SCE::Forge::Text<262144>> _e_subject = _r_subject;
        const auto &_r_correlation_id = source::correlation_id(src);
        const std::optional<::SCE::Forge::Bytes<16, 16>> _e_correlation_id = _r_correlation_id;
        const auto &_r_reply_to = source::reply_to(src);
        const std::optional<::SCE::Forge::Text<262144>> _e_reply_to = _r_reply_to;
        const auto &_r_invoke_id = source::invoke_id(src);
        const std::optional<::SCE::Forge::Bytes<16, 16>> _e_invoke_id = _r_invoke_id;
        const auto &_r_rpc_status = source::rpc_status(src);
        const std::optional<SCE::Generated::RpcStatus::RpcStatus> _e_rpc_status = _r_rpc_status;
        const auto &_r_rpc_error_message = source::rpc_error_message(src);
        const std::optional<::SCE::Forge::Text<262144>> _e_rpc_error_message = _r_rpc_error_message;
        const auto &_r_deadline_unix_ms = source::deadline_unix_ms(src);
        const std::optional<uint64_t> _e_deadline_unix_ms = _r_deadline_unix_ms;
        const auto &_r_sequence_no = source::sequence_no(src);
        const std::optional<uint64_t> _e_sequence_no = _r_sequence_no;
        const auto &_r_routing_id = source::routing_id(src);
        const std::optional<::SCE::Forge::Bytes<16, 16>> _e_routing_id = _r_routing_id;
        const auto &_r_parallel_id = source::parallel_id(src);
        const std::optional<::SCE::Forge::Text<262144>> _e_parallel_id = _r_parallel_id;
        const auto &_r_region_id = source::region_id(src);
        const std::optional<::SCE::Forge::Text<262144>> _e_region_id = _r_region_id;
        const auto &_r_child_session_id = source::child_session_id(src);
        const std::optional<::SCE::Forge::Text<262144>> _e_child_session_id = _r_child_session_id;
        const auto &_r_causal_epoch = source::causal_epoch(src);
        const std::optional<uint64_t> _e_causal_epoch = _r_causal_epoch;
        const auto &_r_causal_vector = source::causal_vector(src);
        const std::optional<::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>> _e_causal_vector =
            _r_causal_vector;
        std::uint64_t _count = 6;
        if (_e_subject) {
            ++_count;
        }
        if (_e_correlation_id) {
            ++_count;
        }
        if (_e_reply_to) {
            ++_count;
        }
        if (_e_invoke_id) {
            ++_count;
        }
        if (_e_rpc_status) {
            ++_count;
        }
        if (_e_rpc_error_message) {
            ++_count;
        }
        if (_e_deadline_unix_ms) {
            ++_count;
        }
        if (_e_sequence_no) {
            ++_count;
        }
        if (_e_routing_id) {
            ++_count;
        }
        if (_e_parallel_id) {
            ++_count;
        }
        if (_e_region_id) {
            ++_count;
        }
        if (_e_child_session_id) {
            ++_count;
        }
        if (_e_causal_epoch) {
            ++_count;
        }
        if (_e_causal_vector) {
            ++_count;
        }
        if (auto _e = cbor::write_map_head(w, _count); _e) {
            return _e;
        }
        {
            const auto &v = _e_id;
            if (auto _e = cbor::write_uint(w, 0); _e) {
                return _e;
            }
            if (auto _e = cbor::write_bytes(w, v.inner()); _e) {
                return _e;
            }
        }
        {
            const auto &v = _e_source;
            if (auto _e = cbor::write_uint(w, 1); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        {
            const auto &v = _e_event_type;
            if (auto _e = cbor::write_uint(w, 2); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        {
            const auto &v = _e_pattern;
            if (auto _e = cbor::write_uint(w, 3); _e) {
                return _e;
            }
            if (auto _e =
                    cbor::write_uint(w, static_cast<std::uint64_t>(SCE::Generated::PatternKind::to_underlying(v)));
                _e) {
                return _e;
            }
        }
        {
            const auto &v = _e_datacontenttype;
            if (auto _e = cbor::write_uint(w, 4); _e) {
                return _e;
            }
            if (auto _e =
                    cbor::write_uint(w, static_cast<std::uint64_t>(SCE::Generated::PayloadCodec::to_underlying(v)));
                _e) {
                return _e;
            }
        }
        {
            const auto &v = _e_data;
            if (auto _e = cbor::write_uint(w, 5); _e) {
                return _e;
            }
            if (auto _e = cbor::write_bytes(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_subject) {
            const auto &v = *_e_subject;
            if (auto _e = cbor::write_uint(w, 6); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_correlation_id) {
            const auto &v = *_e_correlation_id;
            if (auto _e = cbor::write_uint(w, 7); _e) {
                return _e;
            }
            if (auto _e = cbor::write_bytes(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_reply_to) {
            const auto &v = *_e_reply_to;
            if (auto _e = cbor::write_uint(w, 8); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_invoke_id) {
            const auto &v = *_e_invoke_id;
            if (auto _e = cbor::write_uint(w, 9); _e) {
                return _e;
            }
            if (auto _e = cbor::write_bytes(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_rpc_status) {
            const auto &v = *_e_rpc_status;
            if (auto _e = cbor::write_uint(w, 10); _e) {
                return _e;
            }
            if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(SCE::Generated::RpcStatus::to_underlying(v)));
                _e) {
                return _e;
            }
        }
        if (_e_rpc_error_message) {
            const auto &v = *_e_rpc_error_message;
            if (auto _e = cbor::write_uint(w, 11); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_deadline_unix_ms) {
            const auto &v = *_e_deadline_unix_ms;
            if (auto _e = cbor::write_uint(w, 12); _e) {
                return _e;
            }
            if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(v)); _e) {
                return _e;
            }
        }
        if (_e_sequence_no) {
            const auto &v = *_e_sequence_no;
            if (auto _e = cbor::write_uint(w, 14); _e) {
                return _e;
            }
            if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(v)); _e) {
                return _e;
            }
        }
        if (_e_routing_id) {
            const auto &v = *_e_routing_id;
            if (auto _e = cbor::write_uint(w, 15); _e) {
                return _e;
            }
            if (auto _e = cbor::write_bytes(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_parallel_id) {
            const auto &v = *_e_parallel_id;
            if (auto _e = cbor::write_uint(w, 16); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_region_id) {
            const auto &v = *_e_region_id;
            if (auto _e = cbor::write_uint(w, 17); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_child_session_id) {
            const auto &v = *_e_child_session_id;
            if (auto _e = cbor::write_uint(w, 18); _e) {
                return _e;
            }
            if (auto _e = cbor::write_text(w, v.inner()); _e) {
                return _e;
            }
        }
        if (_e_causal_epoch) {
            const auto &v = *_e_causal_epoch;
            if (auto _e = cbor::write_uint(w, 19); _e) {
                return _e;
            }
            if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(v)); _e) {
                return _e;
            }
        }
        if (_e_causal_vector) {
            const auto &v = *_e_causal_vector;
            if (auto _e = cbor::write_uint(w, 20); _e) {
                return _e;
            }
            if (auto _e = cbor::write_array_head(w, static_cast<std::uint64_t>(v.size())); _e) {
                return _e;
            }
            for (const auto &_record : v) {
                if (auto _e = Records::encode_record_causal_entry(w, _record); _e) {
                    return _e;
                }
            }
        }
        return std::nullopt;
    }
};

/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and
/// skips one this codec does not declare; encode writes the entries present in
/// ascending key order, every head in its shortest form. A value made by
/// default holds each required entry at its type's default and each optional
/// one absent.
struct Envelope {
    Envelope() = default;

    /// The value of each entry it presents, in declaration order (ADR 0059 §5).
    Envelope(::SCE::Forge::Bytes<16, 16> id, ::SCE::Forge::Text<262144> source, ::SCE::Forge::Text<262144> event_type,
             SCE::Generated::PatternKind::PatternKind pattern,
             SCE::Generated::PayloadCodec::PayloadCodec datacontenttype, ::SCE::Forge::Bytes<16777216> data,
             std::optional<::SCE::Forge::Text<262144>> subject,
             std::optional<::SCE::Forge::Bytes<16, 16>> correlation_id,
             std::optional<::SCE::Forge::Text<262144>> reply_to, std::optional<::SCE::Forge::Bytes<16, 16>> invoke_id,
             std::optional<SCE::Generated::RpcStatus::RpcStatus> rpc_status,
             std::optional<::SCE::Forge::Text<262144>> rpc_error_message, std::optional<uint64_t> deadline_unix_ms,
             std::optional<uint64_t> sequence_no, std::optional<::SCE::Forge::Bytes<16, 16>> routing_id,
             std::optional<::SCE::Forge::Text<262144>> parallel_id, std::optional<::SCE::Forge::Text<262144>> region_id,
             std::optional<::SCE::Forge::Text<262144>> child_session_id, std::optional<uint64_t> causal_epoch,
             std::optional<::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>> causal_vector)
        : id_(id), source_(source), event_type_(event_type), pattern_(pattern), datacontenttype_(datacontenttype),
          data_(data), subject_(subject), correlation_id_(correlation_id), reply_to_(reply_to), invoke_id_(invoke_id),
          rpc_status_(rpc_status), rpc_error_message_(rpc_error_message), deadline_unix_ms_(deadline_unix_ms),
          sequence_no_(sequence_no), routing_id_(routing_id), parallel_id_(parallel_id), region_id_(region_id),
          child_session_id_(child_session_id), causal_epoch_(causal_epoch), causal_vector_(causal_vector) {}

    /// The entry with key 0.
    [[nodiscard]] ::SCE::Forge::Bytes<16, 16> id() const noexcept {
        return id_;
    }

    /// The entry with key 1.
    [[nodiscard]] ::SCE::Forge::Text<262144> source() const noexcept {
        return source_;
    }

    /// The entry with key 2.
    [[nodiscard]] ::SCE::Forge::Text<262144> event_type() const noexcept {
        return event_type_;
    }

    /// The entry with key 3.
    [[nodiscard]] constexpr SCE::Generated::PatternKind::PatternKind pattern() const noexcept {
        return pattern_;
    }

    /// The entry with key 4.
    [[nodiscard]] constexpr SCE::Generated::PayloadCodec::PayloadCodec datacontenttype() const noexcept {
        return datacontenttype_;
    }

    /// The entry with key 5.
    [[nodiscard]] ::SCE::Forge::Bytes<16777216> data() const noexcept {
        return data_;
    }

    /// The entry with key 6, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Text<262144>> subject() const noexcept {
        if (!subject_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Text<262144>(*subject_);
    }

    /// The entry with key 7, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Bytes<16, 16>> correlation_id() const noexcept {
        if (!correlation_id_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Bytes<16, 16>(*correlation_id_);
    }

    /// The entry with key 8, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Text<262144>> reply_to() const noexcept {
        if (!reply_to_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Text<262144>(*reply_to_);
    }

    /// The entry with key 9, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Bytes<16, 16>> invoke_id() const noexcept {
        if (!invoke_id_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Bytes<16, 16>(*invoke_id_);
    }

    /// The entry with key 10, where the map holds it.
    [[nodiscard]] constexpr std::optional<SCE::Generated::RpcStatus::RpcStatus> rpc_status() const noexcept {
        return rpc_status_;
    }

    /// The entry with key 11, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Text<262144>> rpc_error_message() const noexcept {
        if (!rpc_error_message_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Text<262144>(*rpc_error_message_);
    }

    /// The entry with key 12, where the map holds it.
    [[nodiscard]] constexpr std::optional<uint64_t> deadline_unix_ms() const noexcept {
        return deadline_unix_ms_;
    }

    /// The entry with key 14, where the map holds it.
    [[nodiscard]] constexpr std::optional<uint64_t> sequence_no() const noexcept {
        return sequence_no_;
    }

    /// The entry with key 15, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Bytes<16, 16>> routing_id() const noexcept {
        if (!routing_id_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Bytes<16, 16>(*routing_id_);
    }

    /// The entry with key 16, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Text<262144>> parallel_id() const noexcept {
        if (!parallel_id_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Text<262144>(*parallel_id_);
    }

    /// The entry with key 17, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Text<262144>> region_id() const noexcept {
        if (!region_id_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Text<262144>(*region_id_);
    }

    /// The entry with key 18, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::Text<262144>> child_session_id() const noexcept {
        if (!child_session_id_) {
            return std::nullopt;
        }
        return ::SCE::Forge::Text<262144>(*child_session_id_);
    }

    /// The entry with key 19, where the map holds it.
    [[nodiscard]] constexpr std::optional<uint64_t> causal_epoch() const noexcept {
        return causal_epoch_;
    }

    /// The entry with key 20, where the map holds it.
    [[nodiscard]] std::optional<::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>>
    causal_vector() const noexcept {
        if (!causal_vector_) {
            return std::nullopt;
        }
        return ::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>(*causal_vector_);
    }

    /// Decode one map from `cursor`. On success the cursor advances past it;
    /// `std::nullopt` on any refusal, with the cursor left where it was.
    static std::optional<Envelope> decode(::SCE::Forge::SceCursor &cursor) {
        ::SCE::Forge::SceCursor c = cursor;
        auto value = decode_at(c);
        if (value) {
            cursor = c;
        }
        return value;
    }

    /// Encode this map into `w`: the encode over any source, this value one of
    /// them. It refuses nothing; what it returns is the sink's own failure.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink &w) const noexcept {
        return Wire::encode_source(*this, w);
    }

    /// `encode` into a new vector.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> out;
        ::SCE::Forge::VectorSink sink(out);
        (void)encode(sink);
        return out;
    }

private:
    static std::optional<Envelope> decode_at(::SCE::Forge::SceCursor &c) {
        namespace cbor = ::SCE::Forge::Cbor;
        const auto count = cbor::read_map_len(c);
        if (!count) {
            return std::nullopt;
        }
        std::optional<std::vector<uint8_t>> id;
        std::optional<std::string> source;
        std::optional<std::string> event_type;
        std::optional<SCE::Generated::PatternKind::PatternKind> pattern;
        std::optional<SCE::Generated::PayloadCodec::PayloadCodec> datacontenttype;
        std::optional<std::vector<uint8_t>> data;
        std::optional<std::string> subject;
        std::optional<std::vector<uint8_t>> correlation_id;
        std::optional<std::string> reply_to;
        std::optional<std::vector<uint8_t>> invoke_id;
        std::optional<SCE::Generated::RpcStatus::RpcStatus> rpc_status;
        std::optional<std::string> rpc_error_message;
        std::optional<uint64_t> deadline_unix_ms;
        std::optional<uint64_t> sequence_no;
        std::optional<std::vector<uint8_t>> routing_id;
        std::optional<std::string> parallel_id;
        std::optional<std::string> region_id;
        std::optional<std::string> child_session_id;
        std::optional<uint64_t> causal_epoch;
        std::optional<std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>> causal_vector;
        for (std::uint64_t i = 0; i < *count; ++i) {
            const auto key = cbor::read_uint(c);
            if (!key) {
                return std::nullopt;
            }
            switch (*key) {
            case 0: {
                // A key given twice is not one map entry.
                if (id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                id = std::move(*v);
                break;
            }
            case 1: {
                // A key given twice is not one map entry.
                if (source.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                source = std::move(*v);
                break;
            }
            case 2: {
                // A key given twice is not one map entry.
                if (event_type.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                event_type = std::move(*v);
                break;
            }
            case 3: {
                // A key given twice is not one map entry.
                if (pattern.has_value()) {
                    return std::nullopt;
                }
                const auto raw =
                    cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<uint16_t>::max()));
                if (!raw) {
                    return std::nullopt;
                }
                auto v = SCE::Generated::PatternKind::from_underlying(static_cast<uint16_t>(*raw));
                if (!v) {
                    return std::nullopt;
                }
                pattern = std::move(*v);
                break;
            }
            case 4: {
                // A key given twice is not one map entry.
                if (datacontenttype.has_value()) {
                    return std::nullopt;
                }
                const auto raw =
                    cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<uint8_t>::max()));
                if (!raw) {
                    return std::nullopt;
                }
                auto v = SCE::Generated::PayloadCodec::from_underlying(static_cast<uint8_t>(*raw));
                if (!v) {
                    return std::nullopt;
                }
                datacontenttype = std::move(*v);
                break;
            }
            case 5: {
                // A key given twice is not one map entry.
                if (data.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes(c, 16777216);
                if (!v) {
                    return std::nullopt;
                }
                data = std::move(*v);
                break;
            }
            case 6: {
                // A key given twice is not one map entry.
                if (subject.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                subject = std::move(*v);
                break;
            }
            case 7: {
                // A key given twice is not one map entry.
                if (correlation_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                correlation_id = std::move(*v);
                break;
            }
            case 8: {
                // A key given twice is not one map entry.
                if (reply_to.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                reply_to = std::move(*v);
                break;
            }
            case 9: {
                // A key given twice is not one map entry.
                if (invoke_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                invoke_id = std::move(*v);
                break;
            }
            case 10: {
                // A key given twice is not one map entry.
                if (rpc_status.has_value()) {
                    return std::nullopt;
                }
                const auto raw =
                    cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<uint8_t>::max()));
                if (!raw) {
                    return std::nullopt;
                }
                auto v = SCE::Generated::RpcStatus::from_underlying(static_cast<uint8_t>(*raw));
                if (!v) {
                    return std::nullopt;
                }
                rpc_status = std::move(*v);
                break;
            }
            case 11: {
                // A key given twice is not one map entry.
                if (rpc_error_message.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                rpc_error_message = std::move(*v);
                break;
            }
            case 12: {
                // A key given twice is not one map entry.
                if (deadline_unix_ms.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_uint(c);
                if (!v) {
                    return std::nullopt;
                }
                deadline_unix_ms = std::move(*v);
                break;
            }
            case 14: {
                // A key given twice is not one map entry.
                if (sequence_no.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_uint(c);
                if (!v) {
                    return std::nullopt;
                }
                sequence_no = std::move(*v);
                break;
            }
            case 15: {
                // A key given twice is not one map entry.
                if (routing_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                routing_id = std::move(*v);
                break;
            }
            case 16: {
                // A key given twice is not one map entry.
                if (parallel_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                parallel_id = std::move(*v);
                break;
            }
            case 17: {
                // A key given twice is not one map entry.
                if (region_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                region_id = std::move(*v);
                break;
            }
            case 18: {
                // A key given twice is not one map entry.
                if (child_session_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                child_session_id = std::move(*v);
                break;
            }
            case 19: {
                // A key given twice is not one map entry.
                if (causal_epoch.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_uint(c);
                if (!v) {
                    return std::nullopt;
                }
                causal_epoch = std::move(*v);
                break;
            }
            case 20: {
                // A key given twice is not one map entry.
                if (causal_vector.has_value()) {
                    return std::nullopt;
                }
                const auto _len = cbor::read_array_len(c);
                if (!_len || *_len > 64) {
                    return std::nullopt;
                }
                std::optional<std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>> v{std::in_place};
                for (std::uint64_t _n = 0; _n < *_len; ++_n) {
                    auto _record = Records::decode_record_causal_entry(c);
                    if (!_record) {
                        return std::nullopt;
                    }
                    v->push_back(std::move(*_record));
                }
                if (!v) {
                    return std::nullopt;
                }
                causal_vector = std::move(*v);
                break;
            }
            default:
                if (!cbor::skip(c)) {
                    return std::nullopt;
                }
                break;
            }
        }
        if (!id.has_value()) {
            return std::nullopt;
        }
        if (!source.has_value()) {
            return std::nullopt;
        }
        if (!event_type.has_value()) {
            return std::nullopt;
        }
        if (!pattern.has_value()) {
            return std::nullopt;
        }
        if (!datacontenttype.has_value()) {
            return std::nullopt;
        }
        if (!data.has_value()) {
            return std::nullopt;
        }
        const auto _b_id = ::SCE::Forge::Bytes<16, 16>::make(*id);
        if (!_b_id) {
            return std::nullopt;
        }
        const auto _b_source = ::SCE::Forge::Text<262144>::make(*source);
        if (!_b_source) {
            return std::nullopt;
        }
        const auto _b_event_type = ::SCE::Forge::Text<262144>::make(*event_type);
        if (!_b_event_type) {
            return std::nullopt;
        }
        const auto _b_data = ::SCE::Forge::Bytes<16777216>::make(*data);
        if (!_b_data) {
            return std::nullopt;
        }
        std::optional<::SCE::Forge::Text<262144>> _b_subject;
        if (subject) {
            _b_subject = ::SCE::Forge::Text<262144>::make(*subject);
            if (!_b_subject) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Bytes<16, 16>> _b_correlation_id;
        if (correlation_id) {
            _b_correlation_id = ::SCE::Forge::Bytes<16, 16>::make(*correlation_id);
            if (!_b_correlation_id) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Text<262144>> _b_reply_to;
        if (reply_to) {
            _b_reply_to = ::SCE::Forge::Text<262144>::make(*reply_to);
            if (!_b_reply_to) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Bytes<16, 16>> _b_invoke_id;
        if (invoke_id) {
            _b_invoke_id = ::SCE::Forge::Bytes<16, 16>::make(*invoke_id);
            if (!_b_invoke_id) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Text<262144>> _b_rpc_error_message;
        if (rpc_error_message) {
            _b_rpc_error_message = ::SCE::Forge::Text<262144>::make(*rpc_error_message);
            if (!_b_rpc_error_message) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Bytes<16, 16>> _b_routing_id;
        if (routing_id) {
            _b_routing_id = ::SCE::Forge::Bytes<16, 16>::make(*routing_id);
            if (!_b_routing_id) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Text<262144>> _b_parallel_id;
        if (parallel_id) {
            _b_parallel_id = ::SCE::Forge::Text<262144>::make(*parallel_id);
            if (!_b_parallel_id) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Text<262144>> _b_region_id;
        if (region_id) {
            _b_region_id = ::SCE::Forge::Text<262144>::make(*region_id);
            if (!_b_region_id) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Text<262144>> _b_child_session_id;
        if (child_session_id) {
            _b_child_session_id = ::SCE::Forge::Text<262144>::make(*child_session_id);
            if (!_b_child_session_id) {
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>> _b_causal_vector;
        if (causal_vector) {
            _b_causal_vector =
                ::SCE::Forge::List<SCE::Generated::CausalEntry::CausalEntryPayload, 64>::make(*causal_vector);
            if (!_b_causal_vector) {
                return std::nullopt;
            }
        }
        return Envelope{
            /*id=*/*_b_id,
            /*source=*/*_b_source,
            /*event_type=*/*_b_event_type,
            /*pattern=*/*pattern,
            /*datacontenttype=*/*datacontenttype,
            /*data=*/*_b_data,
            /*subject=*/_b_subject,
            /*correlation_id=*/_b_correlation_id,
            /*reply_to=*/_b_reply_to,
            /*invoke_id=*/_b_invoke_id,
            /*rpc_status=*/rpc_status,
            /*rpc_error_message=*/_b_rpc_error_message,
            /*deadline_unix_ms=*/deadline_unix_ms,
            /*sequence_no=*/sequence_no,
            /*routing_id=*/_b_routing_id,
            /*parallel_id=*/_b_parallel_id,
            /*region_id=*/_b_region_id,
            /*child_session_id=*/_b_child_session_id,
            /*causal_epoch=*/causal_epoch,
            /*causal_vector=*/_b_causal_vector,
        };
    }

    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16, 16> id_{};
    ::SCE::Forge::Bounded<std::string, 262144> source_{};
    ::SCE::Forge::Bounded<std::string, 262144> event_type_{};
    SCE::Generated::PatternKind::PatternKind pattern_{SCE::Generated::PatternKind::PatternKind::FireForget};
    SCE::Generated::PayloadCodec::PayloadCodec datacontenttype_{SCE::Generated::PayloadCodec::PayloadCodec::None};
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16777216> data_{};
    std::optional<::SCE::Forge::Bounded<std::string, 262144>> subject_{};
    std::optional<::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16, 16>> correlation_id_{};
    std::optional<::SCE::Forge::Bounded<std::string, 262144>> reply_to_{};
    std::optional<::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16, 16>> invoke_id_{};
    std::optional<SCE::Generated::RpcStatus::RpcStatus> rpc_status_{};
    std::optional<::SCE::Forge::Bounded<std::string, 262144>> rpc_error_message_{};
    std::optional<uint64_t> deadline_unix_ms_{};
    std::optional<uint64_t> sequence_no_{};
    std::optional<::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16, 16>> routing_id_{};
    std::optional<::SCE::Forge::Bounded<std::string, 262144>> parallel_id_{};
    std::optional<::SCE::Forge::Bounded<std::string, 262144>> region_id_{};
    std::optional<::SCE::Forge::Bounded<std::string, 262144>> child_session_id_{};
    std::optional<uint64_t> causal_epoch_{};
    std::optional<::SCE::Forge::Bounded<std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>, 64>>
        causal_vector_{};
};

}  // namespace SCE::Generated::Envelope

#endif  // SCE_FORGE_ENVELOPE_H
