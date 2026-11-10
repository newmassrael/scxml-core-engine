// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of a generated suite (docs/SCE_TTCN3_LANE.md §4, §10.5):
// it runs every test case a world selects against its recorded input trace
// and writes each one's canonical test log.
//
// The unit of a suite generates the program's `main`
// (`SCE::Generated::<Suite>::main`), which hands Runner::main() the suite's
// tables and the reader of its module parameters
// (`SCE::Generated::<Suite>::moduleParameter`); a build makes it a program
// with a one-line `main` that calls it. The command line is the same on every
// backend:
//
//   <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
//
// - `--trace-dir`: the directory of the traces, `<test case>.json` each
//   (`schemas/sce-test-trace.v1.schema.json`);
// - `--log-dir`: the directory the logs are written to, `<test case>.log.json`
//   each, the canonical test log's text with nothing after it
//   (`schemas/sce-test-log.v1.schema.json`), made where it is missing;
// - `--world`: the features the world enables, by name, separated by commas;
//   none where it is not given (ADR 0057 §2);
// - `--module-parameter`: a module parameter's value in place of its default,
//   written as `<sce:module-parameter value>` writes a literal of its type
//   (docs/SCE_TEST_PROFILE.md §11.4, §11.8); repeated, one per parameter.
//
// Each test case run prints `<test case> <verdict>` on its own line. A test
// case whose trace is missing or refused ends with `error` and its log is
// written, the reason on the standard error (P13). The exit status is 0 when
// every selected test case ran and its log was written, whatever its verdict:
// the verdict is the log's. It is 2 when the command line is not one of the
// above, a module parameter's value is refused, or a log cannot be written.
// The Rust sibling is `sce_rust_runtime::test::runner`, whose command line,
// output and refusals these are, word for word.

#pragma once

#include <cerrno>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <ios>
#include <ostream>
#include <sstream>
#include <stdexcept>
#include <streambuf>
#include <string>
#include <system_error>
#include <utility>
#include <vector>

#include "saved/SavedJson.h"
#include "saved/SavedValues.h"
#include "test/FromCanonical.h"
#include "test/ModuleParameters.h"
#include "test/Suite.h"
#include "test/Trace.h"
#include "test/Verdict.h"

namespace SCE::Test::Runner {

/// The reader of a suite's module parameters, generated as
/// `SCE::Generated::<Suite>::moduleParameter`: the saved value of the module
/// parameter @p id of its declared type, read from @p literal, the value the
/// command line gives it; std::invalid_argument, naming why, where it is not
/// one.
using ParameterReader = ModuleParameterValue (*)(const std::string &id, const std::string &literal);

/// The program's usage line.
inline constexpr const char *USAGE = "usage: <runner> --trace-dir <dir> --log-dir <dir> "
                                     "[--world <feature>,...] [--module-parameter <id>=<value>]...";

/// What the command line asks of the runner.
struct RunnerOptions {
    /// The directory of the traces, `<test case>.json` each.
    std::filesystem::path traceDir;
    /// The directory the logs are written to, `<test case>.log.json` each.
    std::filesystem::path logDir;
    /// The features the world enables.
    Features world;
    /// Each `--module-parameter`, its id and the literal it gives, in the
    /// order given.
    std::vector<std::pair<std::string, std::string>> moduleParameters;

