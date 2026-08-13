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
#include <iostream>
#include <iterator>
#include <memory>
#include <string>
#include <vector>

#include "ai_loop_sm.h"
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
        return next();
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
    size_t cursor_ = 0;
    int starts_ = 0;
};

// ── the host ──────────────────────────────────────────────────────────

class LoopHost {
public:
    LoopHost(Machine &machine, AgentSession &session) : m_(machine), s_(session) {}

    /// A standing instruction: the host's side of the document's
    /// `screen_rules`. The machine decides THAT a dialog is screened; this
    /// decides WHICH reply a given question gets.
    void addRule(std::string question, std::string keys, std::string text) {
        rules_.push_back({std::move(question), std::move(keys), std::move(text)});
    }

    /// Run until the machine reaches one of the outcomes the document
    /// enumerates. Returns its name.
    std::string run(int max_steps = 400) {
        // W3C SCXML B.1: the machine's datamodel is ECMAScript, so it needs a
        // script engine handed to it before it starts. Aliasing constructor
        // with a no-op deleter — the provider owns the engine's lifetime and
        // this shared_ptr is a non-owning view, the same idiom the W3C AOT
        // harness uses.
        m_.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                               [](SCE::IScriptEngine *) {}));
        m_.initialize();
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

private:
    struct Rule {
        std::string question, keys, text;
    };

    bool active(State s) const {
        const auto states = m_.getPolicy().getActiveStates();
        return std::find(states.begin(), states.end(), s) != states.end();
    }

    /// The script-engine session the machine's datamodel lives in. Created
    /// lazily by the machine on first use, so this is read after
    /// `initialize()`.
    std::string sessionId() const {
        const auto &id = m_.getPolicy().sessionId_;
        return id.has_value() ? *id : std::string{};
    }

    /// Read a datamodel string out of the document. The prompts belong to
    /// whoever edited the SCXML, so the host never hard-codes one.
    std::string data(const std::string &name) {
        const std::string session = sessionId();
        if (session.empty()) {
            // Silence here would look like an empty prompt rather than a
            // missing datamodel, so say which it is.
            return "<no script session; datamodel unreadable>";
        }
        // The SAME engine the machine was handed. Reading from a different
        // instance finds a session that exists but holds none of this
        // document's variables.
        auto result = SCE::ScriptEngineProvider::getScriptEngine().getVariable(session, name).get();
        if (!result.isSuccess()) {
            return "<" + name + " unreadable>";
        }
        return result.template getValue<std::string>();
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
            const TurnResult r = s_.prompt(data("start_prompt"));
            fire(Event::Prompt_sent, "prompt.sent");
            report(r);
            return true;
        }
        if (active(State::Working)) {
            report(s_.prompt(data("turn_prompt")));
            return true;
        }
        if (active(State::Closing)) {
            report(s_.prompt(data("end_prompt")));
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
    void screen() {
        for (const auto &rule : rules_) {
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

    /// Did the agent say it is finished? The marker comes out of the document,
    /// so the host never hard-codes a phrase.
    void judge() {
        const std::string marker = data("done_marker");
        const bool done = !marker.empty() && last_text_.find(marker) != std::string::npos;
        // `judging` branches on `_event.data.done`, so the verdict rides the
        // event rather than a host-side flag the document cannot see.
        fire(Event::Judge, "judge", done ? R"({"done":true})" : R"({"done":false})");
    }

    /// Improve the run's own setup, then let the machine restart into it.
    ///
    /// A real host reads the transcript so far and rewrites the prompts, the
    /// agent's instruction file, its memory, its tool set. All of those are
    /// read at session start, which is why the machine's only forward exit
    /// from here is a restart.
    void reflect() {
        ++reflections_;
        std::cout << "        [reflect] rewriting the milestone from the record\n";
        fire(Event::Reflect_applied, "reflect.applied",
             R"({"start_prompt":"","turn_prompt":"","milestone":"refined"})");
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
    std::vector<Rule> rules_;
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

void scenario(const std::string &title, std::vector<TurnResult> script, bool with_rules, const std::string &expected) {
    std::cout << "\n-- " << title << "\n";
    ScriptedSession session(std::move(script));
    Machine machine;
    LoopHost host(machine, session);
    if (with_rules) {
        host.addRule("design-decision", "Escape", "Ignore cost. Rethink for the most durable answer, then proceed.");
    }
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
}

}  // namespace

int main() {
    SCE::JSEngine::instance().initialize();

    std::cout << "ai_loop - a statechart supervising an agent\n"
              << "The host below performs; the document decides.\n";

    // 1. Nine turns of work. The eighth trips `reflect_every`, so the loop
    //    reflects and restarts; the tenth reports the milestone.
    {
        std::vector<TurnResult> script;
        for (int i = 0; i < 9; ++i) {
            script.push_back(done("still working"));
        }
        script.push_back(done("MILESTONE REACHED"));
        script.push_back(done("final report"));
        scenario("works, reflects on schedule, restarts into the improved prompts", std::move(script), false,
                 "converged");
    }

    // 2. The agent asks something the author already decided. Nobody is woken.
    scenario("a question a standing instruction already answers",
             {done("working"), blocked("design-decision"), done("MILESTONE REACHED"), done("final report")}, true,
             "converged");

    // 3. The same question with no rule for it. A person is woken and answers.
    scenario("a question nothing covers - a person is woken",
             {done("working"), blocked("design-decision"), done("MILESTONE REACHED"), done("final report")}, false,
             "converged");

    // 4. The session dies mid-turn. `watch` sees it even though the cycle was
    //    waiting on a turn that will never arrive.
    scenario("a session that dies mid-turn is rebuilt",
             {done("working"), lost(), done("MILESTONE REACHED"), done("final report")}, false, "converged");

    std::cout << "\n"
              << (failures == 0 ? "Every run ended in an outcome the document enumerates."
                                : "SOME RUNS ENDED SOMEWHERE THE DOCUMENT DID NOT EXPECT.")
              << "\n";

    SCE::JSEngine::instance().shutdown();
    return failures == 0 ? 0 : 1;
}
