// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The runner program of a generated suite on the JVM (docs/SCE_TTCN3_LANE.md
// §4): [runRunner] over the platform's files, standard output and standard
// error, ending the process with the exit status it returns. The `main` of a
// generated suite's unit calls [runnerMain], so the unit's class is the
// program (`java -cp <classpath> <package>.<Suite>Kt --trace-dir ...`).
//
// Cross-language sibling: `sce_rust_runtime::test::runner::main` (Rust).

package com.sce.runtime.test

import java.io.File
import java.io.IOException
import kotlin.system.exitProcess

/** The platform's files, as the runner reads and writes them ([RunnerFiles]). */
object JvmRunnerFiles : RunnerFiles {
    override fun path(dir: String, name: String): String = File(dir, name).path

    override fun readText(path: String): String = try {
        File(path).readText(Charsets.UTF_8)
    } catch (failure: IOException) {
        throw RunnerFileFailure(failure.toString())
    }

    override fun makeDirectories(dir: String) {
        val directory = File(dir)
        // `mkdirs` answers false both where it fails and where the directory
        // is already there, so the directory itself is what is asked.
        if (!directory.mkdirs() && !directory.isDirectory) {
            throw RunnerFileFailure("no directory could be made there")
        }
    }

    override fun writeText(path: String, text: String) {
        try {
            File(path).writeText(text, Charsets.UTF_8)
        } catch (failure: IOException) {
            throw RunnerFileFailure(failure.toString())
        }
    }
}

/**
 * The runner program of [suite], its module parameters read by [parameter], on
 * the command line [args] a JVM `main` is given (docs/SCE_TTCN3_LANE.md §4):
 * [runRunner] over [JvmRunnerFiles], the standard output and the standard
 * error, after which the process exits with the status it returns.
 *
 * The standard output is the contract's, the `<test case> <verdict>` lines and
 * nothing else, which a lane parses. So the program keeps it for those lines
 * and hands the JVM's `System.out` to the standard error while the cases run:
 * a machine's `<log>`, which a generated Kotlin statechart writes with
 * `println`, and any other diagnostic of the run, goes there.
 */
fun runnerMain(suite: Suite, parameter: ParameterReader, args: Array<String>): Nothing {
    val contract = System.out
    System.setOut(System.err)
    val status = try {
        runRunner(
            suite = suite,
            parameter = parameter,
            args = args.toList(),
            files = JvmRunnerFiles,
            out = { line -> contract.println(line) },
            err = { line -> System.err.println(line) },
        )
    } finally {
        System.setOut(contract)
    }
    contract.flush()
    System.err.flush()
    exitProcess(status)
}
