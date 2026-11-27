// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0147, D3: the door of docs/adr/0084 is a concurrent object, and each run
// of the generated C++ engine's inbox is a history. This program runs the inbox of
// tests/integration/inbox_probe.scxml on real threads, records every post each
// poster made and every event the engine put on the machine's external queue, with
// the end of the machine's run, and writes each run in the form every runtime's
// inbox history is written in (tests/forge/conformance/inbox_history.schema.json).
// It judges nothing: `sce-codegen check-inbox-history` holds each history
// linearizable to docs/formal/InboxService.tla, as it holds every runtime's (ctest
// `cpp_inbox_history_check`, which runs after this program, through the script the C11
// machine's runs are judged by, tests/forge/conformance/check_inbox_histories.cmake).
//
// What enters the external queue is told by the engine: the machine is built with
// its inbox port on (`SCE_INBOX_PORT`, core/InboxPort.h), so the engine tells the
// port below when it begins to put each event there, ahead of the drain before it,
// and immediately after the event entered, saying whether its inbox held the event.
// The drain, the order it keeps and the inbox are the engine's; this program reads
// the clock around them.
//
// A run: a first poster posts `ping`s, and an ending of `job` through the tree's
// directory by the machine's own session id, while the owner stands between two
// drives. The owner then raises a `ping` of its own, whose drain applies them all
// first; while the first of them enters, a post is made that the drain did not read,
// which may follow the event. Then several threads post typed and named `ping`s, and
// now and then an ending naming a session the tree does not hold, while the owner
// drives the machine, raises `ping`s of its own and stops it once half the posts are
// made. Last, once the posters are joined, a `ping` through the machine's handle and an
// ending naming the machine through the directory. Every input and every own event
// carries its value as its data, which no other of the run has, and the port names
// each event it is told by that value.
//
// The engine closes the machine's inbox when its run ends (`stop`): what it holds
// then is applied, a later post through its handle is refused as ended, and the
// machine leaves the tree's directory, so a later post naming it there is refused as
// unknown. Every history says `closes: true`. The inbox has no bound (docs/adr/0084,
// D2), so no history gives a capacity, and no post is refused as full.
//
// The end of the run is recorded around `stop`, from before the call to the instant
// the inbox is known to have closed: the first event the port is told inside the
// call is the first input of the drain that follows the close, so the end returns at
// the reading taken there, or after the call when the port is told none. That
// drain's enqueues follow the end, as an input taken before the end enters after it.
//
// Beside the runs, the first run is written again with the own event whose drain a
// post followed read at its push, after that post returned, as a recorder that read
// it there would have: the post is then held to enter ahead of the event, which it
// did not, and that history is refused (ctest `cpp_inbox_history_check_at_the_push`).
//
//   integration_inbox_history_recorder <directory>
//
// writes <directory>/runs/inbox_cpp_run<NN>.json and
// <directory>/at_the_push/inbox_cpp_run00.json.

#include "inbox_probe_sm.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <functional>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <system_error>
#include <thread>
#include <utility>
#include <vector>

#include "common/ForwardedEvent.h"
#include "core/HostProcessor.h"
#include "core/InboxPort.h"
#include "scripting/JSEngine.h"
#include "scripting/ScriptEngineProvider.h"

