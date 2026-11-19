// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The replay of a scenario of `tests/host_features/scenarios` (docs/SCE_HOST_FEATURES.md),
// shared by the two C++ engines that run one: the generated machine
// (AStandardHostScenarioTest) and the Interpreter running the lowered document
// (AStaticDatamodelRunsLoweredUnderTheInterpreterTest; docs/adr/0084, D7). Both
// register the reference host on the machine the same way (`SCE::StandardHost::serve`),
// and both read a scenario through this file, so the two are held to one reading of it:
// what a `host` directive sets, what `host_invokes` and `host_state` are, and the order
// a step is taken in. What differs is only how each engine is driven and read, which it
// hands in as a `HostScenarioDriver`.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` and `expect.host_state` are what the host was asked and what it
// holds afterwards. The expected values are derived from the specification, not observed
// from the host.
//
// The COSE operations (docs/adr/0107, D7) are a feature the host serves only in a build
// that found libsodium, whose target `sce_host_cose` defines SCE_HOST_COSE
// (cmake/SCEHostCose.cmake). A rig of such a build adds them to its host, and its
// `cose_keys` directive sets their keys; a rig of another build serves none of them.

#pragma once

#include "host/StandardHost.h"

#if defined(SCE_HOST_COSE)
#include "host/CoseHost.h"
#include "host/CredentialHost.h"
#include "host/SodiumCoseCrypto.h"
#endif

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <tuple>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#error "SCE_PROJECT_ROOT must name the checkout: the host feature scenarios are read from it"
#endif

namespace SCE::Tests::HostScenario {

using json = nlohmann::json;
namespace host = SCE::StandardHost;

/// The directory every engine's replay reads the scenarios from.
inline std::filesystem::path scenarioDirectory() {
    return std::filesystem::path(SCE_PROJECT_ROOT) / "tests" / "host_features" / "scenarios";
}

/// The scenario of @p machine, `<machine>.json`.
inline json readScenario(const std::string &machine) {
    const auto path = scenarioDirectory() / (machine + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.good()) << "the scenario is not readable: " << path;
    return json::parse(in);
}

/// The stem of every scenario of the directory, sorted.
inline std::vector<std::string> scenarioNames() {
    std::vector<std::string> names;
    for (const auto &entry : std::filesystem::directory_iterator(scenarioDirectory())) {
        if (entry.path().extension() == ".json") {
            names.push_back(entry.path().stem().string());
        }
    }
    std::sort(names.begin(), names.end());
    return names;
}

/// The machine the COSE operations' scenarios name (docs/adr/0107, D7).
inline constexpr const char *COSE_MACHINE = "static_host_cose";

/// The stem of every scenario of the directory that names the machine @p machine, sorted.
inline std::vector<std::string> scenariosOf(const std::string &machine) {
    std::vector<std::string> names;
    for (const auto &name : scenarioNames()) {
        if (readScenario(name).at("machine").get<std::string>() == machine) {
            names.push_back(name);
        }
    }
    return names;
}

/// The bytes a scenario writes as hex, two digits a byte, or nothing when the text is not that.
inline std::optional<std::vector<uint8_t>> bytesOfHex(const std::string &hex) {
    const auto digit = [](char c) -> int {
        if (c >= '0' && c <= '9') {
            return c - '0';
        }
        if (c >= 'a' && c <= 'f') {
            return c - 'a' + 10;
        }
        if (c >= 'A' && c <= 'F') {
            return c - 'A' + 10;
        }
        return -1;
    };
    if (hex.size() % 2 != 0) {
        return std::nullopt;
    }
    std::vector<uint8_t> bytes;
    for (size_t i = 0; i < hex.size(); i += 2) {
        const int high = digit(hex[i]);
        const int low = digit(hex[i + 1]);
        if (high < 0 || low < 0) {
            return std::nullopt;
        }
        bytes.push_back(static_cast<uint8_t>(high * 16 + low));
    }
    return bytes;
}

/// Answers what a scenario says and keeps what it was given.
class ScriptTransport : public host::Transport {
public:
    bool set = false;
    bool hold = false;
    bool fail = false;
    host::HttpResponse response;
    std::vector<host::HttpRequest> log;

    host::Exchange exchange(const host::HttpRequest &request) override {
        log.push_back(request);
        if (hold) {
            return {host::Exchange::Kind::Hold, {}};
        }
        if (fail || !set) {
            return {host::Exchange::Kind::Failure, {}};
        }
        return {host::Exchange::Kind::Response, response};
    }

