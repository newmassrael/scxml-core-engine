// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the C++ side of an SCE deployment reports of what it does, through a port (docs/adr/0109,
// D16): the reference host of the standard host features (host/StandardHost.h and its features)
// and the client of the `https` binding (mesh/transports/HttpsClient.h) report to a Metrics their
// embedding gives them, and one that gives none reports nothing. SCE links no metrics library: the
// embedding implements Metrics over the one it uses.
//
// Fixed names, closed label sets. Every series is a Counter or a Histogram built only by the named
// functions below, from the enums of this header, so a caller cannot hand a label a text of its
// own: no series carries a client's name, a session's key or anything a client chose. The names,
// their labels and their values are the registry `sce-metrics-1` of docs/SCE_SERVER_DEPLOY.md
// section 10, which the Rust crate `sce-rust-metrics` holds too, value for value; this header
// holds the families the C++ side reports: `sce_client_exchanges`, `sce_host_operation_failures`,
// `sce_host_operation_duration_seconds` and `sce_host_audit_appends`. It reports no level, so it
// has no gauge.

#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <mutex>
#include <optional>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Metrics {

/// The registry of names this header follows (SCE_WIRE_CONTRACTS.md, "Server metrics").
inline constexpr const char *kRegistry = "sce-metrics-1";

/// What an exchange a client made came to: the status its server answered (each status an SCE
/// `https` server answers), `other` for a status no SCE server answers (an intermediary's), and
/// `none` when no response came. The label set of `sce_client_exchanges` alone: the server's
/// `sce_server_exchanges` holds only the statuses it answers, which the C++ side does not report.
enum class ExchangeOutcome : std::uint8_t {
    Ok,
    BadRequest,
    Unauthorized,
    NotFound,
    MethodNotAllowed,
    RequestTimeout,
    PayloadTooLarge,
    UnsupportedMediaType,
    TooManyRequests,
    ServiceUnavailable,
    Other,
    NoResponse,
};

/// Every exchange outcome, in the order of the enum.
inline constexpr std::array<ExchangeOutcome, 12> kExchangeOutcomes = {
    ExchangeOutcome::Ok,
    ExchangeOutcome::BadRequest,
    ExchangeOutcome::Unauthorized,
    ExchangeOutcome::NotFound,
    ExchangeOutcome::MethodNotAllowed,
    ExchangeOutcome::RequestTimeout,
    ExchangeOutcome::PayloadTooLarge,
    ExchangeOutcome::UnsupportedMediaType,
    ExchangeOutcome::TooManyRequests,
    ExchangeOutcome::ServiceUnavailable,
    ExchangeOutcome::Other,
    ExchangeOutcome::NoResponse,
};

/// The label value `outcome` is.
constexpr const char *value(ExchangeOutcome outcome) {
    switch (outcome) {
    case ExchangeOutcome::Ok:
        return "200";
    case ExchangeOutcome::BadRequest:
        return "400";
    case ExchangeOutcome::Unauthorized:
        return "401";
    case ExchangeOutcome::NotFound:
        return "404";
    case ExchangeOutcome::MethodNotAllowed:
        return "405";
    case ExchangeOutcome::RequestTimeout:
        return "408";
    case ExchangeOutcome::PayloadTooLarge:
        return "413";
    case ExchangeOutcome::UnsupportedMediaType:
        return "415";
    case ExchangeOutcome::TooManyRequests:
        return "429";
    case ExchangeOutcome::ServiceUnavailable:
        return "503";
    case ExchangeOutcome::Other:
        return "other";
    case ExchangeOutcome::NoResponse:
        return "none";
    }
    return "other";
}

/// The outcome of a response of status `code`: `other` for a code an SCE server never answers.
constexpr ExchangeOutcome exchangeOutcomeOf(std::uint16_t code) {
    switch (code) {
    case 200:
        return ExchangeOutcome::Ok;
    case 400:
        return ExchangeOutcome::BadRequest;
    case 401:
        return ExchangeOutcome::Unauthorized;
    case 404:
        return ExchangeOutcome::NotFound;
    case 405:
        return ExchangeOutcome::MethodNotAllowed;
    case 408:
        return ExchangeOutcome::RequestTimeout;
    case 413:
        return ExchangeOutcome::PayloadTooLarge;
    case 415:
        return ExchangeOutcome::UnsupportedMediaType;
    case 429:
        return ExchangeOutcome::TooManyRequests;
    case 503:
        return ExchangeOutcome::ServiceUnavailable;
    default:
        return ExchangeOutcome::Other;
    }
}

