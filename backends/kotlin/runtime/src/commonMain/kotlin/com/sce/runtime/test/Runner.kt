// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of a generated suite (docs/SCE_TTCN3_LANE.md §4, §10.5):
// it runs every test case a world selects against its recorded input trace
// and writes each one's canonical test log.
//
// The unit of a suite generates the program's `main`, which hands [runnerMain]
// (`RunnerMain.kt`, on the JVM) the suite's tables and the reader of its module
// parameters (`moduleParameter`); the program is the class of the suite's unit
// (`<package>.<Suite>Kt`). The command line is the same on every backend:
//
//     <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]...
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
//
// The program's logic is here, over the files and streams of [RunnerFiles],
// so a test runs it in place as the platform's program runs it.
//
// Cross-language sibling: `sce_rust_runtime::test::runner` (Rust).

package com.sce.runtime.test

import com.sce.runtime.SavedValues

/**
 * The reader of a suite's module parameters, generated as `moduleParameter` in
 * the suite's unit: the saved value of the module parameter `id` of its
 * declared type, read from `literal`, the value the command line gives it.
 *
 * @throws OverrideRefusal naming why, where the suite declares no such
 *   parameter or the literal is no value of its type.
 */
typealias ParameterReader = (id: String, literal: String) -> ModuleParameterValue

/** The program's usage line. */
const val RUNNER_USAGE: String =
    "usage: <runner> --trace-dir <dir> --log-dir <dir> [--world <feature>,...] [--module-parameter <id>=<value>]..."

/** A command line that is not the runner's, refused, naming why. */
class RunnerUsageRefusal(
    /** Why. */
    val reason: String,
) : Exception(reason)

/** What the command line asks of the runner. */
data class RunnerOptions(
    /** The directory of the traces, `<test case>.json` each. */
    val traceDir: String,
    /** The directory the logs are written to, `<test case>.log.json` each. */
    val logDir: String,
    /** The features the world enables. */
    val world: Features,
    /**
     * Each `--module-parameter`, its id and the literal it gives, in the order
     * given.
     */
    val moduleParameters: List<Pair<String, String>>,
) {
    companion object {
        /**
         * The options [args] give, the program's name not among them.
         *
         * @throws RunnerUsageRefusal naming why, where they are not the
         *   program's command line.
         */
        fun parse(args: List<String>): RunnerOptions {
            var traceDir: String? = null
            var logDir: String? = null
            var world: String? = null
            val moduleParameters = mutableListOf<Pair<String, String>>()
            // Every option takes one value: the line is read a pair at a time.
            for (pair in args.chunked(2)) {
                val option = pair[0]
                if (pair.size < 2) {
                    throw RunnerUsageRefusal("$option is given no value")
                }
                val value = pair[1]
                when {
                    option == "--trace-dir" && traceDir == null -> traceDir = value
                    option == "--log-dir" && logDir == null -> logDir = value
                    option == "--world" && world == null -> world = value
                    option == "--module-parameter" -> {
                        val split = value.indexOf('=')
                        if (split < 0) {
                            throw RunnerUsageRefusal("--module-parameter $value is not <id>=<value>")
                        }
                        val id = value.substring(0, split)
                        if (moduleParameters.any { it.first == id }) {
                            throw RunnerUsageRefusal("the module parameter '$id' is given twice")
                        }
                        moduleParameters.add(id to value.substring(split + 1))
                    }
                    option == "--trace-dir" || option == "--log-dir" || option == "--world" ->
                        throw RunnerUsageRefusal("$option is given twice")
                    else -> throw RunnerUsageRefusal("$option is no option of the runner")
                }
            }
            return RunnerOptions(
                traceDir = traceDir ?: throw RunnerUsageRefusal("--trace-dir is not given"),
                logDir = logDir ?: throw RunnerUsageRefusal("--log-dir is not given"),
                world = world?.split(',')?.filter { it.isNotEmpty() }?.toSet() ?: emptySet(),
                moduleParameters = moduleParameters,
            )
        }
    }
}

/** Why a file of the runner is not read or written, or a directory not made, in the platform's words. */
class RunnerFileFailure(
    /** Why. */
    val reason: String,
) : Exception(reason)

/**
 * The files the runner program reads and writes, on its platform: the common
 * code has none of its own (`RunnerMain.kt` holds the JVM's).
 */
interface RunnerFiles {
    /** The path of the file [name] in the directory [dir], as a message names it. */
    fun path(dir: String, name: String): String

    /**
     * The text of the file at [path], in UTF-8.
     *
     * @throws RunnerFileFailure where it is not read.
     */
    fun readText(path: String): String

    /**
     * Makes the directory [dir], and those above it, where they are missing.
     *
     * @throws RunnerFileFailure where it is not made.
     */
    fun makeDirectories(dir: String)

    /**
     * Writes [text] in UTF-8 as the whole of the file at [path], with nothing
     * after it.
     *
     * @throws RunnerFileFailure where it is not written.
     */
    fun writeText(path: String, text: String)
}

/**
 * The trace of the test case [testCase], `<dir>/<test case>.json`, read by
 * [files].
 *
 * @throws TraceRefusal where it is not read, naming its path, or is refused.
 */
