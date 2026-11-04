// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file StandardHost.h
 * @brief The reference host of the standard host features (docs/SCE_HOST_FEATURES.md):
 *        clock, key-value store, notifications and HTTP, as the operations a generated
 *        machine's host-run invokes ask for.
 *
 * A ReferenceHost is the whole of the four features' semantics as one object. It is told
 * the start of an operation (its type and the request's text parameters) and answers with
 * an Outcome: a completion, a refusal, or nothing yet. It knows no engine, so the same
 * object serves a generated machine through serve(), a test, or another embedding. Time
 * comes from a ClockSource and the network from a Transport, both the embedding's to
 * supply.
 */

#include <algorithm>
#include <array>
#include <cctype>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <ctime>
#include <deque>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "core/HostProcessor.h"
#include "core/LogMacros.h"
#include "core/TypedHostInvokeRequest.h"

namespace SCE::StandardHost {

/// Every operation type of the standard host features, in the order the specification lists them.
inline constexpr std::array<const char *, 12> TYPES = {
    "sce-host-clock-now",
    "sce-host-clock-zone",
    "sce-host-kv-get",
    "sce-host-kv-put",
    "sce-host-kv-delete",
    "sce-host-kv-next-key",
    "sce-host-notify-permission",
    "sce-host-notify-request-permission",
    "sce-host-notify-post",
    "sce-host-notify-cancel",
    "sce-host-http-fetch",
    "sce-host-model-generate",
};

/// The request of an operation as the engine hands it to a host: each `<param>` by name, as text.
/// A 64-bit integer is its decimal digits.
using Params = std::map<std::string, std::string>;

/// What a host answers a start with.
struct Outcome {
    enum class Kind {
        /// A completion: `data` is the result record, as the JSON a completion carries. A failure is
        /// such a record whose `error` is a code of the operation's closed set.
        Done,
        /// The host would not start the operation; `data` is the reason, as the JSON `error.execution` carries.
        Refuse,
        /// The host has started the operation and answers later, or never.
        Hold,
    };
    Kind kind = Kind::Hold;
    std::string data;
};

/// A start the host was given: the operation, the invocation's id and the request's parameters.
struct Start {
    std::string operation;
    std::string id;
    Params params;
};

/// Both readings of one instant.
struct Reading {
    /// Milliseconds since 1970-01-01T00:00:00Z, POSIX time.
    int64_t utcMs = 0;
    /// Milliseconds since an origin fixed for the life of the process.
    int64_t monotonicMs = 0;
};

/// The time zone a host reads local time in.
struct Zone {
    /// An IANA identifier, or empty when only an offset is known.
    std::string id;
    /// Seconds east of UTC in force now.
    int32_t utcOffsetS = 0;
};

/// Where a host's time comes from. A source that cannot be read answers nothing, which the host
/// reports as `unavailable`.
class ClockSource {
public:
    virtual ~ClockSource() = default;
    virtual std::optional<Reading> read() = 0;
    virtual std::optional<Zone> zone() = 0;
};

/// A clock a scenario sets: it reads what it was told. An empty field is a clock that cannot be read.
class ScriptedClock : public ClockSource {
public:
    std::optional<Reading> reading = Reading{};
    std::optional<Zone> zoneNow = Zone{};

    std::optional<Reading> read() override {
        return reading;
    }

    std::optional<Zone> zone() override {
        return zoneNow;
    }
};

/// The operating system's clock.
class SystemClock : public ClockSource {
public:
    std::optional<Reading> read() override {
        const auto utc =
            std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch());
        const auto mono =
            std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - origin_);
        return Reading{utc.count(), mono.count()};
    }

    /// The offset in force now, and the identifier the TZ variable names when it names a zone, which the
    /// C library has no other way to report.
    std::optional<Zone> zone() override {
        const std::time_t now = std::time(nullptr);
        std::tm local{};
        if (localtime_r(&now, &local) == nullptr) {
            return std::nullopt;
        }
        std::string id;
        if (const char *tz = std::getenv("TZ")) {
            id = tz;
            if (!id.empty() && id.front() == ':') {
                id.erase(0, 1);
            }
            if (id.find('/') == std::string::npos && id != "UTC") {
                id.clear();
            }
        }
        return Zone{id, static_cast<int32_t>(local.tm_gmtoff)};
    }

private:
    std::chrono::steady_clock::time_point origin_ = std::chrono::steady_clock::now();
};