/// The type of a standard host operation (docs/SCE_HOST_FEATURES.md), each by its `type`.
enum class HostOperation : std::uint8_t {
    ClockNow,
    ClockZone,
    KvGet,
    KvPut,
    KvPutIf,
    KvDelete,
    KvNextKey,
    NotifyPermission,
    NotifyRequestPermission,
    NotifyPost,
    NotifyCancel,
    HttpFetch,
    ModelGenerate,
    CoseSign1,
    CoseVerify1,
    CoseEncrypt0,
    CoseDecrypt0,
    KeyDerive,
    RandomBytes,
    Mac,
    MacVerify,
    KeyGenerate,
    KeyPublic,
    KeyRetire,
    CredentialIssue,
    CredentialRevoke,
    AuditAppend,
};

/// The `type` of every standard host operation, in the order of HostOperation.
inline constexpr std::array<const char *, 27> kHostOperationTypes = {
    "sce-host-clock-now",
    "sce-host-clock-zone",
    "sce-host-kv-get",
    "sce-host-kv-put",
    "sce-host-kv-put-if",
    "sce-host-kv-delete",
    "sce-host-kv-next-key",
    "sce-host-notify-permission",
    "sce-host-notify-request-permission",
    "sce-host-notify-post",
    "sce-host-notify-cancel",
    "sce-host-http-fetch",
    "sce-host-model-generate",
    "sce-host-cose-sign1",
    "sce-host-cose-verify1",
    "sce-host-cose-encrypt0",
    "sce-host-cose-decrypt0",
    "sce-host-key-derive",
    "sce-host-random-bytes",
    "sce-host-mac",
    "sce-host-mac-verify",
    "sce-host-key-generate",
    "sce-host-key-public",
    "sce-host-key-retire",
    "sce-host-credential-issue",
    "sce-host-credential-revoke",
    "sce-host-audit-append",
};

/// The label value `operation` is: its `type`.
constexpr const char *value(HostOperation operation) {
    return kHostOperationTypes[static_cast<std::size_t>(operation)];
}

/// The operation whose `type` is `type`; none for a type that is no standard host operation.
inline std::optional<HostOperation> hostOperationOf(std::string_view type) {
    for (std::size_t i = 0; i < kHostOperationTypes.size(); ++i) {
        if (type == kHostOperationTypes[i]) {
            return static_cast<HostOperation>(i);
        }
    }
    return std::nullopt;
}

/// The code a standard host operation failed with: the union of every operation's closed set
/// (docs/SCE_HOST_FEATURES.md), `invalid_argument` for a start the host refused, and `other` for
/// a code no operation names (one an embedding injected).
enum class HostFailure : std::uint8_t {
    Unavailable,
    QuotaExceeded,
    Network,
    ResponseTooLarge,
    InvalidBody,
    DestinationRefused,
    Busy,
    ContextExceeded,
    AnswerTooLarge,
    Declined,
    UnknownKey,
    KeyRetired,
    UnknownStream,
    InvalidArgument,
    Other,
};

/// The code of every HostFailure, in the order of the enum.
inline constexpr std::array<const char *, 15> kHostFailureCodes = {
    "unavailable",         "quota_exceeded", "network",          "response_too_large", "invalid_body",
    "destination_refused", "busy",           "context_exceeded", "answer_too_large",   "declined",
    "unknown_key",         "key_retired",    "unknown_stream",   "invalid_argument",   "other",
};

/// The label value `failure` is: its code.
constexpr const char *value(HostFailure failure) {
    return kHostFailureCodes[static_cast<std::size_t>(failure)];
}

/// The value of the failure code `code`: `other` for a code no operation names.
inline HostFailure hostFailureOf(std::string_view code) {
    for (std::size_t i = 0; i + 1 < kHostFailureCodes.size(); ++i) {
        if (code == kHostFailureCodes[i]) {
            return static_cast<HostFailure>(i);
        }
    }
    return HostFailure::Other;
}

/// One label of a series: its fixed name and its value, each a text of this header.
struct Label {
    const char *name = "";
    const char *value = "";
};

/// The labels of one series, at most two, in the order the family names them.
struct Labels {
    std::array<Label, 2> pairs{};
    std::size_t size = 0;
};

/// A counter series: a family of the registry with its label values. Built only by the functions
/// below, so its labels are values of the closed sets above.
class Counter {
public:
    /// `sce_client_exchanges`: an exchange an `https` client made, by the status its server
    /// answered, `none` when no response came.
    static constexpr Counter clientExchanges(ExchangeOutcome outcome) {
        return Counter(Family::ClientExchanges, static_cast<std::uint8_t>(outcome), 0);
    }

    /// `sce_host_operation_failures`: a start of `operation` answered with the failure `code`, or
    /// refused (`invalid_argument`).
    static constexpr Counter hostOperationFailures(HostOperation operation, HostFailure code) {
        return Counter(Family::HostOperationFailures, static_cast<std::uint8_t>(operation),
                       static_cast<std::uint8_t>(code));
    }

    /// `sce_host_audit_appends`: an entry appended to the audit log.
    static constexpr Counter auditAppends() {
        return Counter(Family::AuditAppends, 0, 0);
    }