namespace {

using Machine = SCE::Generated::inbox_probe::inbox_probe;
using Event = SCE::Generated::inbox_probe::Event;

static_assert(Machine::HAS_INBOX_PORT, "the machine is built with its inbox port on (SCE_INBOX_PORT=1)");

/// The type the fixture was compiled for (`HOST_INVOKER` in tests/CMakeLists.txt).
constexpr const char *TYPE = "x-sce-host";
/// The fixture's one host-run invocation.
constexpr const char *JOB = "job";

/// Threads that post, beside the owner, and the posts each makes in one run.
constexpr int POSTERS = 4;
constexpr int PER_POSTER = 48;
constexpr int TOTAL_POSTS = POSTERS * PER_POSTER;
/// The `ping`s the first poster posts before the owner's first own event.
constexpr int FILLER_PINGS = 8;
/// Runs: the schedules that matter are a small fraction of those a machine gives.
constexpr int RUNS = 8;
/// Own events the owner raises at most in one run, and the value of the first,
/// past every input a poster posts.
constexpr uint64_t OWN_PER_RUN = 24;
constexpr uint64_t OWN_BASE = 1000000;
/// The number the history names the machine's session by, and the session the
/// tree does not hold.
constexpr uint64_t SESSION = 0;
constexpr uint64_t ABSENT = 1;
constexpr const char *ABSENT_ID = "inbox.history.absent";
/// The inputs of the posts no poster of the run's middle makes: the one the drain
/// before the first own event did not read, and the two made once the run ended.
constexpr uint64_t BEHIND_INPUT = 900000;
constexpr uint64_t LATE_HANDLE_INPUT = 900001;
constexpr uint64_t LATE_ADDRESSED_INPUT = 900002;

// ── The clock and the record ─────────────────────────────────────────

/// The one clock every thread of a run reads, immediately before and immediately
/// after each operation it records: one read-modify-write chain on one atomic
/// orders the readings with the calls between them, so "returned before invoked"
/// in the record means it in the run, and no two readings are equal.
std::atomic<uint64_t> g_clock{0};

uint64_t tick() {
    return g_clock.fetch_add(1, std::memory_order_acq_rel) + 1;
}

/// The door a post went through (docs/adr/0147, D1).
enum class Door { Addressed, Handle };

/// What the inbox answered a post, as the history says it.
enum class Outcome { Taken, RefusedEnded, RefusedUnknown };

/// One post a poster observed.
struct PostRecord {
    uint64_t session = 0;
    uint64_t input = 0;
    Door door = Door::Handle;
    Outcome outcome = Outcome::Taken;
    uint64_t invoked = 0;
    uint64_t returned = 0;
};

/// One operation the owner observed: an end, or an enqueue of `item`, which the
/// owner raised itself when `own`.
struct OwnerRecord {
    bool end = false;
    uint64_t item = 0;
    bool own = false;
    uint64_t invoked = 0;
    uint64_t returned = 0;
};

/// One run: the owner's operations in its program order, and each poster's.
struct History {
    std::vector<OwnerRecord> owner;
    std::vector<std::vector<PostRecord>> posters;
};

/// The value an event carries as its data: the digits of `"<value>"`; none when it
/// carries none.
std::optional<uint64_t> valueOf(std::string_view data) {
    std::size_t at = 0;
    while (at < data.size() && (data[at] < '0' || data[at] > '9')) {
        ++at;
    }
    if (at == data.size()) {
        return std::nullopt;
    }
    uint64_t value = 0;
    for (; at < data.size() && data[at] >= '0' && data[at] <= '9'; ++at) {
        value = value * 10 + static_cast<uint64_t>(data[at] - '0');
    }
    return value;
}

/// Post with `post`, which answers whether the inbox took the input, recorded as a
/// post of input `value` to `session` through `door` with the clock read
/// immediately around it. A post through the machine's handle is refused only once
/// its inbox closed, and one through the directory only for the session it names.
PostRecord recordPost(uint64_t session, uint64_t value, Door door, const std::function<bool()> &post) {
    PostRecord record;
    record.session = session;
    record.input = value;
    record.door = door;
    record.invoked = tick();
    const bool taken = post();
    record.returned = tick();
    if (taken) {
        record.outcome = Outcome::Taken;
    } else {
        record.outcome = door == Door::Handle ? Outcome::RefusedEnded : Outcome::RefusedUnknown;
    }
    return record;
}

/// What the owner records: the engine's port, and the end of the run. Written on
/// the owner only.
class Recorder {
public:
    /// The inbox port: each event the engine puts on the machine's external queue,
    /// told when the engine begins to put it there and once it entered.
    void told(const SCE::Core::InboxEnqueue &enqueue) {
        const std::size_t at = enqueue.posted ? 1 : 0;
        if (!enqueue.entered) {
            uint64_t reading = tick();
            if (ending_) {
                // Inside `stop`: the inbox closed before the drain that applies what
                // it held began, so the end returns here and the drain follows it.
                if (!enqueue.posted) {
                    fail("an event of the machine's own was put on its queue while its run ended, before its "
                         "inbox closed, where the end of the run cannot be read");
                }
                closeEnd(reading);
                reading = tick();
            }
            begun_[at] = reading;
            if (!enqueue.posted) {
                ownOpen_ = true;
            }
            return;
        }
        const std::optional<uint64_t> value = valueOf(enqueue.data);
        if (!value) {
            fail("an event entered the external queue with no value as its data: " + std::string(enqueue.data));
            return;
        }
        OwnerRecord record;
        record.item = *value;
        record.own = !enqueue.posted;
        // docs/adr/0147, D3: an input the drain before an event of the machine's own
        // applies left the inbox at that drain's read, which comes after the event's
        // first call, so its enqueue is invoked at the event's reading. Read at its own
        // first call, a post that returned between the drain's read and that call would
        // be held to enter ahead of the event, which the drain did not read.
        record.invoked = enqueue.posted && !ownOpen_ ? begun_[1] : begun_[0];
        record.returned = tick();
        owner_.push_back(record);
        if (!enqueue.posted) {
            ownOpen_ = false;
        }
        if (enqueue.posted && behind_) {
            const std::function<void()> post = std::move(behind_);
            behind_ = nullptr;
            post();
        }
    }