    void directive(const json &d) {
        set = true;
        hold = d.value("hold", false);
        fail = d.contains("fail");
        response = {};
        if (hold || fail) {
            return;
        }
        response.status = d.at("status").get<int>();
        for (const auto &pair : d.at("headers")) {
            response.headers.push_back({pair[0].get<std::string>(), pair[1].get<std::string>()});
        }
        if (d.contains("body")) {
            response.body = d.at("body").get<std::string>();
        } else if (d.contains("body_hex")) {
            const std::string hex = d.at("body_hex").get<std::string>();
            for (size_t i = 0; i + 1 < hex.size(); i += 2) {
                response.body.push_back(static_cast<char>(std::stoi(hex.substr(i, 2), nullptr, 16)));
            }
        } else if (d.contains("body_repeat")) {
            const auto &repeat = d.at("body_repeat");
            for (int i = 0; i < repeat[1].get<int>(); ++i) {
                response.body += repeat[0].get<std::string>();
            }
        }
    }
};

/// The text a `model` answer gives: `text`, or `text_repeat` as a text and a count.
inline std::string answerText(const json &answer) {
    if (answer.contains("text")) {
        return answer.at("text").get<std::string>();
    }
    std::string text;
    const json &repeat = answer.at("text_repeat");
    for (int i = 0; i < repeat.at(1).get<int>(); ++i) {
        text += repeat.at(0).get<std::string>();
    }
    return text;
}

/// The request fields a scenario writes as numbers; every other is text.
inline bool integerField(const std::string &name) {
    return name == "at_ms" || name == "length" || name == "max_tokens" || name == "temperature_milli" ||
           name == "ttl_s";
}

#if defined(SCE_HOST_COSE)
/// The primitives on libsodium, but for a random source that gives the bytes a scenario's `random` directive
/// names before the operating system's: a draw takes them in order, and once they are spent draws the rest
/// from the system, as the Rust reference host's `ScriptedRandom` does.
class ScriptedRandomCrypto : public host::SodiumCoseCrypto {
public:
    /// Give `bytes` next, in place of what was queued before and not drawn.
    void queue(const std::vector<uint8_t> &bytes) {
        queued_.assign(bytes.begin(), bytes.end());
    }

    std::optional<std::vector<uint8_t>> random(size_t size) override {
        const size_t taken = std::min(size, queued_.size());
        std::vector<uint8_t> out(queued_.begin(), queued_.begin() + static_cast<std::ptrdiff_t>(taken));
        queued_.erase(queued_.begin(), queued_.begin() + static_cast<std::ptrdiff_t>(taken));
        const auto rest = host::SodiumCoseCrypto::random(size - taken);
        if (!rest) {
            return std::nullopt;
        }
        out.insert(out.end(), rest->begin(), rest->end());
        return out;
    }

private:
    std::vector<uint8_t> queued_;
};
#endif

/// The reference host, over a clock, a transport and a model provider a scenario sets, and in a build
/// that has them, the COSE operations on libsodium with the keys and the random bytes a scenario sets,
/// and the credential operations over the same key ring and clock.
struct Rig {
    host::ScriptedClock clock;
    ScriptTransport transport;
#if defined(SCE_HOST_COSE)
    ScriptedRandomCrypto crypto;
    host::CoseFeature cose{crypto};
    host::CredentialFeature credentials{cose, crypto, clock};
#endif
    host::ReferenceHost host{clock, transport};
    host::ScriptedProvider model;

    Rig() {
        host.setModelProvider(model);
#if defined(SCE_HOST_COSE)
        EXPECT_TRUE(host.addFeature(cose)) << "the COSE operations are served by the host already";
        EXPECT_TRUE(host.addFeature(credentials)) << "the credential operations are served by the host already";
#endif
    }

    /// The `credentials` member of `host_state`: each name the host issued a token for, to its tokens as
    /// `{issued_ms, expires_ms, revoked}`, in the order of those three. A token's digest depends on the
    /// session's id, which differs by lane, so a token is named by what the host keeps beside it.
    json credentialState() {
        json state = json::object();
#if defined(SCE_HOST_COSE)
        std::map<std::string, std::vector<std::tuple<uint64_t, uint64_t, bool>>> byName;
        for (const auto &entry : credentials.tokens()) {
            const host::StoredToken &token = entry.second;
            byName[token.name].emplace_back(token.issuedMs, token.expiresMs, token.revoked);
        }
        for (auto &[name, tokens] : byName) {
            std::sort(tokens.begin(), tokens.end());
            json list = json::array();
            for (const auto &[issuedMs, expiresMs, revoked] : tokens) {
                list.push_back({{"issued_ms", issuedMs}, {"expires_ms", expiresMs}, {"revoked", revoked}});
            }
            state[name] = list;
        }
#else
        ADD_FAILURE() << "`credentials` is the state of the credential operations, which this build does not "
                      << "serve: it found no libsodium (cmake/SCEHostCose.cmake)";
#endif
        return state;
    }

