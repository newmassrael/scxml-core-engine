// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Host for `ai_loop.scxml` — the half that DOES things.
//
// The statechart decides when to prompt an agent, when to let a person in, and
// when the agent's whole session has to be replaced. It performs none of it.
// This file is the other half: it owns the session, sends the prompts, reads
// what came back, and reports each outcome to the machine as an event.
//
// The seam between them is `AgentSession` below. It is deliberately narrow —
// five operations, none of which mention statecharts — because that is what
// lets the same machine run against a terminal, an HTTP endpoint, or (here) a
// script. `ScriptedSession` replays a fixed transcript, so the example runs
// anywhere with no agent installed and no network. Swapping in a real one
// changes this file and nothing else.
//
// Note what is NOT in the host: no turn counter, no "have we reflected yet",
// no "should we stop now". Every one of those lives in the document, which is
// what keeps `stepOnce()` a dispatch table rather than a second, undeclared
// state machine shadowing the first.
//
// Read `ai_loop.scxml` first — it explains the topology and why it has three
// concurrent regions. This file explains how a host talks to it.

#include <algorithm>
#include <cstdlib>
#include <functional>
#include <iostream>
#include <iterator>
#include <memory>
#include <optional>
#include <string>
#include <variant>
#include <vector>

#include "ai_loop_sm.h"
#include "common/EventDataHelper.h"
#include "scripting/JSEngine.h"
#include "scripting/ScriptEngineProvider.h"

namespace {

using SCE::Generated::ai_loop::Event;
using SCE::Generated::ai_loop::State;
using Machine = SCE::Generated::ai_loop::ai_loop;

// ── the seam ──────────────────────────────────────────────────────────

/// What one turn of the agent produced.
struct TurnResult {
    enum class Kind {
        Done,         ///< finished a turn and went quiet
        Blocked,      ///< asking something, cannot continue
        Interrupted,  ///< a person took over: typed, or pressed Escape
        Lost,         ///< the session is gone (crashed, killed, closed)
    };
    Kind kind = Kind::Done;
    /// The agent's output for this turn. Searched for the done marker.
    std::string text;
    /// For `Blocked`: which shape of question, matched against the document's
    /// `screen_rules`.
    std::string question;
};

class AgentSession {
public:
    virtual ~AgentSession() = default;
    /// Start a fresh session. Anything the loop wrote since the last one —
    /// instructions, context, tool configuration — is read here, which is the
    /// whole reason the loop restarts instead of reconfiguring.
    virtual void start() = 0;
    /// End the session. After this, `start()` is the only way back.
    virtual void stop() = 0;
    /// Send a prompt and wait for the agent to settle.
    virtual TurnResult prompt(const std::string &text) = 0;
    /// Answer a dialog on a standing instruction's behalf: keys first, then
    /// text.
    virtual TurnResult answer(const std::string &keys, const std::string &text) = 0;
    /// Raise whatever attention signal this environment offers, so a waiting
    /// run is visible without anybody watching the screen for it.
    virtual void notify() = 0;
};

/// Replays a fixed transcript. No agent, no network, same output every run.
class ScriptedSession : public AgentSession {
public:
    explicit ScriptedSession(std::vector<TurnResult> script) : script_(std::move(script)) {}

    void start() override {
        ++starts_;
        std::cout << "        [session] started (#" << starts_ << ")\n";
    }

    void stop() override {
        std::cout << "        [session] stopped\n";
    }

    TurnResult prompt(const std::string &text) override {
        std::cout << "        [prompt]  " << firstLine(text) << "\n";
        prompts_.push_back(text);
        return next();
    }

    /// Every prompt this session was sent, in order.
    ///
    /// Recorded because a prompt is the one thing the loop produces that
    /// nothing else can check: the machine's outcome is the same whether the
    /// text it sent was the author's, a reflection's, or empty. `main()`
    /// asserts on this after the reflection scenario for exactly that reason.
    const std::vector<std::string> &prompts() const {
        return prompts_;
    }

    TurnResult answer(const std::string &keys, const std::string &text) override {
        std::cout << "        [answer]  <" << keys << "> " << firstLine(text) << "\n";
        return next();
    }

