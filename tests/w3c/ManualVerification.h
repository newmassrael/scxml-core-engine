// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "interfaces/ITestResultValidator.h"
#include <chrono>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace SCE::W3C {

/**
 * @brief What one Interpreter run of a manual W3C fixture showed
 *
 * A fixture whose metadata says `manual: True` names an observable its final
 * state cannot carry: values two sessions log, an event that must never be
 * processed, a document the platform must refuse. The runner records each of
 * them here (W3CTestRunner::runManualFixture) and the fixture's verifier judges
 * them (manualVerifierFor), so the judgment is a pure function of the run.
 */
struct ManualRun {
    /// The Interpreter accepted the document; false when it rejected it at load
    bool loaded{false};

    /// Why the document was rejected (StateMachine::getLastLoadError), empty
    /// when it loaded or when the loader gave no detail
    std::string loadError;

    /// The machine had stopped running when the watch ended; false when the
    /// run's timeout ended it
    bool halted{false};

    /// The top-level final the run halted in, or none
    std::optional<std::string> terminalState;

    /// The terminal state, or the active state when the timeout ended the run
    std::string endState;

    /// Events the machine took off its queues and processed, from either
    /// queue (StateMachine::Statistics::totalEvents)
    int eventsProcessed{0};

    /// The log the run was watched under held anything at all. A logger
    /// backend that cannot capture (ILoggerBackend::startCapture is a no-op by
    /// default) leaves it empty, and then the <log> output is unknown rather
    /// than absent.
    bool logCaptured{false};

    /// The message of every <log> the run executed, its invoked sessions'
    /// included, in the order they were logged (logMessagesIn)
    std::vector<std::string> logMessages;
};

/// The verdict on one run of a manual fixture
using ManualJudge = ValidationResult (*)(const ManualRun &run);

/**
 * @brief How a manual fixture is run, and how its run is judged
 */
struct ManualVerifier {
    ManualJudge judge;

    /// How long the run is watched from its start, even after the machine
    /// halts: what a verifier judges can happen after the halt (the delayed
    /// event of an invoked session that should have been cancelled). Zero for
    /// a run that is over when the machine halts.
    std::chrono::milliseconds watchAtLeast;
};

/**
 * @brief The verifier of every manual fixture the runner judges, by fixture ID
 *
 * A manual fixture absent from this table is not run, and is reported as a
 * skip under its own ID (W3CTestRunner::runManualFixture), so a manual fixture
 * registered without a verifier shows in every tally instead of passing.
 */
const std::map<std::string, ManualVerifier> &manualVerifiers();

/**
 * @brief The verifier of one fixture, by its full ID ("230"), or null
 */
const ManualVerifier *manualVerifierFor(const std::string &fixtureId);

/**
 * @brief The <log> messages a captured log holds, in order
 *
 * SCE::Logger writes a message as "<function>() - <message>"
 * (Logger::logAt), and ActionExecutorImpl::log writes the message of a <log>
 * as "SCXML: <message>". Each captured entry is one line, so a message that
 * itself holds a line break is read up to that break.
 *
 * @param capturedLogs What SCE::Logger::getCapturedLogs returned
 */
std::vector<std::string> logMessagesIn(const std::string &capturedLogs);

/**
 * @brief The values logged under one label, in order
 *
 * ActionExecutorImpl::executeLogAction writes a labelled message as
 * `label + ": " + value`, so the value is what follows that prefix, and the
 * message of a <log> with a label and no expr as the label alone, whose value
 * reads as empty.
 *
 * @param logMessages The run's <log> messages (logMessagesIn)
 * @param label The `label` attribute of the <log>, exactly as the document has it
 */
std::vector<std::string> valuesLoggedUnder(const std::vector<std::string> &logMessages, const std::string &label);

}  // namespace SCE::W3C