/// One header field: its name and its value.
struct HttpHeader {
    std::string name;
    std::string value;
};

/// What the host hands its transport: the request, its header lines already split into name and value.
struct HttpRequest {
    std::string method;
    std::string url;
    std::vector<HttpHeader> headers;
    std::string body;
};

/// What a transport answers: the response as it came.
struct HttpResponse {
    int status = 0;
    std::vector<HttpHeader> headers;
    std::string body;
};

/// A transport's answer to one exchange.
struct Exchange {
    enum class Kind {
        /// The response as it came.
        Response,
        /// No response was received: name resolution, connection, TLS, or the connection ended.
        Failure,
        /// The transport has not answered; the host holds the exchange until it is cancelled.
        Hold,
    };
    Kind kind = Kind::Failure;
    HttpResponse response;
};

/// Carries one exchange.
class Transport {
public:
    virtual ~Transport() = default;
    virtual Exchange exchange(const HttpRequest &request) = 0;
};

// ── Generated text (docs/SCE_HOST_FEATURES.md, section 6; docs/adr/0087, section 4) ──
//
// The host judges the request (an empty prompt, `max_tokens` zero, a temperature past 2000
// thousandths, an answer schema the machine's table does not hold), then asks its ModelProvider for
// a Generation, and never cuts the answer: one past the bound of `text` fails `answer_too_large`. No
// provider here reaches a model. ScriptedProvider gives the answers a scenario wrote;
// ExhaustiveProvider gives the outcome class a test names at each call (docs/adr/0087, section 9.4).

/// The operation that generates a text.
inline constexpr const char *MODEL_GENERATE = "sce-host-model-generate";
/// The bound of a result's `text`, in bytes (`model_answer` of the result record).
inline constexpr size_t ANSWER_MAX_BYTES = 4096;
/// The highest sampling temperature a request may ask for, in thousandths.
inline constexpr uint64_t TEMPERATURE_MAX_MILLI = 2000;

/// A generation's request, as a provider reads it; `answerSchema` is null for a free text.
struct GenerateRequest {
    std::string prompt;
    const SCE::AnswerSchema *answerSchema = nullptr;
    uint32_t maxTokens = 0;
    uint32_t temperatureMilli = 0;
};

/// What a provider made of one generation.
struct Generation {
    enum class Kind {
        /// An answer: the completion's fields but `error`.
        Answer,
        /// A failure, by its `code` of the operation's closed set.
        Failure,
        /// The host declines to start it (`invalid_argument`).
        Refusal,
        /// No answer yet, or ever: a deadline or a cancel ends the call.
        Hold,
    };
    Kind kind = Kind::Hold;
    std::string text;
    std::string finish;
    bool constrained = false;
    std::string model;
    uint32_t promptTokens = 0;
    uint32_t answerTokens = 0;
    std::string code;
};

/// Where a host's generations come from: a model on the device, a remote one, or a script.
class ModelProvider {
public:
    virtual ~ModelProvider() = default;
    virtual Generation generate(const GenerateRequest &request) = 0;
};

/// A text's tokens as the reference providers count them: its bytes.
inline uint32_t tokens(const std::string &text) {
    return static_cast<uint32_t>(text.size());
}

/// One answer a scripted provider gives; `finish` is `stop` or `length`.
struct ScriptedAnswer {
    std::string text;
    std::string finish = "stop";
};

/// A provider that gives the answers it was told, one a generation, in order. It is `constrained` only
/// when the request names an answer schema; `context` is the most tokens a prompt and `max_tokens` may
/// take together, 0 for none; `hold` holds every generation unanswered.
class ScriptedProvider : public ModelProvider {
public:
    std::deque<ScriptedAnswer> answers;
    bool constrained = false;
    std::string name = "scripted";
    uint64_t context = 0;
    bool hold = false;