    /// Apply a `cose_keys` directive to the COSE operations: it replaces the host's four tables, a table
    /// it does not name with none, `sign` (an Ed25519 seed in hex by its key id), `aead` (a
    /// ChaCha20/Poly1305 key in hex by its key id), `mac` (an HMAC-SHA-256 key of any length in hex by its
    /// key id) and `trust` (the ids of the directive's signing keys a trust set trusts, by the set's name).
    /// The keys generated and derived before are kept.
    void coseKeys(const json &d) {
#if defined(SCE_HOST_COSE)
        for (const auto &[member, ignored] : d.items()) {
            if (member != "sign" && member != "aead" && member != "mac" && member != "trust") {
                ADD_FAILURE() << "`" << member << "` is not a table of `cose_keys`";
            }
        }
        const auto keys = [&d](const char *member) {
            std::map<std::string, std::vector<uint8_t>> out;
            if (!d.contains(member)) {
                return out;
            }
            for (const auto &[id, hex] : d.at(member).items()) {
                const auto key = bytesOfHex(hex.get<std::string>());
                EXPECT_TRUE(key.has_value()) << "the key " << id << " is not written as hex";
                out[id] = key.value_or(std::vector<uint8_t>{});
            }
            return out;
        };
        EXPECT_TRUE(cose.setSigningKeys(keys("sign")))
            << "a signing key of `cose_keys` is not a 32-byte seed under an id a request can name";
        EXPECT_TRUE(cose.setSealingKeys(keys("aead")))
            << "a sealing key of `cose_keys` is not a 32-byte key under an id a request can name";
        EXPECT_TRUE(cose.setMacKeys(keys("mac"))) << "a MAC key of `cose_keys` is not under an id a request can name";
        cose.setTrustSets(d.contains("trust") ? d.at("trust").get<std::map<std::string, std::vector<std::string>>>()
                                              : std::map<std::string, std::vector<std::string>>{});
#else
        ADD_FAILURE() << "`cose_keys` sets the keys of the COSE operations, which this build does not serve: "
                      << "it found no libsodium (cmake/SCEHostCose.cmake); " << d.dump();
#endif
    }

    /// Apply a `random` directive: the bytes, in hex, the host's random source gives next.
    void randomBytes(const json &d) {
#if defined(SCE_HOST_COSE)
        const auto bytes = bytesOfHex(d.get<std::string>());
        EXPECT_TRUE(bytes.has_value()) << "the random bytes are not written as hex";
        crypto.queue(bytes.value_or(std::vector<uint8_t>{}));
#else
        ADD_FAILURE() << "`random` sets the random source of the COSE operations, which this build does not "
                      << "serve: it found no libsodium (cmake/SCEHostCose.cmake); " << d.dump();
#endif
    }

    /// Apply a `model` directive to the scripted provider: each member it names.
    void scriptModel(const json &d) {
        if (d.contains("answers")) {
            model.answers.clear();
            for (const auto &answer : d.at("answers")) {
                model.answers.push_back({answerText(answer), answer.value("finish", std::string("stop"))});
            }
        }
        model.constrained = d.value("constrained", model.constrained);
        model.name = d.value("name", model.name);
        model.context = d.value("context", model.context);
        model.hold = d.value("hold", model.hold);
    }

    void directives(const json &directives) {
        for (const auto &[name, value] : directives.items()) {
            if (name == "clock") {
                host::Reading reading = *clock.reading;
                host::Zone zone = *clock.zoneNow;
                reading.utcMs = value.value("utc_ms", reading.utcMs);
                reading.monotonicMs = value.value("monotonic_ms", reading.monotonicMs);
                zone.id = value.value("zone_id", zone.id);
                zone.utcOffsetS = value.value("utc_offset_s", zone.utcOffsetS);
                clock.reading = reading;
                clock.zoneNow = zone;
            } else if (name == "permission") {
                host.setPermission(value.get<std::string>());
            } else if (name == "user_decision") {
                host.setUserDecision(value.get<std::string>());
            } else if (name == "quota") {
                host.setQuota(value.get<size_t>());
            } else if (name == "fail") {
                host.setFaults(value.get<std::map<std::string, std::string>>());
            } else if (name == "http") {
                transport.directive(value);
            } else if (name == "model") {
                scriptModel(value);
            } else if (name == "cose_keys") {
                coseKeys(value);
            } else if (name == "random") {
                randomBytes(value);
            } else {
                ADD_FAILURE() << "`" << name << "` is not a host directive";
            }
        }
    }