    void notify() override {
        std::cout << "        [notify]  a person is needed\n";
    }

    int starts() const {
        return starts_;
    }

private:
    TurnResult next() {
        if (cursor_ >= script_.size()) {
            // A transcript that runs out is a bug in the example, not a state
            // the loop should have to model. Say so rather than inventing a
            // turn that never happened.
            std::cout << "        [script]  exhausted — reporting a lost session\n";
            return TurnResult{TurnResult::Kind::Lost, "", ""};
        }
        return script_[cursor_++];
    }

    static std::string firstLine(const std::string &s) {
        auto nl = s.find('\n');
        return nl == std::string::npos ? s : s.substr(0, nl) + " ...";
    }

    std::vector<TurnResult> script_;
    std::vector<std::string> prompts_;
    size_t cursor_ = 0;
    int starts_ = 0;
};

// ── the host ──────────────────────────────────────────────────────────

/// Hand the machine a script engine and boot it.
///
/// W3C SCXML B.1: the machine's datamodel is ECMAScript, so it needs an
/// engine before it starts. Aliasing constructor with a no-op deleter — the
/// provider owns the engine's lifetime and this shared_ptr is a non-owning
/// view, the same idiom the W3C AOT harness uses.
void boot(Machine &machine) {
    machine.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                                [](SCE::IScriptEngine *) {}));
    machine.initialize();
}

class LoopHost {
public:
    LoopHost(Machine &machine, AgentSession &session) : m_(machine), s_(session) {}

    /// Run until the machine reaches one of the outcomes the document
    /// enumerates. Returns its name.
    std::string run(int max_steps = 400) {
        boot(m_);
        s_.start();
        for (int i = 0; i < max_steps; ++i) {
            if (const char *outcome = terminal()) {
                return outcome;
            }
            if (trace_) {
                std::cout << "        [where]   " << describe() << "\n";
            }
            if (!stepOnce()) {
                return "stalled in " + describe();
            }
        }
        return "step budget exhausted in " + describe();
    }

    /// A standing instruction, as the document declares it.
    struct Rule {
        std::string question, keys, text;
    };

    /// The standing instructions, as the DOCUMENT declares them.
    ///
    /// The host used to keep its own table and `main()` filled it in, which
    /// left the document's `screen_rules` block read by nobody: the part of
    /// the file whose own comment says the loop is "carrying out a decision
    /// made in advance and written down" was written down and then never
    /// consulted. Two tables of standing instructions is one too many, and it
    /// is the wrong one that decided.
    ///
    /// It reads as JSON because that is the encoding the engine's own
    /// `JSON.stringify` produces (W3C SCXML B.2) — the same text every
    /// backend's reader hands over, so a host ported to another language
    /// parses the same bytes. A real deployment uses its JSON library; this
    /// one uses the runtime's, to keep the example free of dependencies.
    std::vector<Rule> standingInstructions() const {
        std::vector<Rule> rules;
        const auto json = m_.getPolicy().screen_rules();
        if (!json.has_value()) {
            return rules;
        }
        const auto parsed = SCE::EventDataHelper::jsonStringToScriptValue(*json);
        if (!parsed.has_value() || !std::holds_alternative<std::shared_ptr<ScriptArray>>(*parsed)) {
            return rules;
        }
        for (const ScriptValue &entry : std::get<std::shared_ptr<ScriptArray>>(*parsed)->elements) {
            if (!std::holds_alternative<std::shared_ptr<ScriptObject>>(entry)) {
                continue;
            }
            const auto &fields = std::get<std::shared_ptr<ScriptObject>>(entry)->properties;
            const auto field = [&fields](const char *key) -> std::string {
                const auto it = fields.find(key);
                return (it != fields.end() && std::holds_alternative<std::string>(it->second))
                           ? std::get<std::string>(it->second)
                           : std::string{};
            };
            rules.push_back({field("when"), field("keys"), field("text")});
        }
        return rules;
    }

private:
    bool active(State s) const {
        const auto states = m_.getPolicy().getActiveStates();
        return std::find(states.begin(), states.end(), s) != states.end();
    }

