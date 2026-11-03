// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0057 §6 and §10: the C++ machine generated for every build of a
// product line, compiled under one feature set, is the machine that set's
// variant generates alone (the commuting law `select(gen(D)) = gen(project(D))`).
//
// The generator proves it of the tokens, build by build, before it writes
// anything. This program is compiled under one feature set (`-D` of each
// feature's macro, as a build of the product line selects), so the compiler
// resolves the `#if` itself, and drives both machines through every run of up
// to four of the events either one names: they must agree on every observation
// a host has of a machine — where it is, what it asked its host to do, which
// events it discarded, which names it had no event for, and where it ended.
//
// Fixture: tests/product_line/session.scxml, generated with the namespace
// prefix `lifted` once for every build and with `alone` for this feature set;
// see the block that builds this program in tests/CMakeLists.txt.

#include "alone/session_sm.h"
#include "lifted/session_sm.h"

#include <cstddef>
#include <cstdint>
#include <gtest/gtest.h>
#include <optional>
#include <ostream>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

namespace Lifted = SCE::Generated::lifted::session;
namespace Alone = SCE::Generated::alone::session;

/// Every event name either machine knows, and one neither does.
const std::vector<std::string> EVENTS = {"ready", "inbound.start", "connect", "done", "close", "unknown"};
constexpr std::size_t LONGEST_RUN = 4;

/// What a host observes of a machine after a run of events.
struct Observation {
    std::string active;
    std::string calls;
    std::uint32_t discarded = 0;
    /// Names the machine has no event for: neither machine can be handed one.
    std::uint32_t unnamed = 0;
    std::string terminal;

    bool operator==(const Observation &other) const {
        return active == other.active && calls == other.calls && discarded == other.discarded &&
               unnamed == other.unnamed && terminal == other.terminal;
    }
};

std::ostream &operator<<(std::ostream &out, const Observation &seen) {
    return out << "active=" << seen.active << " calls=" << seen.calls << " discarded=" << seen.discarded
               << " unnamed=" << seen.unnamed << " terminal=" << seen.terminal;
}

/// The host serves the actions this build's machine declares, as any host of
/// a product line does: an action of a feature it leaves out is no member of
/// the machine's interface.
template <typename Actions> class Host : public Actions {
public:
    explicit Host(std::string &calls) : calls_(calls) {}

    void onIdle() override {
        called("on_idle");
    }
#if defined(SCE_FEATURE_SESSION_UNICAST)
    void announceUnicast() override {
        called("announce_unicast");
    }
#endif
#if defined(SCE_FEATURE_SESSION_UNICAST_ACCEPT)
    void beginAccept() override {
        called("begin_accept");
    }

    void endAccept() override {
        called("end_accept");
    }
#endif

private:
    void called(const char *action) {
        calls_ += action;
        calls_ += ';';
    }

    std::string &calls_;
};

/// The observation of `Machine` after `run`.
template <typename Machine, typename Actions> Observation observe(const std::vector<std::string> &run) {
    using Policy = typename Machine::PolicyType;
    Observation seen;
    Host<Actions> host(seen.calls);
    Machine sm(host);
    sm.initialize();
    for (const std::string &name : run) {
        const auto event = sm.resolveEventByName(name);
        if (!event) {
            ++seen.unnamed;
            continue;
        }
        sm.raiseExternal(*event);
        sm.step();
    }
    for (const auto state : sm.getActiveStates()) {
        seen.active += Policy::getStateName(state);
        seen.active += ';';
    }
    seen.discarded = sm.discardedExternalEvents();
    if (const auto end = sm.terminalState()) {
        seen.terminal = Policy::getStateName(*end);
    }
    return seen;
}

std::string spelled(const std::vector<std::string> &run) {
    std::string out = "[";
    for (std::size_t i = 0; i < run.size(); ++i) {
        out += (i ? ", " : "") + run[i];
    }
    return out + "]";
}

TEST(TheMachineForEveryBuildIsEachBuildsOwnAotTest, EveryRunIsObservedAlike) {
    unsigned long runs = 0;
    bool ended = false;
    bool calledHost = false;
    for (std::size_t length = 0; length <= LONGEST_RUN; ++length) {
        std::vector<std::size_t> index(length, 0);
        for (;;) {
            std::vector<std::string> run;
            for (const std::size_t i : index) {
                run.push_back(EVENTS[i]);
            }
            const Observation lifted = observe<Lifted::session, Lifted::SessionActions>(run);
            const Observation alone = observe<Alone::session, Alone::SessionActions>(run);
            ++runs;
            ASSERT_EQ(lifted, alone) << "the machine for every build and this build's own part on the run "
                                     << spelled(run);
            ended |= !alone.terminal.empty();
            calledHost |= !alone.calls.empty();
            // The next run of this length, the last event turning fastest.
            std::size_t digit = length;
            while (digit > 0 && ++index[digit - 1] == EVENTS.size()) {
                index[--digit] = 0;
            }
            if (digit == 0) {
                break;
            }
        }
    }
    // Held to something, not only to itself: a run that ended the machine and
    // one that called the host were compared.
    EXPECT_TRUE(ended) << "no run ended the machine, so the comparison held the machines to less than it says";
    EXPECT_TRUE(calledHost) << "no run called the host, so the comparison held the machines to less than it says";
    EXPECT_GT(runs, 0u);
}

/// The default state is a value the document computes, and differs between
/// builds: `probe` is the first child exactly where `session-unicast` is on.
TEST(TheMachineForEveryBuildIsEachBuildsOwnAotTest, TheDefaultStateIsThisBuilds) {
    const Observation start = observe<Lifted::session, Lifted::SessionActions>({});
#if defined(SCE_FEATURE_SESSION_UNICAST)
    EXPECT_EQ(start.active, "probe;");
#else
    EXPECT_EQ(start.active, "idle;");
#endif
}

}  // namespace

}  // namespace SCE::Tests