    /// What the host was started with since the last step, as a scenario writes it: the request as its
    /// record, the integer fields as numbers.
    json starts() {
        json out = json::array();
        for (const auto &start : host.takeStarts()) {
            json request = json::object();
            for (const auto &[name, text] : start.params) {
                request[name] = integerField(name) ? json(std::stoll(text)) : json(text);
            }
            out.push_back({{"type", start.operation}, {"id", start.id}, {"request", request}});
        }
        return out;
    }

    json hostState(const json &wanted) {
        json state = json::object();
        for (const auto &[name, ignored] : wanted.items()) {
            if (name == "store") {
                json store = json::object();
                for (const auto &[key, value] : host.storeEntries()) {
                    store[key] = value;
                }
                state[name] = store;
            } else if (name == "waiting") {
                state[name] = host.waiting();
            } else if (name == "shown") {
                state[name] = host.shown();
            } else if (name == "cancelled") {
                state[name] = host.cancelled();
            } else if (name == "http") {
                json log = json::array();
                for (const auto &r : transport.log) {
                    json headers = json::array();
                    for (const auto &h : r.headers) {
                        headers.push_back({{"name", h.name}, {"value", h.value}});
                    }
                    log.push_back({{"method", r.method}, {"url", r.url}, {"headers", headers}, {"body", r.body}});
                }
                state[name] = log;
            } else if (name == "credentials") {
                state[name] = credentialState();
            } else {
                ADD_FAILURE() << "`" << name << "` is not host state";
            }
        }
        return state;
    }
};

/// How one engine is driven through a scenario and read back. Each member is asked in
/// the order a step is taken: the host's directives first, then `advance` for a step's
/// `advance_ms`, then `raise` for its `event`, and then the expectations.
struct HostScenarioDriver {
    /// Move the machine's time on by the milliseconds named, and run what fell due.
    std::function<void(uint64_t)> advance;
    /// Deliver the external event named, with the JSON text of the step's `data` (empty
    /// for none), and run it to completion. `where` names the step for a failure.
    std::function<void(const std::string &where, const std::string &event, const std::string &data)> raise;
    /// The machine's current state, by its id.
    std::function<std::string()> state;
    /// Hold the variable named to the value the step expects.
    std::function<void(const std::string &where, const std::string &name, const json &want)> expectVariable;
};

/// Replay @p scenario on the machine @p driver drives, served by @p rig's host, which the
/// caller registered on the machine (`SCE::StandardHost::serve`) and started.
inline void replay(const json &scenario, Rig &rig, const HostScenarioDriver &driver) {
    ASSERT_FALSE(scenario.at("steps").empty()) << "a scenario with no steps judges nothing";
    int n = 0;
    for (const auto &step : scenario.at("steps")) {
        const std::string where = "step " + std::to_string(n++) + " (" + step.value("note", "") + ")";
        SCOPED_TRACE(where);
        if (step.contains("host")) {
            rig.directives(step.at("host"));
        }
        if (step.contains("advance_ms")) {
            driver.advance(step.at("advance_ms").get<uint64_t>());
        }
        if (step.contains("event")) {
            driver.raise(where, step.at("event").get<std::string>(),
                         step.contains("data") ? step.at("data").dump() : "");
        }
        const json &expect = step.at("expect");
        const json started = rig.starts();
        if (expect.contains("host_invokes")) {
            EXPECT_EQ(started, expect.at("host_invokes")) << where << ": what the host was asked";
        }
        if (expect.contains("host_state")) {
            EXPECT_EQ(rig.hostState(expect.at("host_state")), expect.at("host_state"))
                << where << ": what the host holds";
        }
        if (expect.contains("state")) {
            EXPECT_EQ(driver.state(), expect.at("state").get<std::string>()) << where << ": the current state";
        }
        if (expect.contains("variables")) {
            for (const auto &[variable, want] : expect.at("variables").items()) {
                driver.expectVariable(where, variable, want);
            }
        }
    }
}

}  // namespace SCE::Tests::HostScenario