    /// A value the document owns, or a phrase saying it could not be read.
    ///
    /// The prompts belong to whoever edited the SCXML, so the host never
    /// hard-codes one — and it never reaches past the machine for one either.
    /// The generated policy carries a typed accessor per declared `<data>`
    /// (W3C SCXML 5.3), which is the whole of what a host needs: the live
    /// value, in the host's own type, with `nullopt` for the cases a consumer
    /// acts on identically — no session yet, or the variable is holding
    /// something else now.
    ///
    /// This used to be a hand-written `getVariable` against the process-wide
    /// engine provider, spelling the variable's name as a string and casting
    /// the result to `std::string` unchecked. Every part of that was a way to
    /// be wrong: the provider is not necessarily the engine this machine was
    /// handed, a mistyped name reads as "unreadable" rather than as a
    /// mistake, and a variable holding a number came back as whatever the
    /// cast made of it.
    static std::string readable(const std::optional<std::string> &value, const char *name) {
        // Silence here would look like an empty prompt rather than a missing
        // datamodel, so say which it is.
        return value.has_value() ? *value : "<" + std::string(name) + " unreadable>";
    }

    /// One turn of the host's own loop: look at where the machine is, do the
    /// thing that state implies, report the result back as an event.
    bool stepOnce() {
        if (active(State::Priming)) {
            // Two events, in this order. `priming` leaves on `prompt.sent` —
            // "the session has been told what it is here for" — and only then
            // is the machine in `working`, where a turn result means anything.
            // Reporting the turn first would deliver it to a state that does
            // not handle it, and the run would sit in `priming` forever.
            const TurnResult r = s_.prompt(readable(m_.getPolicy().start_prompt(), "start_prompt"));
            fire(Event::Prompt_sent, "prompt.sent");
            report(r);
            return true;
        }
        if (active(State::Working)) {
            report(s_.prompt(readable(m_.getPolicy().turn_prompt(), "turn_prompt")));
            return true;
        }
        if (active(State::Closing)) {
            report(s_.prompt(readable(m_.getPolicy().end_prompt(), "end_prompt")));
            return true;
        }
        if (active(State::Screening)) {
            screen();
            return true;
        }
        if (active(State::Judging)) {
            judge();
            return true;
        }
        if (active(State::Reflecting)) {
            reflect();
            return true;
        }
        if (active(State::Restarting)) {
            // The session is replaced, not reconfigured: everything reflection
            // wrote is read on the way up.
            s_.stop();
            s_.start();
            fire(Event::Session_ready, "session.ready");
            return true;
        }
        if (active(State::Paused)) {
            s_.notify();
            // A real host waits here — for a keypress, a timeout, a message.
            // The example answers at once so its output stays deterministic.
            std::cout << "        [person]  answers\n";
            fire(Event::Turn_done, "turn.done");
            return true;
        }
        return false;
    }

    /// Deliver one event, and under trace say where it left the machine. Every
    /// event the host sends goes through here so a trace can never miss one.
    ///
    /// `raiseExternal` only enqueues — the machine does not move until the
    /// queue is run — so the payload spelling needs its own `step()`. Both
    /// spellings go through this one place, which is what makes the sentence
    /// above true of an event that carries data as well as one that does not.
    void fire(Event e, const char *name, const std::string &data = "") {
        m_.raiseExternal(e, data);
        m_.step();
        if (trace_) {
            std::cout << "        [event]   " << name << " -> " << describe() << "\n";
        }
    }

    /// Report what a turn produced. One place, so every path that runs the
    /// agent tells the machine the same four things.
    void report(const TurnResult &r) {
        switch (r.kind) {
        case TurnResult::Kind::Done:
            last_text_ = r.text;
            fire(Event::Turn_done, "turn.done");
            break;
        case TurnResult::Kind::Blocked:
            last_question_ = r.question;
            fire(Event::Turn_blocked, "turn.blocked");
            break;
        case TurnResult::Kind::Interrupted:
            fire(Event::Turn_interrupted, "turn.interrupted");
            break;
        case TurnResult::Kind::Lost:
            fire(Event::Session_lost, "session.lost");
            break;
        }
    }