    Generation generate(const GenerateRequest &request) override {
        Generation out;
        if (hold) {
            SCE_LOG_INFO("scripted provider: holds the generation");
            return out;
        }
        const uint32_t promptTokens = tokens(request.prompt);
        if (context != 0 && uint64_t{promptTokens} + request.maxTokens > context) {
            SCE_LOG_INFO("scripted provider: {} prompt tokens and {} more pass a context of {}", promptTokens,
                         request.maxTokens, context);
            out.kind = Generation::Kind::Failure;
            out.code = "context_exceeded";
            return out;
        }
        if (answers.empty()) {
            SCE_LOG_INFO("scripted provider: no answer left to give");
            out.kind = Generation::Kind::Failure;
            out.code = "unavailable";
            return out;
        }
        const ScriptedAnswer answer = answers.front();
        answers.pop_front();
        SCE_LOG_DEBUG("scripted provider: answers {} bytes, finish {}", answer.text.size(), answer.finish);
        out.kind = Generation::Kind::Answer;
        out.text = answer.text;
        out.finish = answer.finish;
        out.constrained = constrained && request.answerSchema != nullptr;
        out.model = name;
        out.promptTokens = promptTokens;
        out.answerTokens = tokens(answer.text);
        return out;
    }
};

/// One outcome class of a generation (docs/adr/0087, section 9.4), as the table every lane reads names it
/// (tests/host_features/model_outcome_classes.json): an answer with `text` and `finish`, a failure with
/// `code`, a refusal, or a hold the test ends by a deadline or a cancel.
struct OutcomeClass {
    Generation::Kind kind = Generation::Kind::Hold;
    std::string text;
    std::string finish;
    std::string code;
};

/// A provider that stands for a model that may give any outcome at any call: each generation takes the
/// next class of the sequence the test named.
class ExhaustiveProvider : public ModelProvider {
public:
    explicit ExhaustiveProvider(const std::vector<OutcomeClass> &sequence)
        : sequence_(sequence.begin(), sequence.end()) {}

    Generation generate(const GenerateRequest &request) override {
        Generation out;
        if (sequence_.empty()) {
            SCE_LOG_WARN("exhaustive provider: asked past the sequence the test named");
            out.kind = Generation::Kind::Failure;
            out.code = "unavailable";
            return out;
        }
        const OutcomeClass next = sequence_.front();
        sequence_.pop_front();
        SCE_LOG_DEBUG("exhaustive provider: answers class {}", static_cast<int>(next.kind));
        out.kind = next.kind;
        if (next.kind == Generation::Kind::Answer) {
            out.text = next.text;
            out.finish = next.finish;
            out.model = "exhaustive";
            out.promptTokens = tokens(request.prompt);
            out.answerTokens = tokens(next.text);
        } else if (next.kind == Generation::Kind::Failure) {
            out.code = next.code;
        }
        return out;
    }

private:
    std::deque<OutcomeClass> sequence_;
};