    /// Run `post` on the owner once the next input the engine applies from its inbox
    /// has entered: inside the drain that applies it, after that drain read the inbox.
    void behindTheNextInput(std::function<void()> post) {
        behind_ = std::move(post);
    }

    /// Stop `sm`, recorded as the end of its run.
    void end(Machine &sm) {
        ending_ = true;
        endInvoked_ = tick();
        sm.stop();
        if (ending_) {
            closeEnd(tick());
        }
    }

    const std::vector<OwnerRecord> &owner() const {
        return owner_;
    }

    const std::string &failure() const {
        return failure_;
    }

private:
    void closeEnd(uint64_t returned) {
        ending_ = false;
        OwnerRecord record;
        record.end = true;
        record.invoked = endInvoked_;
        record.returned = returned;
        owner_.push_back(record);
    }

    void fail(const std::string &why) {
        if (failure_.empty()) {
            failure_ = why;
        }
    }

    std::vector<OwnerRecord> owner_;
    /// The reading taken when the engine began to put the event it is telling on
    /// the queue: for an event of the machine's own at [0], for an input from the
    /// inbox at [1]. Two, because the inputs the drain before an event of the
    /// machine's own applies are told between that event's two calls.
    uint64_t begun_[2] = {0, 0};
    /// An event of the machine's own is being put on the queue: the port's first
    /// call of it came and its second has not.
    bool ownOpen_ = false;
    bool ending_ = false;
    uint64_t endInvoked_ = 0;
    std::function<void()> behind_;
    std::string failure_;
};

// ── The machine and its posts ────────────────────────────────────────

/// `value` as the JSON string of its digits, which the machine reads back in `_event.data`.
std::string quoted(uint64_t value) {
    return "\"" + std::to_string(value) + "\"";
}

/// `ping` carrying `value`, as the document's own event.
Machine::InboxInput ping(uint64_t value) {
    return Machine::InboxExternalEvent{Machine::EventWithMetadata(Event::Ping, quoted(value))};
}

/// `ping` carrying `value`, named by text, as a host that knows no enumeration
/// names it.
Machine::InboxInput pingByName(uint64_t value) {
    SCE::Common::ForwardedEvent named;
    named.name = "ping";
    named.data = quoted(value);
    named.type = "external";
    return Machine::InboxExternalEventByName{named};
}

/// A completion of start `token` of `job`, carrying `value`, naming the session
/// `sessionId`: posted through the tree's directory.
Machine::InboxInput ending(const std::string &sessionId, uint64_t token, uint64_t value) {
    return Machine::InboxHostInvokeEnding{sessionId, SCE::HostInvokeEnding{/*processorType=*/TYPE,
                                                                           /*invokeId=*/JOB,
                                                                           /*token=*/token,
                                                                           /*done=*/true,
                                                                           /*data=*/quoted(value),
                                                                           /*origin=*/std::nullopt}};
}

/// How many of `posts` the inbox answered with `outcome`.
std::size_t countPosts(const History &history, Outcome outcome) {
    std::size_t count = 0;
    for (const std::vector<PostRecord> &posts : history.posters) {
        for (const PostRecord &post : posts) {
            count += post.outcome == outcome ? 1 : 0;
        }
    }
    return count;
}

/// How many enqueues the owner recorded of events its inbox held (`own` false) or
/// of its own (`own` true).
std::size_t countEntered(const History &history, bool own) {
    std::size_t count = 0;
    for (const OwnerRecord &op : history.owner) {
        count += !op.end && op.own == own ? 1 : 0;
    }
    return count;
}

// ── One run ──────────────────────────────────────────────────────────

/// Run once into `history`; false, saying why in `why`, when the run could not be
/// made or did not exercise what it is for.
bool runOnce(History &history, std::string &why) {
    g_clock.store(0, std::memory_order_release);

    Machine sm;
    // The starts of `job` the host was asked for, on the owner. The host answers
    // nothing, so `job` runs until an ending is applied or the machine leaves
    // `working`.
    std::vector<SCE::HostInvokeRequest> starts;
    sm.registerInvoker(TYPE, [&starts](const SCE::HostInvokeEvent &event) {
        if (event.start.has_value()) {
            starts.push_back(*event.start);
        }
        return std::optional<SCE::HostInvokeResponse>();
    });
    sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                           [](SCE::IScriptEngine *) {}));
    if (sm.initialize().has_value()) {
        why = "the machine did not start";
        return false;
    }
    sm.step();
    if (starts.empty()) {
        why = "`job` never started";
        return false;
    }
    const SCE::HostInvokeRequest job = starts.back();
    if (job.sessionId.empty()) {
        why = "the machine holds no session id, so no post can name it through the tree's directory";
        return false;
    }

    Recorder recorder;
    sm.setInboxPort([&recorder](const SCE::Core::InboxEnqueue &enqueue) { recorder.told(enqueue); });
    const Machine::Inbox inbox = sm.inbox();

    // The first poster, while the owner stands between two drives: `ping`s through
    // the machine's handle, and the ending of `job` through the directory.
    std::vector<PostRecord> filler;
    std::thread fill([&filler, &inbox, &job] {
        for (int k = 0; k < FILLER_PINGS; ++k) {
            const uint64_t value = static_cast<uint64_t>(k) + 1;
            filler.push_back(recordPost(SESSION, value, Door::Handle, [&] { return inbox.post(ping(value)); }));
        }
        const uint64_t value = static_cast<uint64_t>(FILLER_PINGS) + 1;
        filler.push_back(recordPost(SESSION, value, Door::Addressed,
                                    [&] { return inbox.post(ending(job.sessionId, job.token, value)); }));
    });
    fill.join();

    // An event of the owner's own behind what the first poster's posts left in the
    // inbox, so every run enters one; and a post made while the first input its
    // drain applies enters, which that drain did not read.
    std::vector<PostRecord> behind;
    recorder.behindTheNextInput([&behind, &inbox] {
        std::thread poster([&behind, &inbox] {
            behind.push_back(
                recordPost(SESSION, BEHIND_INPUT, Door::Handle, [&] { return inbox.post(ping(BEHIND_INPUT)); }));
        });
        poster.join();
    });
    uint64_t own = OWN_BASE;
    sm.raiseExternal(Event::Ping, quoted(own));
    ++own;

    // The posters, and the owner: a drive, an own event now and then, and the stop
    // once half the posts are made.
    const uint64_t base = static_cast<uint64_t>(FILLER_PINGS) + 2;
    std::vector<std::vector<PostRecord>> logs(POSTERS);
    std::atomic<bool> released{false};
    std::atomic<int> running{POSTERS};
    std::atomic<int> made{0};
    std::vector<std::thread> posters;
    for (int poster = 0; poster < POSTERS; ++poster) {
        posters.emplace_back([&, poster] {
            while (!released.load(std::memory_order_acquire)) {
                std::this_thread::yield();
            }
            std::vector<PostRecord> &mine = logs[static_cast<std::size_t>(poster)];
            for (int k = 0; k < PER_POSTER; ++k) {
                const uint64_t value = base + static_cast<uint64_t>(poster * PER_POSTER + k);
                switch (k % 6) {
                case 5:
                    mine.push_back(recordPost(ABSENT, value, Door::Addressed,
                                              [&] { return inbox.post(ending(ABSENT_ID, 0, value)); }));
                    break;
                case 1:
                case 3:
                    mine.push_back(
                        recordPost(SESSION, value, Door::Handle, [&] { return inbox.post(pingByName(value)); }));
                    break;
                default:
                    mine.push_back(recordPost(SESSION, value, Door::Handle, [&] { return inbox.post(ping(value)); }));
                    break;
                }
                made.fetch_add(1, std::memory_order_acq_rel);
                std::this_thread::yield();
            }
            running.fetch_sub(1, std::memory_order_acq_rel);
        });
    }
    released.store(true, std::memory_order_release);

    bool ended = false;
    unsigned round = 0;
    while (running.load(std::memory_order_acquire) > 0) {
        sm.step();
        ++round;
        if (!ended && round % 3 == 0 && own < OWN_BASE + OWN_PER_RUN) {
            sm.raiseExternal(Event::Ping, quoted(own));
            ++own;
        }
        if (!ended && made.load(std::memory_order_acquire) * 2 >= TOTAL_POSTS) {
            recorder.end(sm);
            ended = true;
        }
        std::this_thread::yield();
    }
    for (std::thread &poster : posters) {
        poster.join();
    }
    if (!ended) {
        recorder.end(sm);
    }

    // Once the run ended, and the machine left the tree's directory: a post through
    // its handle, refused as ended, and one naming it through the directory, refused
    // as unknown.
    std::vector<PostRecord> late;
    std::thread lateness([&late, &inbox, &job] {
        late.push_back(
            recordPost(SESSION, LATE_HANDLE_INPUT, Door::Handle, [&] { return inbox.post(ping(LATE_HANDLE_INPUT)); }));
        late.push_back(recordPost(SESSION, LATE_ADDRESSED_INPUT, Door::Addressed,
                                  [&] { return inbox.post(ending(job.sessionId, job.token, LATE_ADDRESSED_INPUT)); }));
    });
    lateness.join();
    sm.clearInboxPort();

    history.owner = recorder.owner();
    history.posters.clear();
    history.posters.push_back(std::move(filler));
    history.posters.push_back(std::move(behind));
    for (std::vector<PostRecord> &posts : logs) {
        history.posters.push_back(std::move(posts));
    }
    history.posters.push_back(std::move(late));

    if (!recorder.failure().empty()) {
        why = recorder.failure();
        return false;
    }
    if (history.posters[1].size() != 1) {
        why = "no post was made while the drain before the first own event applied what it read";
        return false;
    }
    const std::size_t taken = countPosts(history, Outcome::Taken);
    if (countEntered(history, false) != taken) {
        why = std::to_string(countEntered(history, false)) + " inputs entered the queue of the " +
              std::to_string(taken) + " posts taken";
        return false;
    }
    if (countPosts(history, Outcome::RefusedEnded) == 0 || countPosts(history, Outcome::RefusedUnknown) == 0 ||
        countEntered(history, true) == 0) {
        why = "the run was refused as ended " + std::to_string(countPosts(history, Outcome::RefusedEnded)) +
              " times and as unknown " + std::to_string(countPosts(history, Outcome::RefusedUnknown)) +
              " times, and entered " + std::to_string(countEntered(history, true)) +
              " events of the machine's own, where each was due at least once";
        return false;
    }
    return true;
}