    /// Does a standing instruction cover the question that is up?
    ///
    /// Both halves of the answer come off the machine, because both are the
    /// author's decision written down in advance: `screen_rules` says which
    /// questions were decided, and `screen_permissions` says whether that
    /// covers a permission dialog at all. The document explains why the flag
    /// is separate — redirecting a design question costs a turn, approving a
    /// permission grants an agent the ability to act — and a host that
    /// consulted only the rules would hand out the second along with the
    /// first.
    void screen() {
        if (isPermission(last_question_) && !m_.getPolicy().screen_permissions().value_or(false)) {
            std::cout << "        [rule]    '" << last_question_
                      << "' asks for permission to act, and the document does not screen those\n";
            fire(Event::Screen_none, "screen.none");
            return;
        }
        for (const auto &rule : standingInstructions()) {
            if (rule.question == last_question_) {
                std::cout << "        [rule]    '" << rule.question << "' was decided in advance\n";
                const TurnResult r = s_.answer(rule.keys, rule.text);
                fire(Event::Screen_matched, "screen.matched");
                // The reply produced a turn of its own; report it after the
                // machine is back in `working` so it lands where it belongs.
                report(r);
                return;
            }
        }
        std::cout << "        [rule]    nothing covers '" << last_question_ << "'\n";
        fire(Event::Screen_none, "screen.none");
    }

    /// Whether a raised dialog is asking for permission to act rather than
    /// for a decision. The environment names its own dialogs; this example
    /// scripts them, so one prefix is enough to tell the two kinds apart.
    static bool isPermission(const std::string &question) {
        return question.rfind("permission", 0) == 0;
    }

    /// Did the agent say it is finished? The marker comes out of the document,
    /// so the host never hard-codes a phrase.
    void judge() {
        const std::string marker = m_.getPolicy().done_marker().value_or(std::string{});
        const bool done = !marker.empty() && last_text_.find(marker) != std::string::npos;
        // `judging` branches on `_event.data.done`, so the verdict rides the
        // event rather than a host-side flag the document cannot see.
        fire(Event::Judge, "judge", done ? R"({"done":true})" : R"({"done":false})");
    }

    /// Improve the run's own setup, then let the machine restart into it.
    ///
    /// A real host reads the transcript so far and rewrites the prompts, the
    /// session's instruction file, its memory, its tool set. All of those are
    /// read at session start, which is why the machine's only forward exit
    /// from here is a restart.
    ///
    /// The prompts sent here are COMPOSED from the refined milestone rather
    /// than left blank, and that is not cosmetic. This host used to write
    /// `{"start_prompt":"","turn_prompt":"","milestone":"refined"}`, so the
    /// document that says a restart exists to pick up the improvements came
    /// back holding two empty strings — and the fresh session was primed with
    /// nothing at all while the scenario title claimed it restarted into the
    /// improved prompts. Every run still converged, because an outcome cannot
    /// tell an empty prompt from an authored one. `main()` now reads what was
    /// actually sent.
    void reflect() {
        ++reflections_;
        const std::string milestone =
            "the smallest verifiable step toward " + readable(m_.getPolicy().north_star(), "north_star");
        std::cout << "        [reflect] rewriting the milestone from the record\n";
        fire(Event::Reflect_applied, "reflect.applied",
             R"({"start_prompt":")" +
                 jsonEscape("Resuming. Milestone: " + milestone + "\nReport what you did and what is left.") +
                 R"(","turn_prompt":")" +
                 jsonEscape("Continue toward: " + milestone +
                            "\nDo the next smallest thing that is verifiable, then report.") +
                 R"(","milestone":")" + jsonEscape(milestone) + R"("})");
    }

    /// The two characters a prompt can carry that JSON cannot hold raw.
    ///
    /// The event payload is JSON because that is what the machine parses into
    /// `_event.data`; a prompt is prose, and prose has newlines. Escaping only
    /// what this example can produce keeps the seam readable — a real host
    /// hands the job to its JSON library.
    static std::string jsonEscape(const std::string &text) {
        std::string out;
        out.reserve(text.size());
        for (const char c : text) {
            if (c == '"' || c == '\\') {
                out += '\\';
                out += c;
            } else if (c == '\n') {
                out += "\\n";
            } else {
                out += c;
            }
        }
        return out;
    }

    const char *terminal() const {
        if (active(State::Converged)) {
            return "converged";
        }
        if (active(State::Exhausted)) {
            return "exhausted";
        }
        if (active(State::Blocked)) {
            return "blocked";
        }
        if (active(State::Failed)) {
            return "failed";
        }
        if (active(State::Cancelled)) {
            return "cancelled";
        }
        return nullptr;
    }

    /// The active set, by name. Numbers would be unreadable here and a stall
    /// report is exactly where a reader needs to know which states those were.
    std::string describe() const {
        static const char *kNames[] = {"abandoned", "alive",      "blocked", "budget",     "cancelled",
                                       "closing",   "converged",  "drive",   "exhausted",  "failed",
                                       "judging",   "paused",     "priming", "rebuilding", "reflecting",
                                       "reported",  "restarting", "run",     "running",    "screening",
                                       "spent",     "stuck",      "watch",   "within",     "working"};
        std::string out;
        for (State s : m_.getPolicy().getActiveStates()) {
            const auto i = static_cast<size_t>(s);
            out += (out.empty() ? "" : " ");
            out += i < std::size(kNames) ? kNames[i] : "?";
        }
        return "[" + out + "]";
    }