namespace detail {

/// A JSON string: quotes, backslashes and control characters escaped, the rest as it is.
inline std::string jsonString(const std::string &text) {
    std::string out = "\"";
    for (const char c : text) {
        const auto u = static_cast<unsigned char>(c);
        switch (c) {
        case '"':
            out += "\\\"";
            break;
        case '\\':
            out += "\\\\";
            break;
        case '\n':
            out += "\\n";
            break;
        case '\r':
            out += "\\r";
            break;
        case '\t':
            out += "\\t";
            break;
        default:
            if (u < 0x20) {
                static const char *hex = "0123456789abcdef";
                out += "\\u00";
                out += hex[u >> 4];
                out += hex[u & 0xf];
            } else {
                out += c;
            }
        }
    }
    return out + '"';
}

/// A field of a result record: its name, and its value already written as JSON.
using Field = std::pair<std::string, std::string>;

inline Field text(const std::string &name, const std::string &value) {
    return {name, jsonString(value)};
}

inline Field boolean(const std::string &name, bool value) {
    return {name, value ? "true" : "false"};
}

inline Field number(const std::string &name, int64_t value) {
    return {name, std::to_string(value)};
}

inline std::string object(const std::vector<Field> &fields) {
    std::string out = "{";
    for (size_t i = 0; i < fields.size(); ++i) {
        out += (i ? "," : "") + jsonString(fields[i].first) + ":" + fields[i].second;
    }
    return out + "}";
}

/// A result record as the JSON a completion carries, its `error` empty. A 64-bit integer is written as
/// the text of its digits, as a payload's reader takes it.
inline Outcome record(std::vector<Field> fields) {
    fields.insert(fields.begin(), text("error", ""));
    return {Outcome::Kind::Done, object(fields)};
}

/// The failure `code` of `operation`: its result record with the fields at their zero and `error` the
/// code, so a machine reads the failure as a typed field.
inline Outcome failure(const std::string &operation, const std::string &code) {
    std::vector<Field> zero;
    if (operation == "sce-host-clock-now") {
        zero = {text("utc_ms", "0"), text("monotonic_ms", "0")};
    } else if (operation == "sce-host-clock-zone") {
        zero = {text("zone_id", ""), number("utc_offset_s", 0)};
    } else if (operation == "sce-host-kv-get") {
        zero = {boolean("found", false), text("value", "")};
    } else if (operation == "sce-host-kv-put") {
        zero = {boolean("replaced", false)};
    } else if (operation == "sce-host-kv-delete" || operation == "sce-host-notify-cancel") {
        zero = {boolean("existed", false)};
    } else if (operation == "sce-host-kv-next-key") {
        zero = {boolean("found", false), text("key", "")};
    } else if (operation == "sce-host-notify-permission" || operation == "sce-host-notify-request-permission") {
        zero = {text("state", "")};
    } else if (operation == "sce-host-notify-post") {
        zero = {text("status", "")};
    } else if (operation == MODEL_GENERATE) {
        zero = {text("text", ""),           boolean("constrained", false), text("model", ""),
                number("prompt_tokens", 0), number("answer_tokens", 0),    text("finish", "")};
    } else {
        zero = {number("status", 0), text("headers", ""), text("body", "")};
    }
    zero.insert(zero.begin(), text("error", code));
    return {Outcome::Kind::Done, object(zero)};
}

/// Whether `bytes` is well-formed UTF-8: no overlong form, no surrogate, nothing past U+10FFFF.
inline bool validUtf8(const std::string &bytes) {
    size_t i = 0;
    const size_t n = bytes.size();
    auto at = [&](size_t k) { return static_cast<unsigned char>(bytes[k]); };
    while (i < n) {
        const unsigned char b = at(i);
        size_t extra = 0;
        uint32_t code = 0;
        if (b < 0x80) {
            ++i;
            continue;
        }
        if (b >= 0xC2 && b <= 0xDF) {
            extra = 1;
            code = b & 0x1F;
        } else if (b >= 0xE0 && b <= 0xEF) {
            extra = 2;
            code = b & 0x0F;
        } else if (b >= 0xF0 && b <= 0xF4) {
            extra = 3;
            code = b & 0x07;
        } else {
            return false;
        }
        if (i + extra >= n) {
            return false;
        }
        for (size_t k = 1; k <= extra; ++k) {
            if ((at(i + k) & 0xC0) != 0x80) {
                return false;
            }
            code = (code << 6) | (at(i + k) & 0x3F);
        }
        if ((extra == 2 && code < 0x800) || (extra == 3 && code < 0x10000) || code > 0x10FFFF ||
            (code >= 0xD800 && code <= 0xDFFF)) {
            return false;
        }
        i += extra + 1;
    }
    return true;
}

inline bool isToken(const std::string &name) {
    if (name.empty()) {
        return false;
    }
    return std::all_of(name.begin(), name.end(), [](char c) {
        return std::isalnum(static_cast<unsigned char>(c)) != 0 ||
               std::string_view("!#$%&'*+-.^_`|~").find(c) != std::string_view::npos;
    });
}

inline std::string trimBlanks(const std::string &s) {
    const auto first = s.find_first_not_of(" \t");
    if (first == std::string::npos) {
        return "";
    }
    return s.substr(first, s.find_last_not_of(" \t") - first + 1);
}

/// The header lines of a request, split into name and value, or nothing for a line that is not one.
/// Empty lines are ignored, and the value is without the spaces and tabs around it.
inline std::optional<std::vector<HttpHeader>> headerLines(const std::string &text) {
    std::vector<HttpHeader> headers;
    size_t start = 0;
    while (start <= text.size()) {
        size_t end = text.find('\n', start);
        if (end == std::string::npos) {
            end = text.size();
        }
        std::string line = text.substr(start, end - start);
        start = end + 1;
        if (!line.empty() && line.back() == '\r') {
            line.pop_back();
        }
        if (line.empty()) {
            continue;
        }
        const auto colon = line.find(':');
        if (colon == std::string::npos || !isToken(line.substr(0, colon))) {
            return std::nullopt;
        }
        headers.push_back({line.substr(0, colon), trimBlanks(line.substr(colon + 1))});
    }
    return headers;
}

/// An absolute `http` or `https` URL with a host, and no ASCII space or control character (U+0000 to U+0020
/// and U+007F) in it.
inline bool isHttpUrl(const std::string &url) {
    size_t skip = 0;
    if (url.rfind("http://", 0) == 0) {
        skip = 7;
    } else if (url.rfind("https://", 0) == 0) {
        skip = 8;
    } else {
        return false;
    }
    const std::string rest = url.substr(skip);
    const auto end = rest.find_first_of("/?#");
    const std::string authority = end == std::string::npos ? rest : rest.substr(0, end);
    if (authority.empty() || authority.front() == ':') {
        return false;
    }
    return std::none_of(url.begin(), url.end(), [](char c) {
        const auto u = static_cast<unsigned char>(c);
        return u <= 0x20 || u == 0x7F;
    });
}

}  // namespace detail