    /// The options @p args give, the program's name not among them; refused
    /// with std::invalid_argument, naming why, where they are not the
    /// program's command line.
    static RunnerOptions parse(const std::vector<std::string> &args) {
        RunnerOptions options;
        bool traceDirGiven = false;
        bool logDirGiven = false;
        bool worldGiven = false;
        std::string world;
        // Every option takes one value: the line is read a pair at a time.
        for (std::size_t at = 0; at < args.size(); at += 2) {
            const std::string &option = args[at];
            if (at + 1 == args.size()) {
                throw std::invalid_argument(option + " is given no value");
            }
            const std::string &value = args[at + 1];
            if (option == "--trace-dir" && !traceDirGiven) {
                options.traceDir = value;
                traceDirGiven = true;
            } else if (option == "--log-dir" && !logDirGiven) {
                options.logDir = value;
                logDirGiven = true;
            } else if (option == "--world" && !worldGiven) {
                world = value;
                worldGiven = true;
            } else if (option == "--module-parameter") {
                const std::size_t equals = value.find('=');
                if (equals == std::string::npos) {
                    throw std::invalid_argument("--module-parameter " + value + " is not <id>=<value>");
                }
                std::string id = value.substr(0, equals);
                for (const auto &given : options.moduleParameters) {
                    if (given.first == id) {
                        throw std::invalid_argument("the module parameter '" + id + "' is given twice");
                    }
                }
                options.moduleParameters.emplace_back(std::move(id), value.substr(equals + 1));
            } else if (option == "--trace-dir" || option == "--log-dir" || option == "--world") {
                throw std::invalid_argument(option + " is given twice");
            } else {
                throw std::invalid_argument(option + " is no option of the runner");
            }
        }
        if (!traceDirGiven) {
            throw std::invalid_argument("--trace-dir is not given");
        }
        if (!logDirGiven) {
            throw std::invalid_argument("--log-dir is not given");
        }
        std::stringstream features(world);
        std::string feature;
        while (std::getline(features, feature, ',')) {
            if (!feature.empty()) {
                options.world.insert(feature);
            }
        }
        return options;
    }
};

/// The trace of the test case @p testCase, `<dir>/<test case>.json`; refused
/// with TraceRefusal, naming why, where it is not read or not a trace.
inline Trace readTrace(const std::filesystem::path &dir, const std::string &testCase) {
    const std::filesystem::path path = dir / (testCase + ".json");
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        throw TraceRefusal(path.string() + " is not read: " + std::strerror(errno));
    }
    std::ostringstream text;
    text << in.rdbuf();
    if (in.bad()) {
        throw TraceRefusal(path.string() + " is not read: " + std::strerror(errno));
    }
    return Trace::parse(text.str());
}

/// The runner program of @p suite, its module parameters read by
/// @p parameter, on the command line @p args, the program's name not among
/// them (docs/SCE_TTCN3_LANE.md §4), which prints each test case's verdict on
/// @p out and each reason on @p err: see the header's description. Returns
/// the exit status.
inline int main(const Suite &suite, ParameterReader parameter, const std::vector<std::string> &args, std::ostream &out,
                std::ostream &err) {
    const auto usage = [&err](const std::string &why) {
        err << why << "\n" << USAGE << "\n";
        return 2;
    };
    RunnerOptions options;
    try {
        options = RunnerOptions::parse(args);
    } catch (const std::invalid_argument &refused) {
        return usage(refused.what());
    }
    ModuleParameters overrides;
    for (const auto &[id, literal] : options.moduleParameters) {
        try {
            overrides.set(id, parameter(id, literal));
        } catch (const std::invalid_argument &refused) {
            return usage("--module-parameter " + id + "=" + literal + ": " + refused.what());
        }
    }
    std::vector<CaseResult> results;
    try {
        results = runSuiteWithTraces(
            suite, options.world,
            [&options](const std::string &testCase) { return readTrace(options.traceDir, testCase); }, overrides);
    } catch (const std::invalid_argument &refused) {
        return usage(refused.what());
    }
    std::error_code made;
    std::filesystem::create_directories(options.logDir, made);
    if (made) {
        err << options.logDir.string() << " is not made: " << made.message() << "\n";
        return 2;
    }
    for (const CaseResult &result : results) {
        for (const LogRecord &line : result.result.log) {
            if (line.kind == LogKind::Error) {
                err << result.testCase << ": " << line.component << ": " << line.detail << "\n";
            }
        }
        const std::filesystem::path path = options.logDir / (result.testCase + ".log.json");
        std::ofstream file(path, std::ios::binary | std::ios::trunc);
        file << result.canonicalLog(suite.name);
        file.close();
        if (!file) {
            err << path.string() << " is not written\n";
            return 2;
        }
        out << result.testCase << " " << verdictName(result.result.verdict) << "\n";
    }
    out.flush();
    return 0;
}

namespace detail {

/// A stream buffer that writes through to a C stream, so the program writes
/// its standard output and its standard error through std::ostream without
/// naming the standard library's own stream objects.
class CStreamBuffer : public std::streambuf {
public:
    explicit CStreamBuffer(std::FILE *file) : file_(file) {}

protected:
    int_type overflow(int_type character) override {
        if (traits_type::eq_int_type(character, traits_type::eof())) {
            return traits_type::not_eof(character);
        }
        return std::fputc(character, file_) == EOF ? traits_type::eof() : character;
    }