public:
    /// Print the active set before every host step. Off by default so the
    /// example's normal output stays about the loop, not the trace.
    void setTrace(bool on) {
        trace_ = on;
    }

private:
    bool trace_ = false;

    Machine &m_;
    AgentSession &s_;
    std::string last_text_, last_question_;
    int reflections_ = 0;
};

TurnResult done(std::string text) {
    return TurnResult{TurnResult::Kind::Done, std::move(text), ""};
}

TurnResult blocked(std::string question) {
    return TurnResult{TurnResult::Kind::Blocked, "", std::move(question)};
}

TurnResult lost() {
    return TurnResult{TurnResult::Kind::Lost, "", ""};
}

int failures = 0;

void expect(const char *what, bool held) {
    if (!held) {
        ++failures;
    }
    std::cout << "   " << (held ? "ok   " : "FAIL ") << what << "\n";
}

/// The document a host edits is the document it can read back.
///
/// This is the C++ half of a claim its Rust sibling also makes — see
/// `the_strategy_a_host_edits_is_the_strategy_it_can_read_back` and
/// `the_standing_instructions_can_be_read_back_off_the_machine` in
/// `backends/rust/tests/tests/ai_loop.rs`. Asserting it on both engines is
/// what `scripts/regen_ai_loop.sh` says this pair is for: a clause only one
/// engine honours has to fail on the other. The topology clauses were paired
/// that way from the start; the datamodel ones were not, and for a while the
/// Rust side asserted a readable document while this side reached around the
/// machine for the same values by hand.
void readback() {
    std::cout << "\n-- the document a host edits is the document it can read back\n";
    Machine machine;
    boot(machine);
    ScriptedSession idle({});
    const LoopHost host(machine, idle);
    const auto &p = machine.getPolicy();

    // The marker a host matches the agent's report against, and the budget it
    // reports progress out of. Neither can be hard-coded by a supervisor that
    // did not write the document.
    expect("done_marker reads back", p.done_marker() == std::optional<std::string>("MILESTONE REACHED"));
    expect("max_turns reads back", p.max_turns() == std::optional<int64_t>(40));
    expect("north_star reads back", p.north_star().value_or("").find("(edit me)") != std::string::npos);

    // The standing instructions: the block that decides when a person is NOT
    // woken. A host that cannot list it cannot show anyone which decisions are
    // standing, which is the difference between carrying out a decision and
    // making one.
    const auto rules = host.standingInstructions();
    expect("screen_rules lists every standing instruction", rules.size() == 3);
    const bool named = std::all_of(rules.begin(), rules.end(), [](const LoopHost::Rule &r) {
        return !r.question.empty() && !r.keys.empty() && !r.text.empty();
    });
    expect("each standing instruction carries its question, keys and reply", named);

    // Separate from the rules on purpose, and read separately for the same
    // reason: a standing yes to acting is a different promise.
    expect("screen_permissions reads back", p.screen_permissions() == std::optional<bool>(false));
}