/// The standard host features' semantics, over a clock and a transport.
class ReferenceHost {
public:
    ReferenceHost(ClockSource &clock, Transport &transport) : clock_(clock), transport_(transport) {}

    /// Inject failures: each operation type named fails with the code it maps to, once the request has
    /// been accepted. An empty map ends them.
    void setFaults(std::map<std::string, std::string> faults) {
        faults_ = std::move(faults);
    }

    /// Bound the store to `keys` keys; a write of a new key past it fails `quota_exceeded`.
    void setQuota(size_t keys) {
        quota_ = keys;
    }

    /// The notification permission, as the platform says it: `granted`, `denied` or `undetermined`.
    void setPermission(std::string state) {
        permission_ = std::move(state);
    }

    /// What the user chooses when asked: `granted`, `denied`, or `none` for a host with no user to ask.
    void setUserDecision(std::string decision) {
        userDecision_ = std::move(decision);
    }

    /// The store's keys and values, in key order.
    const std::map<std::string, std::string> &storeEntries() const {
        return store_;
    }

    /// The ids of the waiting notifications, in the order posted.
    const std::vector<std::string> &waiting() const {
        return waiting_;
    }

    /// The ids of the shown notifications, in the order posted.
    const std::vector<std::string> &shown() const {
        return shown_;
    }

    /// The invocations the host was told to stop, in the order told.
    const std::vector<std::string> &cancelled() const {
        return cancelled_;
    }

    /// The starts the host was given since this was last called, oldest first.
    std::vector<Start> takeStarts() {
        return std::exchange(starts_, {});
    }

    /// Generate texts with `provider`, which outlives the host, from here on.
    void setModelProvider(ModelProvider &provider) {
        provider_ = &provider;
    }

    /// Resolve a generation's `answer_schema` through the `count` schemas at `schemas`, the table of the
    /// machine this host serves (its generated `ANSWER_SCHEMAS`). A request that names an id the table
    /// does not hold is refused.
    void setAnswerSchemas(const SCE::AnswerSchema *schemas, size_t count) {
        answerSchemas_.assign(schemas, schemas + count);
    }

    template <size_t N> void setAnswerSchemas(const SCE::AnswerSchema (&schemas)[N]) {
        setAnswerSchemas(schemas, N);
    }

