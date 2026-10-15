// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The entry point of the step server: `--machine <name>`, then one request per line of
// stdin and one reply per line of stdout, until `shutdown` or the end of stdin.

package com.sce.world

import java.io.BufferedReader
import java.io.FileDescriptor
import java.io.FileOutputStream
import java.io.InputStreamReader
import java.io.PrintStream
import kotlin.system.exitProcess

fun main(args: Array<String>) {
    val name = machineName(args) ?: run {
        System.err.println("usage: --machine <name>, one of ${hostableMachines.keys.sorted().joinToString()}")
        exitProcess(2)
    }
    val create = hostableMachines[name] ?: run {
        System.err.println("no machine named `$name` is hosted here: ${hostableMachines.keys.sorted().joinToString()}")
        exitProcess(2)
    }
    // The protocol owns stdout. Anything else that prints, a runtime's diagnostic or a
    // machine's own output, goes to stderr, which the world shows when a run fails; a
    // stray line on stdout would be read as a reply and fail the run for the wrong reason.
    val protocol = PrintStream(FileOutputStream(FileDescriptor.out), false, Charsets.UTF_8)
    System.setOut(System.err)
    val input = BufferedReader(InputStreamReader(System.`in`, Charsets.UTF_8))

    val server = StepServer(name, create)
    try {
        while (!server.stopped) {
            // The end of stdin is the world going away without a word: leave quietly.
            val line = input.readLine() ?: break
            protocol.print(server.handle(line))
            protocol.print('\n')
            protocol.flush()
        }
    } finally {
        server.close()
    }
}

private fun machineName(args: Array<String>): String? {
    val at = args.indexOf("--machine")
    return if (at >= 0) args.getOrNull(at + 1) else null
}