fun readTrace(files: RunnerFiles, dir: String, testCase: String): Trace {
    val path = files.path(dir, "$testCase.json")
    val text = try {
        files.readText(path)
    } catch (failure: RunnerFileFailure) {
        throw TraceRefusal("$path is not read: ${failure.reason}")
    }
    return Trace.parse(text)
}

/**
 * The runner program of [suite], its module parameters read by [parameter],
 * on the command line [args], the program's name not among them
 * (docs/SCE_TTCN3_LANE.md §4), over [files], each line of the standard output
 * handed to [out] and of the standard error to [err]: see the file's header.
 * Returns the program's exit status.
 */
fun runRunner(
    suite: Suite,
    parameter: ParameterReader,
    args: List<String>,
    files: RunnerFiles,
    out: (String) -> Unit,
    err: (String) -> Unit,
): Int {
    val usage = { why: String ->
        err(why)
        err(RUNNER_USAGE)
        2
    }
    val options = try {
        RunnerOptions.parse(args)
    } catch (refused: RunnerUsageRefusal) {
        return usage(refused.reason)
    }
    val overrides = ModuleParameters()
    for ((id, literal) in options.moduleParameters) {
        try {
            overrides.set(id, parameter(id, literal))
        } catch (refused: OverrideRefusal) {
            return usage("--module-parameter $id=$literal: ${refused.reason}")
        }
    }
    val results = try {
        runSuiteWithTraces(suite, options.world, { readTrace(files, options.traceDir, it) }, overrides)
    } catch (refused: OverrideRefusal) {
        return usage(refused.reason)
    }
    try {
        files.makeDirectories(options.logDir)
    } catch (failure: RunnerFileFailure) {
        err("${options.logDir} is not made: ${failure.reason}")
        return 2
    }
    for (result in results) {
        for (line in result.result.log) {
            if (line.kind == LogKind.ERROR) {
                err("${result.testCase}: ${line.component}: ${line.detail}")
            }
        }
        val path = files.path(options.logDir, "${result.testCase}.log.json")
        try {
            files.writeText(path, result.canonicalLog(suite.name))
        } catch (failure: RunnerFileFailure) {
            err("$path is not written: ${failure.reason}")
            return 2
        }
        out("${result.testCase} ${result.result.verdict.spelling}")
    }
    return 0
}

/**
 * The refusal of a value for the module parameter [id], which the suite does
 * not declare.
 */
fun undeclaredParameter(id: String): OverrideRefusal =
    OverrideRefusal("the suite declares no module parameter '$id'")

/**
 * A value of the integer type [type] of the module parameter [id]: the text of
 * its decimal digits, as the canonical form writes one. The suite's unit saves
 * it with `SavedValues.of` of its type.
 *
 * @throws OverrideRefusal naming the parameter, where it is no such text or
 *   past the range of [type].
 */
fun <T : Any> integerParameter(id: String, literal: String, type: FromCanonical.IntegerType<T>): T =
    try {
        FromCanonical.integer(literal, type, id)
    } catch (refused: TraceRefusal) {
        throw OverrideRefusal(refused.reason)
    }

/**
 * A value of the `bool` module parameter [id]: `true` or `false`.
 *
 * @throws OverrideRefusal otherwise.
 */
fun boolParameter(id: String, literal: String): ModuleParameterValue = when (literal) {
    "true" -> SavedValues.of(true)
    "false" -> SavedValues.of(false)
    else -> throw OverrideRefusal("$id: '$literal' is neither true nor false")
}

/**
 * A value of a text module parameter: the text itself. Its bound, like every
 * value's type, is held where the component reads it, as for a default.
 */
fun textParameter(literal: String): ModuleParameterValue = SavedValues.of(literal)

/**
 * A value of an enum module parameter: the name its document declares for the
 * variant, which is how a value of an enum is saved. A name its enum does not
 * declare is refused where the component reads it, as for a default.
 */
fun enumParameter(literal: String): ModuleParameterValue = literal

/**
 * A value of the byte string module parameter [id]: its octets in hex, two
 * digits each, as a literal of docs/SCE_TEST_PROFILE.md §11.4 writes them.
 *
 * @throws OverrideRefusal where it is not.
 */
fun bytesParameter(id: String, literal: String): ModuleParameterValue {
    val refused = { OverrideRefusal("$id: '$literal' is not octets in hex") }
    if (literal.length % 2 != 0) {
        throw refused()
    }
    val octets = ByteArray(literal.length / 2)
    for (index in octets.indices) {
        val high = hexDigit(literal[2 * index]) ?: throw refused()
        val low = hexDigit(literal[2 * index + 1]) ?: throw refused()
        octets[index] = ((high shl 4) or low).toByte()
    }
    return SavedValues.of(octets)
}

/**
 * The value of one ASCII hex digit of either case, or null: digits alone, where
 * a reader of a number would also read a sign, and `digitToIntOrNull` the
 * digits of every script.
 */
private fun hexDigit(digit: Char): Int? = when (digit) {
    in '0'..'9' -> digit - '0'
    in 'a'..'f' -> digit - 'a' + 10
    in 'A'..'F' -> digit - 'A' + 10
    else -> null
}