    /// Start the operation of invocation `id` with `params`.
    ///
    /// Nothing for a type that is not one of the standard host features: the host serves none of it.
    /// Otherwise the arguments are judged first (a refusal starts nothing), then a failure the embedding
    /// injected, then the operation itself.
    std::optional<Outcome> start(const std::string &operation, const std::string &id, const Params &params) {
        if (std::find(TYPES.begin(), TYPES.end(), operation) == TYPES.end()) {
            return std::nullopt;
        }
        starts_.push_back({operation, id, params});
        const auto argument = [&params](const char *name) {
            const auto found = params.find(name);
            return found == params.end() ? std::string() : found->second;
        };
        bool refused = false;
        if (operation == "sce-host-kv-get" || operation == "sce-host-kv-delete" || operation == "sce-host-kv-put") {
            const std::string key = argument("key");
            refused = key.empty() || key.find('\0') != std::string::npos;
        } else if (operation == "sce-host-notify-post" || operation == "sce-host-notify-cancel") {
            refused = argument("id").empty();
        } else if (operation == "sce-host-http-fetch") {
            static const std::array<const char *, 7> methods = {"GET",   "HEAD",   "POST",   "PUT",
                                                                "PATCH", "DELETE", "OPTIONS"};
            const std::string method = argument("method");
            refused = std::find(methods.begin(), methods.end(), method) == methods.end() ||
                      !detail::isHttpUrl(argument("url")) || !detail::headerLines(argument("headers")).has_value();
        } else if (operation == MODEL_GENERATE) {
            std::string why;
            refused = !judgeGeneration(params, why).has_value();
            if (refused) {
                SCE_LOG_INFO("{} {}: refused, {}", operation, id, why);
            }
        }
        if (refused) {
            return Outcome{Outcome::Kind::Refuse, R"({"reason":"invalid_argument"})"};
        }
        if (const auto fault = faults_.find(operation); fault != faults_.end()) {
            SCE_LOG_INFO("{} {}: fails {}, injected", operation, id, fault->second);
            return detail::failure(operation, fault->second);
        }
        if (operation == MODEL_GENERATE) {
            return modelGenerate(id, params);
        }
        if (operation == "sce-host-clock-now") {
            return clockNow();
        }
        if (operation == "sce-host-clock-zone") {
            return clockZone();
        }
        if (operation == "sce-host-kv-get") {
            return kvGet(argument("key"));
        }
        if (operation == "sce-host-kv-put") {
            return kvPut(argument("key"), argument("value"));
        }
        if (operation == "sce-host-kv-delete") {
            return detail::record({detail::boolean("existed", store_.erase(argument("key")) != 0)});
        }
        if (operation == "sce-host-kv-next-key") {
            return kvNextKey(argument("after"), argument("prefix"));
        }
        if (operation == "sce-host-notify-permission") {
            return detail::record({detail::text("state", permission_)});
        }
        if (operation == "sce-host-notify-request-permission") {
            if (permission_ == "undetermined") {
                // A host with no user to ask answers denied.
                permission_ = userDecision_ == "granted" ? "granted" : "denied";
            }
            return detail::record({detail::text("state", permission_)});
        }
        if (operation == "sce-host-notify-post") {
            return notifyPost(argument("id"), argument("at_ms"));
        }
        if (operation == "sce-host-notify-cancel") {
            return detail::record({detail::boolean("existed", withdraw(argument("id")))});
        }
        return httpFetch(id, argument("method"), argument("url"), argument("headers"), argument("body"));
    }

    /// The host is told to stop invocation `id`, because its state exited or its deadline passed. What it
    /// held for it is dropped.
    void cancel(const std::string &id) {
        held_.erase(std::remove(held_.begin(), held_.end(), id), held_.end());
        cancelled_.push_back(id);
    }

private:
    Outcome clockNow() {
        const auto reading = clock_.read();
        if (!reading) {
            return detail::failure("sce-host-clock-now", "unavailable");
        }
        // No answer goes below one already given.
        if (!monotonicFloor_ || reading->monotonicMs > *monotonicFloor_) {
            monotonicFloor_ = reading->monotonicMs;
        }
        return detail::record({detail::text("utc_ms", std::to_string(reading->utcMs)),
                               detail::text("monotonic_ms", std::to_string(*monotonicFloor_))});
    }

    Outcome clockZone() {
        const auto zone = clock_.zone();
        if (!zone) {
            return detail::failure("sce-host-clock-zone", "unavailable");
        }
        return detail::record({detail::text("zone_id", zone->id), detail::number("utc_offset_s", zone->utcOffsetS)});
    }

    Outcome kvGet(const std::string &key) {
        const auto found = store_.find(key);
        if (found == store_.end()) {
            return detail::record({detail::boolean("found", false), detail::text("value", "")});
        }
        return detail::record({detail::boolean("found", true), detail::text("value", found->second)});
    }

    Outcome kvPut(const std::string &key, const std::string &value) {
        const bool replaced = store_.count(key) != 0;
        if (!replaced && quota_ && store_.size() >= *quota_) {
            return detail::failure("sce-host-kv-put", "quota_exceeded");
        }
        store_[key] = value;
        return detail::record({detail::boolean("replaced", replaced)});
    }

    /// The first key, in ascending order of UTF-8 bytes, that is greater than `after` and begins with
    /// `prefix`; `after` is excluded, and an empty `after` starts at the least key. A `std::string` compares
    /// by its bytes, as unsigned characters, which for UTF-8 is that order.
    Outcome kvNextKey(const std::string &after, const std::string &prefix) {
        for (const auto &entry : store_) {
            const std::string &key = entry.first;
            if ((after.empty() || key > after) && key.compare(0, prefix.size(), prefix) == 0) {
                return detail::record({detail::boolean("found", true), detail::text("key", key)});
            }
        }
        return detail::record({detail::boolean("found", false), detail::text("key", "")});
    }