// ── The JSON form ────────────────────────────────────────────────────

void appendPost(std::string &out, const PostRecord &post) {
    out += "{\"call\": \"post\", \"session\": " + std::to_string(post.session) +
           ", \"input\": " + std::to_string(post.input) + ", \"door\": \"" +
           (post.door == Door::Addressed ? "addressed" : "handle") + "\", ";
    switch (post.outcome) {
    case Outcome::Taken:
        out += "\"outcome\": \"taken\", ";
        break;
    case Outcome::RefusedEnded:
        out += "\"outcome\": \"refused\", \"reason\": \"ended\", ";
        break;
    case Outcome::RefusedUnknown:
        out += "\"outcome\": \"refused\", \"reason\": \"unknown\", ";
        break;
    }
    out += "\"invoked\": " + std::to_string(post.invoked) + ", \"returned\": " + std::to_string(post.returned) + "}";
}

void appendOwnerOperation(std::string &out, const OwnerRecord &op) {
    if (op.end) {
        out += "{\"call\": \"end\", \"session\": " + std::to_string(SESSION) + ", ";
    } else {
        out += "{\"call\": \"enqueue\", \"session\": " + std::to_string(SESSION) +
               ", \"item\": " + std::to_string(op.item) + ", \"source\": \"" + (op.own ? "own" : "posted") + "\", ";
    }
    out += "\"invoked\": " + std::to_string(op.invoked) + ", \"returned\": " + std::to_string(op.returned) + "}";
}