    /// The family's name, which the OpenMetrics exposition samples with `_total`.
    constexpr const char *family() const {
        switch (family_) {
        case Family::ClientExchanges:
            return "sce_client_exchanges";
        case Family::HostOperationFailures:
            return "sce_host_operation_failures";
        case Family::AuditAppends:
            return "sce_host_audit_appends";
        }
        return "";
    }

    /// Its labels, in the order its family names them.
    Labels labels() const {
        Labels held;
        switch (family_) {
        case Family::ClientExchanges:
            held.pairs[0] = {"status", value(static_cast<ExchangeOutcome>(first_))};
            held.size = 1;
            break;
        case Family::HostOperationFailures:
            held.pairs[0] = {"type", value(static_cast<HostOperation>(first_))};
            held.pairs[1] = {"code", value(static_cast<HostFailure>(second_))};
            held.size = 2;
            break;
        case Family::AuditAppends:
            break;
        }
        return held;
    }

    constexpr bool operator==(const Counter &other) const {
        return family_ == other.family_ && first_ == other.first_ && second_ == other.second_;
    }

    constexpr bool operator!=(const Counter &other) const {
        return !(*this == other);
    }

private:
    enum class Family : std::uint8_t { ClientExchanges, HostOperationFailures, AuditAppends };

    constexpr Counter(Family family, std::uint8_t first, std::uint8_t second)
        : family_(family), first_(first), second_(second) {}

    Family family_;
    std::uint8_t first_;
    std::uint8_t second_;
};

/// A histogram series: a family of the registry with its label values, built only by the
/// functions below.
class Histogram {
public:
    /// `sce_host_operation_duration_seconds`: how long the reference host took to answer a start
    /// of `operation`, in seconds; a start it holds is not observed.
    static constexpr Histogram hostOperationDuration(HostOperation operation) {
        return Histogram(operation);
    }

    /// The family's name.
    constexpr const char *family() const {
        return "sce_host_operation_duration_seconds";
    }

    /// Its labels: the operation's type.
    Labels labels() const {
        Labels held;
        held.pairs[0] = {"type", value(operation_)};
        held.size = 1;
        return held;
    }

    constexpr bool operator==(const Histogram &other) const {
        return operation_ == other.operation_;
    }

    constexpr bool operator!=(const Histogram &other) const {
        return !(*this == other);
    }

private:
    explicit constexpr Histogram(HostOperation operation) : operation_(operation) {}

    HostOperation operation_;
};

/// What a component reports to: the port its embedding implements. Called from any thread, often
/// from several at once, on the paths that serve the machine: an implementation answers at once
/// and never blocks on a collector.
class Metrics {
public:
    virtual ~Metrics() = default;
    /// `counter` moved by `by`.
    virtual void count(const Counter &counter, std::uint64_t by) = 0;
    /// One observation of `histogram`: a duration in seconds.
    virtual void observe(const Histogram &histogram, double value) = 0;
};

/// The implementation that reports nothing.
class NoMetrics final : public Metrics {
public:
    void count(const Counter &, std::uint64_t) override {}

    void observe(const Histogram &, double) override {}
};

/// An implementation that keeps every call in the order it was made: what a test of a component
/// reads to assert that an event moved the series that names it, and nothing else.
class Recording final : public Metrics {
public:
    void count(const Counter &counter, std::uint64_t by) override {
        std::lock_guard<std::mutex> lock(mutex_);
        counted_.emplace_back(counter, by);
    }

    void observe(const Histogram &histogram, double value) override {
        std::lock_guard<std::mutex> lock(mutex_);
        observed_.emplace_back(histogram, value);
    }

    /// How far `counter` was moved.
    std::uint64_t counted(const Counter &counter) const {
        std::lock_guard<std::mutex> lock(mutex_);
        std::uint64_t total = 0;
        for (const auto &[moved, by] : counted_) {
            total += moved == counter ? by : 0;
        }
        return total;
    }

    /// How far every series of the family `family` was moved together.
    std::uint64_t countedIn(std::string_view family) const {
        std::lock_guard<std::mutex> lock(mutex_);
        std::uint64_t total = 0;
        for (const auto &[moved, by] : counted_) {
            total += family == moved.family() ? by : 0;
        }
        return total;
    }

    /// Every observation of `histogram`, in order.
    std::vector<double> observed(const Histogram &histogram) const {
        std::lock_guard<std::mutex> lock(mutex_);
        std::vector<double> values;
        for (const auto &[seen, observation] : observed_) {
            if (seen == histogram) {
                values.push_back(observation);
            }
        }
        return values;
    }

    /// How many observations of any histogram were made.
    std::size_t observations() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return observed_.size();
    }

private:
    mutable std::mutex mutex_;
    std::vector<std::pair<Counter, std::uint64_t>> counted_;
    std::vector<std::pair<Histogram, double>> observed_;
};

}  // namespace SCE::Metrics