    /// Remove what `id` names from both lists; whether it named anything.
    bool withdraw(const std::string &id) {
        const size_t before = waiting_.size() + shown_.size();
        waiting_.erase(std::remove(waiting_.begin(), waiting_.end(), id), waiting_.end());
        shown_.erase(std::remove(shown_.begin(), shown_.end(), id), shown_.end());
        return waiting_.size() + shown_.size() != before;
    }

    Outcome notifyPost(const std::string &id, const std::string &atMs) {
        if (permission_ != "granted") {
            return detail::record({detail::text("status", "denied")});
        }
        const auto reading = clock_.read();
        if (!reading) {
            return detail::failure("sce-host-notify-post", "unavailable");
        }
        char *end = nullptr;
        const long long at = std::strtoll(atMs.c_str(), &end, 10);
        withdraw(id);
        if (end != atMs.c_str() && at > reading->utcMs) {
            waiting_.push_back(id);
            return detail::record({detail::text("status", "scheduled")});
        }
        shown_.push_back(id);
        return detail::record({detail::text("status", "posted")});
    }

    Outcome httpFetch(const std::string &id, const std::string &method, const std::string &url,
                      const std::string &headers, const std::string &body) {
        const Exchange exchange = transport_.exchange({method, url, detail::headerLines(headers).value(), body});
        if (exchange.kind == Exchange::Kind::Failure) {
            return detail::failure("sce-host-http-fetch", "network");
        }
        if (exchange.kind == Exchange::Kind::Hold) {
            held_.push_back(id);
            return {Outcome::Kind::Hold, ""};
        }
        std::string joined;
        for (const auto &header : exchange.response.headers) {
            std::string name = header.name;
            std::transform(name.begin(), name.end(), name.begin(), [](unsigned char c) { return std::tolower(c); });
            joined += (joined.empty() ? "" : "\n") + name + ": " + detail::trimBlanks(header.value);
        }
        // A response to HEAD has no body, whatever the transport carried.
        const std::string payload = method == "HEAD" ? std::string() : exchange.response.body;
        if (joined.size() > 4096 || payload.size() > 16384) {
            return detail::failure("sce-host-http-fetch", "response_too_large");
        }
        if (!detail::validUtf8(payload)) {
            return detail::failure("sce-host-http-fetch", "invalid_body");
        }
        return detail::record({detail::number("status", exchange.response.status), detail::text("headers", joined),
                               detail::text("body", payload)});
    }

    /// The request the host accepts, or nothing with `why` it refuses it: an empty prompt, `max_tokens`
    /// zero, a temperature past TEMPERATURE_MAX_MILLI, or an answer schema the table does not hold.
    std::optional<GenerateRequest> judgeGeneration(const Params &params, std::string &why) const {
        const auto argument = [&params](const char *name) {
            const auto found = params.find(name);
            return found == params.end() ? std::string() : found->second;
        };
        const auto count = [](const std::string &digits) -> std::optional<uint64_t> {
            if (digits.empty() || digits.size() > 10 ||
                !std::all_of(digits.begin(), digits.end(), [](unsigned char c) { return std::isdigit(c) != 0; })) {
                return std::nullopt;
            }
            return std::stoull(digits);
        };
        GenerateRequest request;
        request.prompt = argument("prompt");
        const auto maxTokens = count(argument("max_tokens"));
        const auto temperature = count(argument("temperature_milli"));
        if (request.prompt.empty()) {
            why = "the prompt is empty";
            return std::nullopt;
        }
        if (!maxTokens || *maxTokens == 0 || *maxTokens > UINT32_MAX) {
            why = "max_tokens is not a count above zero";
            return std::nullopt;
        }
        if (!temperature || *temperature > TEMPERATURE_MAX_MILLI) {
            why = "the temperature is past 2000 thousandths";
            return std::nullopt;
        }
        request.maxTokens = static_cast<uint32_t>(*maxTokens);
        request.temperatureMilli = static_cast<uint32_t>(*temperature);
        if (const std::string named = argument("answer_schema"); !named.empty()) {
            const auto found = std::find_if(answerSchemas_.begin(), answerSchemas_.end(),
                                            [&named](const SCE::AnswerSchema &schema) { return named == schema.id; });
            if (found == answerSchemas_.end()) {
                why = "the answer schema is not one of the machine's";
                return std::nullopt;
            }
            request.answerSchema = &*found;
        }
        return request;
    }