/// The run in the JSON form.
std::string toJson(const History &history) {
    std::string out =
        "{\n  \"version\": 2,\n  \"sessions\": [" + std::to_string(SESSION) + "],\n  \"closes\": true,\n  \"owner\": [";
    for (std::size_t at = 0; at < history.owner.size(); ++at) {
        out += at == 0 ? "\n    " : ",\n    ";
        appendOwnerOperation(out, history.owner[at]);
    }
    out += "\n  ],\n  \"posters\": [";
    for (std::size_t who = 0; who < history.posters.size(); ++who) {
        out += who == 0 ? "\n    [" : ",\n    [";
        const std::vector<PostRecord> &posts = history.posters[who];
        for (std::size_t at = 0; at < posts.size(); ++at) {
            out += at == 0 ? "\n      " : ",\n      ";
            appendPost(out, posts[at]);
        }
        out += posts.empty() ? "]" : "\n    ]";
    }
    out += "\n  ]\n}\n";
    return out;
}

/// Write `text` as the file `path`; false when it could not be written.
bool writeFile(const std::filesystem::path &path, const std::string &text) {
    std::FILE *out = std::fopen(path.string().c_str(), "w");
    if (out == nullptr) {
        return false;
    }
    const bool written = std::fputs(text.c_str(), out) >= 0;
    return std::fclose(out) == 0 && written;
}