    std::streamsize xsputn(const char *text, std::streamsize count) override {
        return static_cast<std::streamsize>(std::fwrite(text, 1, static_cast<std::size_t>(count), file_));
    }

    int sync() override {
        return std::fflush(file_) == 0 ? 0 : -1;
    }

private:
    std::FILE *file_;
};

}  // namespace detail

/// The runner program of @p suite, its module parameters read by
/// @p parameter, on the command line of a program's `main`: main() above,
/// which prints on the standard output and the standard error.
inline int main(const Suite &suite, ParameterReader parameter, int argc, char **argv) {
    std::vector<std::string> args;
    for (int at = 1; at < argc; ++at) {
        args.emplace_back(argv[at]);
    }
    detail::CStreamBuffer standardOutput(stdout);
    detail::CStreamBuffer standardError(stderr);
    std::ostream out(&standardOutput);
    std::ostream err(&standardError);
    const int status = main(suite, parameter, args, out, err);
    out.flush();
    err.flush();
    return status;
}

/// The refusal of a value for the module parameter @p id, which the suite does
/// not declare.
inline std::invalid_argument undeclaredParameter(const std::string &id) {
    return std::invalid_argument("the suite declares no module parameter '" + id + "'");
}

/// A value of the integer type `Integer` of the module parameter @p id: the
/// text of its decimal digits, as the canonical form writes one, saved by
/// @p write, the writer of its type (saved/SavedValues.h).
template <typename Integer>
ModuleParameterValue integerParameter(const std::string &id, const std::string &literal,
                                      SCE::Saved::JsonValue (*write)(Integer)) {
    try {
        return write(FromCanonical::integer<Integer>(SCE::Saved::JsonValue(literal), id));
    } catch (const TraceRefusal &refused) {
        throw std::invalid_argument(refused.reason());
    }
}

/// A value of the `bool` module parameter @p id: `true` or `false`.
inline ModuleParameterValue boolParameter(const std::string &id, const std::string &literal) {
    if (literal == "true") {
        return SCE::Saved::savedWriteBool(true);
    }
    if (literal == "false") {
        return SCE::Saved::savedWriteBool(false);
    }
    throw std::invalid_argument(id + ": '" + literal + "' is neither true nor false");
}

/// A value of a text module parameter: the text itself. Its bound, like every
/// value's type, is held where the component reads it, as for a default.
inline ModuleParameterValue textParameter(const std::string &literal) {
    return SCE::Saved::savedWriteString(literal);
}

/// A value of an enum module parameter: the name its document declares for the
/// variant, which is how a value of an enum is saved. A name its enum does not
/// declare is refused where the component reads it, as for a default.
inline ModuleParameterValue enumParameter(const std::string &literal) {
    return SCE::Saved::JsonValue(literal);
}

/// A value of the byte string module parameter @p id: its octets in hex, two
/// digits each, as a literal of docs/SCE_TEST_PROFILE.md §11.4 writes them.
inline ModuleParameterValue bytesParameter(const std::string &id, const std::string &literal) {
    const auto digit = [](char hex) -> int {
        if (hex >= '0' && hex <= '9') {
            return hex - '0';
        }
        if (hex >= 'a' && hex <= 'f') {
            return hex - 'a' + 10;
        }
        if (hex >= 'A' && hex <= 'F') {
            return hex - 'A' + 10;
        }
        return -1;
    };
    const std::invalid_argument refused(id + ": '" + literal + "' is not octets in hex");
    if (literal.size() % 2 != 0) {
        throw refused;
    }
    std::vector<std::uint8_t> octets;
    for (std::size_t at = 0; at < literal.size(); at += 2) {
        const int high = digit(literal[at]);
        const int low = digit(literal[at + 1]);
        if (high < 0 || low < 0) {
            throw refused;
        }
        octets.push_back(static_cast<std::uint8_t>(high * 16 + low));
    }
    return SCE::Saved::savedWriteBytes(octets);
}

}  // namespace SCE::Test::Runner