    /// Generate a text for invocation `id`, whose request the host accepted.
    Outcome modelGenerate(const std::string &id, const Params &params) {
        std::string why;
        const auto request = judgeGeneration(params, why);
        const Generation generation = (provider_ ? *provider_ : scripted_).generate(*request);
        switch (generation.kind) {
        case Generation::Kind::Answer:
            if (generation.text.size() > ANSWER_MAX_BYTES) {
                SCE_LOG_INFO("{} {}: an answer of {} bytes is past {}", MODEL_GENERATE, id, generation.text.size(),
                             ANSWER_MAX_BYTES);
                return detail::failure(MODEL_GENERATE, "answer_too_large");
            }
            SCE_LOG_DEBUG("{} {}: answered by {}, finish {}", MODEL_GENERATE, id, generation.model, generation.finish);
            return detail::record(
                {detail::text("text", generation.text), detail::boolean("constrained", generation.constrained),
                 detail::text("model", generation.model), detail::number("prompt_tokens", generation.promptTokens),
                 detail::number("answer_tokens", generation.answerTokens), detail::text("finish", generation.finish)});
        case Generation::Kind::Failure:
            SCE_LOG_INFO("{} {}: fails {}", MODEL_GENERATE, id, generation.code);
            return detail::failure(MODEL_GENERATE, generation.code);
        case Generation::Kind::Refusal:
            SCE_LOG_INFO("{} {}: refused by the provider", MODEL_GENERATE, id);
            return Outcome{Outcome::Kind::Refuse, R"({"reason":"invalid_argument"})"};
        case Generation::Kind::Hold:
            break;
        }
        SCE_LOG_DEBUG("{} {}: held", MODEL_GENERATE, id);
        held_.push_back(id);
        return {Outcome::Kind::Hold, ""};
    }

    ClockSource &clock_;
    Transport &transport_;
    /// Where a generation's text comes from: `scripted_` until the embedding gives another.
    ScriptedProvider scripted_;
    ModelProvider *provider_ = nullptr;
    /// The machine's table of answer schemas, which a generation's `answer_schema` is resolved through.
    std::vector<SCE::AnswerSchema> answerSchemas_;
    std::map<std::string, std::string> store_;
    std::optional<size_t> quota_;
    std::string permission_ = "undetermined";
    std::string userDecision_ = "none";
    /// The ids of the notifications kept to be shown at their instant, and of those handed to the platform,
    /// in the order posted.
    std::vector<std::string> waiting_;
    std::vector<std::string> shown_;
    std::map<std::string, std::string> faults_;
    std::vector<std::string> held_;
    std::vector<std::string> cancelled_;
    std::optional<int64_t> monotonicFloor_;
    std::vector<Start> starts_;
};

/**
 * Register `host` as the invoker of every standard type on `machine`, before it starts.
 *
 * A start is answered with its completion (a failure is a completion whose `error` is a code) or its
 * refusal, or not at all when the host holds it. The cancel the engine delivers when a state exits or a
 * deadline passes reaches the host too.
 */
template <typename Machine> void serve(Machine &machine, ReferenceHost &host) {
    for (const char *operation : TYPES) {
        machine.registerInvoker(
            operation, [&host, operation](const SCE::HostInvokeEvent &event) -> std::optional<SCE::HostInvokeResponse> {
                if (event.cancel.has_value()) {
                    host.cancel(event.cancel->invokeId);
                    return std::nullopt;
                }
                if (!event.start.has_value()) {
                    return std::nullopt;
                }
                Params params;
                for (const auto &[name, values] : event.start->params) {
                    if (!values.empty()) {
                        params[name] = values.front();
                    }
                }
                const auto outcome = host.start(operation, event.start->invokeId, params);
                if (!outcome) {
                    return std::nullopt;
                }
                SCE::HostInvokeResponse response;
                if (outcome->kind == Outcome::Kind::Done) {
                    response.doneData = outcome->data;
                    return response;
                }
                if (outcome->kind == Outcome::Kind::Refuse) {
                    response.refusal = outcome->data;
                    return response;
                }
                return std::nullopt;
            });
    }
}

}  // namespace SCE::StandardHost