/// `history` with the first own event read at its push, after the post its
/// drain did not read returned, as a recorder that read it there would have; none
/// when the run does not have that shape. Every reading is doubled, so a reading
/// fits between two of them.
std::optional<History> readAtThePush(const History &history) {
    History moved = history;
    for (OwnerRecord &op : moved.owner) {
        op.invoked *= 2;
        op.returned *= 2;
    }
    for (std::vector<PostRecord> &posts : moved.posters) {
        for (PostRecord &post : posts) {
            post.invoked *= 2;
            post.returned *= 2;
        }
    }
    const uint64_t postReturned = moved.posters[1].at(0).returned;
    for (OwnerRecord &op : moved.owner) {
        if (!op.end && op.own && op.item == OWN_BASE) {
            if (!(op.invoked < postReturned && postReturned < op.returned)) {
                return std::nullopt;
            }
            op.invoked = postReturned + 1;
            return moved;
        }
    }
    return std::nullopt;
}

}  // namespace

int main(int argc, char **argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s <directory the histories are written to>\n", argv[0]);
        return 2;
    }
    const std::filesystem::path root(argv[1]);
    const std::filesystem::path runs = root / "runs";
    const std::filesystem::path atThePush = root / "at_the_push";
    std::error_code error;
    std::filesystem::create_directories(runs, error);
    if (!error) {
        std::filesystem::create_directories(atThePush, error);
    }
    if (error) {
        std::fprintf(stderr, "FAIL: %s could not be made: %s\n", root.string().c_str(), error.message().c_str());
        return 2;
    }

    SCE::JSEngine::instance().initialize();
    int bad = 0;
    for (int run = 0; run < RUNS; ++run) {
        History history;
        std::string why;
        char name[32];
        std::snprintf(name, sizeof(name), "inbox_cpp_run%02d.json", run);
        if (!runOnce(history, why)) {
            std::fprintf(stderr, "FAIL: run %d of the inbox: %s\n", run, why.c_str());
            bad = 1;
            continue;
        }
        if (!writeFile(runs / name, toJson(history))) {
            std::fprintf(stderr, "FAIL: %s could not be written\n", (runs / name).string().c_str());
            bad = 1;
            continue;
        }
        if (run == 0) {
            const std::optional<History> moved = readAtThePush(history);
            if (!moved) {
                std::fprintf(stderr, "FAIL: run 0: the post the drain did not read returned outside the own event's "
                                     "enqueue\n");
                bad = 1;
            } else if (!writeFile(atThePush / name, toJson(*moved))) {
                std::fprintf(stderr, "FAIL: %s could not be written\n", (atThePush / name).string().c_str());
                bad = 1;
            }
        }
    }
    SCE::JSEngine::instance().shutdown();
    if (bad == 0) {
        std::printf("PASS: %d runs of the inbox written\n", RUNS);
    }
    return bad;
}