/// Run one scripted transcript against a fresh machine.
///
/// There is no "with rules" switch any more, and its absence is the point:
/// the standing instructions are the document's, so which questions get
/// answered without waking anybody is decided by editing `ai_loop.scxml` and
/// nowhere else. A scenario picks the question the agent raises; the document
/// decides what happens to it.
/// `after` runs once the machine has stopped, on the session it drove.
///
/// An outcome is a weak observable: `converged` says the document reached the
/// final it enumerates, not that anything sensible was sent on the way. A
/// scenario with something to say about the transcript says it here.
void scenario(const std::string &title, std::vector<TurnResult> script, const std::string &expected,
              const std::function<void(const ScriptedSession &)> &after = nullptr) {
    std::cout << "\n-- " << title << "\n";
    ScriptedSession session(std::move(script));
    Machine machine;
    LoopHost host(machine, session);
    if (std::getenv("AI_LOOP_TRACE") != nullptr) {
        host.setTrace(true);
    }
    const std::string outcome = host.run();
    const bool ok = outcome == expected;
    if (!ok) {
        ++failures;
    }
    std::cout << "   => " << outcome << (ok ? "   OK" : "   UNEXPECTED, wanted " + expected)
              << "   (sessions started: " << session.starts() << ")\n";
    if (after) {
        after(session);
    }
}

}  // namespace

int main() {
    SCE::JSEngine::instance().initialize();

    std::cout << "ai_loop - a statechart supervising an agent\n"
              << "The host below performs; the document decides.\n";

    readback();

    // 1. Nine turns of work. The eighth trips `reflect_every`, so the loop
    //    reflects and restarts; the tenth reports the milestone.
    {
        std::vector<TurnResult> script;
        for (int i = 0; i < 9; ++i) {
            script.push_back(done("still working"));
        }
        script.push_back(done("MILESTONE REACHED"));
        script.push_back(done("final report"));
        scenario("works, reflects on schedule, restarts into the improved prompts", std::move(script), "converged",
                 [](const ScriptedSession &session) {
                     // The claim in this scenario's own title, checked. The
                     // reflection rewrote the machine's prompts; the restart
                     // exists so a fresh session reads them. What proves it is
                     // the text that went out AFTER the restart — nine turns of
                     // work, then the reflection, so the tenth prompt is the
                     // first one a restarted session received.
                     const auto &sent = session.prompts();
                     const bool enough = sent.size() > 9;
                     expect("the restarted session was prompted", enough);
                     if (!enough) {
                         return;
                     }
                     const std::string &primed = sent[9];
                     expect("the restarted session was not primed with an empty prompt", !primed.empty());
                     expect("the prompt it was primed with carries the refined milestone",
                            primed.find("the smallest verifiable step toward") != std::string::npos);
                 });
    }

    // 2. The agent asks something the author already decided — `screen_rules`
    //    names this question — so nobody is woken.
    scenario("a question a standing instruction already answers",
             {done("working"), blocked("design-decision"), done("MILESTONE REACHED"), done("final report")},
             "converged");

    // 3. A dialog the document does NOT screen. `screen_permissions` is false,
    //    and the document says why: a standing "yes" to acting is a different
    //    promise from a standing "think about it again". A person is woken.
    scenario("a permission dialog the document refuses to answer for anyone",
             {done("working"), blocked("permission.write_files"), done("MILESTONE REACHED"), done("final report")},
             "converged");

    // 4. The session dies mid-turn. `watch` sees it even though the cycle was
    //    waiting on a turn that will never arrive.
    scenario("a session that dies mid-turn is rebuilt",
             {done("working"), lost(), done("MILESTONE REACHED"), done("final report")}, "converged");

    std::cout << "\n"
              << (failures == 0 ? "The document is readable, every run ended in an outcome it enumerates, and the\n"
                                  "restart was primed with what reflection wrote."
                                : "SOMETHING THE DOCUMENT DECLARES DID NOT HOLD.")
              << "\n";

    SCE::JSEngine::instance().shutdown();
    return failures == 0 ? 0 : 1;
}
