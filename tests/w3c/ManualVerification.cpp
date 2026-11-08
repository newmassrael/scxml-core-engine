// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "ManualVerification.h"
#include <algorithm>
#include <array>
#include <sstream>

namespace SCE::W3C {

namespace {

ValidationResult passed(const std::string &reason) {
    return ValidationResult(true, TestResult::PASS, reason);
}

ValidationResult failed(const std::string &reason) {
    return ValidationResult(false, TestResult::FAIL, reason);
}

/// A fixture that has to run was refused at load: the run says nothing about
/// the clause it tests
ValidationResult rejectedAtLoad(const ManualRun &run) {
    return ValidationResult(false, TestResult::ERROR,
                            "Failed to load SCXML content" +
                                (run.loadError.empty() ? std::string() : ": " + run.loadError));
}

/// The verdicts that read the <log> output cannot be given without it
ValidationResult logNotCaptured() {
    return ValidationResult(false, TestResult::ERROR,
                            "the logger captured nothing during the run, so the <log> output the verdict reads is "
                            "unknown (a logger backend without capture)");
}

/// The run was refused at load or still running when the timeout ended it, or
/// it halted somewhere other than `state`; none when it halted in `state`
std::optional<ValidationResult> unlessHaltedIn(const ManualRun &run, const std::string &state) {
    if (!run.loaded) {
        return rejectedAtLoad(run);
    }
    if (!run.halted) {
        return failed("Timeout: StateMachine did not halt within the run's timeout (current state: " + run.endState +
                      ")");
    }
    if (run.endState != state) {
        return failed("halted in '" + run.endState + "', expected '" + state + "'");
    }
    return std::nullopt;
}

/// The one message logged under `label`: none when nothing was, and none with
/// `ambiguous` set when more than one was
std::optional<std::string> theValueLoggedUnder(const ManualRun &run, const std::string &label, bool &ambiguous) {
    const std::vector<std::string> values = valuesLoggedUnder(run.logMessages, label);
    ambiguous = values.size() > 1;
    if (values.size() != 1) {
        return std::nullopt;
    }
    return values.front();
}

std::string quoted(const std::optional<std::string> &value) {
    return value ? "'" + *value + "'" : std::string("nothing (the expression raised an error)");
}

// §scxml-6.2: the sender's two <param>s of one name reach the receiver as
// {"Var1":["2","3"]}, and the receiving transition's guard leads to `final`
// only when both values arrived; `fail` when they did not.
ValidationResult judge178(const ManualRun &run) {
    if (!run.loaded) {
        return rejectedAtLoad(run);
    }
    if (run.endState == "final") {
        return passed("W3C SCXML 6.2: param preserves duplicate keys with multiple values");
    }
    if (run.endState == "fail") {
        return failed("W3C SCXML 6.2 violation: StateMachine reached fail state (event1 not received)");
    }
    return failed("Timeout: StateMachine did not complete within the run's timeout (current state: " + run.endState +
                  ")");
}

// §scxml-6.4: autoforward sends the invoked session an exact copy of every
// external event the parent takes. The child sends `childToParent` to the
// parent, which forwards it back; the parent (in s01) and the child (in sub0)
// each log the seven fields of their copy under the same seven labels, and the
// parent halts in `final` once the child is done.
ValidationResult judge230(const ManualRun &run) {
    if (auto verdict = unlessHaltedIn(run, "final")) {
        return *verdict;
    }
    if (!run.logCaptured) {
        return logNotCaptured();
    }
    static const std::array<const char *, 7> fieldLabels = {
        "name is ", "type is ", "sendid is ", "origin is ", "origintype is ", "invokeid is ", "data is ",
    };
    for (const char *label : fieldLabels) {
        const std::vector<std::string> values = valuesLoggedUnder(run.logMessages, label);
        if (values.size() != 2) {
            return failed("the field logged as '" + std::string(label) + "' was logged " +
                          std::to_string(values.size()) +
                          " time(s); the parent and the child each log their copy of childToParent once");
        }
        if (values[0] != values[1]) {
            return failed("the forwarded copy of childToParent differs: '" + std::string(label) + "' logged '" +
                          values[0] + "' and '" + values[1] + "'");
        }
    }
    return passed("W3C SCXML 6.4: the autoforwarded copy of childToParent logged the same seven fields as the "
                  "original");
}

// §scxml-6.4: leaving s0 cancels the session it invoked, and the cancelled
// session runs the <onexit> of each of its active states, innermost first
// (§scxml-D-exitInterpreter): "Exiting sub01", then "Exiting sub0". Reaching
// its own final state means it was never cancelled; the run is watched past
// that session's 2s timeout (watchAtLeast) so a session left running gets the
// time to say so.
ValidationResult judge250(const ManualRun &run) {
    if (auto verdict = unlessHaltedIn(run, "final")) {
        return *verdict;
    }
    if (!run.logCaptured) {
        return logNotCaptured();
    }
    const auto at = [&run](const std::string &message) -> std::optional<std::size_t> {
        const auto found = std::find(run.logMessages.begin(), run.logMessages.end(), message);
        if (found == run.logMessages.end()) {
            return std::nullopt;
        }
        return static_cast<std::size_t>(found - run.logMessages.begin());
    };
    if (at("entering final state, invocation was not cancelled")) {
        return failed("the invoked session entered subFinal: it was not cancelled when the parent left s0");
    }
    const std::optional<std::size_t> innerExit = at("Exiting sub01");
    const std::optional<std::size_t> outerExit = at("Exiting sub0");
    if (!innerExit) {
        return failed("the cancelled session did not log 'Exiting sub01': the <onexit> of sub01 did not run");
    }
    if (!outerExit) {
        return failed("the cancelled session did not log 'Exiting sub0': the <onexit> of sub0 did not run");
    }
    if (*outerExit < *innerExit) {
        return failed("the cancelled session exited sub0 before sub01; the innermost state is exited first");
    }
    return passed("W3C SCXML 6.4: the cancelled session ran the <onexit> of sub01, then of sub0");
}

// §scxml-5.8: a <script> whose src cannot be downloaded makes the document
// non-conformant, and the platform must reject it. A run at all is the failure.
ValidationResult judge301(const ManualRun &run) {
    if (!run.loaded) {
        return passed("W3C SCXML 5.8: the document was rejected at load" +
                      (run.loadError.empty() ? std::string() : " (" + run.loadError + ")"));
    }
    return failed("the document was loaded and run (it ended in '" + run.endState +
                  "'); a <script> whose src cannot be downloaded must make the platform reject it");
}

// §scxml-5.9: with binding="late", reading Var1 in s0, before s1's <data>
// creates it, must behave as reading a missing part of a loaded value
// (Var1.foo in s1) does. Each state logs the value it read, then whether an
// error was raised; the two must agree on both.
ValidationResult judge307(const ManualRun &run) {
    if (auto verdict = unlessHaltedIn(run, "final")) {
        return *verdict;
    }
    if (!run.logCaptured) {
        return logNotCaptured();
    }

    // What one state logged: the value it read (none when reading it raised an
    // error), and whether an error followed
    struct Reading {
        std::optional<std::string> value;
        bool error{false};
    };

    const auto readingOf = [&run](const std::string &state, const std::string &valueLabel,
                                  const std::string &errorLabel, const std::string &noErrorLabel,
                                  std::optional<std::string> &problem) -> Reading {
        Reading reading;
        bool ambiguous = false;
        reading.value = theValueLoggedUnder(run, valueLabel, ambiguous);
        if (ambiguous) {
            problem = state + " logged its value more than once";
            return reading;
        }
        const std::size_t errors = valuesLoggedUnder(run.logMessages, errorLabel).size();
        const std::size_t noErrors = valuesLoggedUnder(run.logMessages, noErrorLabel).size();
        if (errors + noErrors != 1) {
            problem = state + " logged '" + errorLabel + "' " + std::to_string(errors) + " time(s) and '" +
                      noErrorLabel + "' " + std::to_string(noErrors) + " time(s), expected one of them once";
            return reading;
        }
        reading.error = errors == 1;
        return reading;
    };

    std::optional<std::string> problem;
    const Reading early =
        readingOf("s0", "entering s0 value of Var 1 is: ", "error in state s0", "no error in s0", problem);
    if (problem) {
        return failed(*problem);
    }
    const Reading missing =
        readingOf("s1", "entering s1, value of non-existent substructure of Var 1 is: ", "error in state s1",
                  "No error in s1", problem);
    if (problem) {
        return failed(*problem);
    }

    const auto outcome = [](const Reading &reading) { return std::string(reading.error ? "an error" : "no error"); };
    if (early.value != missing.value || early.error != missing.error) {
        return failed("Var1 before it was loaded read as " + quoted(early.value) + " with " + outcome(early) +
                      ", and a missing part of it read as " + quoted(missing.value) + " with " + outcome(missing));
    }
    return passed("W3C SCXML 5.9: Var1 before it was loaded and a missing part of it both read as " +
                  quoted(early.value) + " with " + outcome(early));
}

/// §scxml-5.9: the platform may reject a document holding an illegal
/// expression at load; a run of it must raise error.execution where the
/// expression is evaluated, which the document turns into reaching `pass`
ValidationResult rejectedOrPassed(const ManualRun &run, const std::string &passMeans) {
    if (!run.loaded) {
        return passed("W3C SCXML 5.9: the document was rejected at load, which the clause allows" +
                      (run.loadError.empty() ? std::string() : " (" + run.loadError + ")"));
    }
    if (!run.halted) {
        return failed("Timeout: StateMachine did not halt within the run's timeout (current state: " + run.endState +
                      ")");
    }
    if (run.endState != "pass") {
        return failed("the document ran and halted in '" + run.endState + "'; " + passMeans);
    }
    return passed("W3C SCXML 5.9: the document ran and reached pass: " + passMeans);
}

// §scxml-5.9: s0's <assign> evaluates an illegal expression; error.execution
// takes s0 to `pass`, any other event (foo) to `fail`.
ValidationResult judge313(const ManualRun &run) {
    return rejectedOrPassed(run, "error.execution is raised when the illegal expression is evaluated");
}

// §scxml-5.9: the illegal expression is evaluated in s03's <onentry> only, so
// error.execution must arrive there (s03 goes to `pass`). An error raised
// earlier is taken by s0's own error.execution transition, to `fail`.
ValidationResult judge314(const ManualRun &run) {
    return rejectedOrPassed(run, "error.execution is raised only when s03 evaluates the illegal expression");
}

// §scxml-3.13: a machine that enters a top-level final halts, and processes no
// event after it: the initial state is a top-level final whose <onentry>
// raises event1, which must never be processed.
//
// The observable is the count of events the machine processed
// (StateMachine::Statistics::totalEvents): every event the Interpreter takes
// off either queue passes StateMachine::processTakenEvent, which counts it,
// and the initial configuration is entered without one, so any count is an
// event processed after the halt. It is read from the machine rather than
// inferred: the document logs nothing, and the internal queue says event1 was
// raised but not whether it was taken.
ValidationResult judge415(const ManualRun &run) {
    if (auto verdict = unlessHaltedIn(run, "final")) {
        return *verdict;
    }
    if (run.eventsProcessed != 0) {
        return failed("the machine processed " + std::to_string(run.eventsProcessed) +
                      " event(s) after it entered the top-level final; event1, raised in its <onentry>, must never "
                      "be processed");
    }
    return passed("W3C SCXML 3.13: the machine halted in the top-level final and processed no event");
}

}  // namespace

const std::map<std::string, ManualVerifier> &manualVerifiers() {
    // A run that is over when the machine halts
    constexpr std::chrono::milliseconds untilItHalts{0};
    // 250's invoked session sends itself `timeout` 2s after it starts, and
    // reaches its own final state on it unless it was cancelled; the watch
    // outlasts that by half a second.
    constexpr std::chrono::milliseconds pastTheCancelledSessionsTimeout{2500};

    static const std::map<std::string, ManualVerifier> verifiers = {
        {"178", {&judge178, untilItHalts}},                     // duplicate <param> names
        {"230", {&judge230, untilItHalts}},                     // an autoforwarded copy's fields
        {"250", {&judge250, pastTheCancelledSessionsTimeout}},  // a cancelled session's <onexit>
        {"301", {&judge301, untilItHalts}},                     // a <script> src that cannot be read
        {"307", {&judge307, untilItHalts}},                     // late binding
        {"313", {&judge313, untilItHalts}},                     // an illegal expression
        {"314", {&judge314, untilItHalts}},                     // where an illegal expression is evaluated
        {"415", {&judge415, untilItHalts}},                     // halting in a top-level final
    };
    return verifiers;
}

const ManualVerifier *manualVerifierFor(const std::string &fixtureId) {
    const auto &verifiers = manualVerifiers();
    const auto found = verifiers.find(fixtureId);
    return found == verifiers.end() ? nullptr : &found->second;
}

std::vector<std::string> logMessagesIn(const std::string &capturedLogs) {
    static const std::string marker = "() - SCXML: ";
    std::vector<std::string> messages;
    std::istringstream lines(capturedLogs);
    std::string line;
    while (std::getline(lines, line)) {
        const std::size_t at = line.find(marker);
        if (at != std::string::npos) {
            messages.push_back(line.substr(at + marker.size()));
        }
    }
    return messages;
}

std::vector<std::string> valuesLoggedUnder(const std::vector<std::string> &logMessages, const std::string &label) {
    const std::string prefix = label + ": ";
    std::vector<std::string> values;
    for (const std::string &message : logMessages) {
        if (message == label) {
            values.emplace_back();
        } else if (message.compare(0, prefix.size(), prefix) == 0) {
            values.push_back(message.substr(prefix.size()));
        }
    }
    return values;
}

}  // namespace SCE::W3C
