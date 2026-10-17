// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Replays the recorded scenario of the group key agreement (backends/rust/mls/tests/fixtures) through
// the C ABI, from Kotlin with JNA: a device is loaded as it was before it joined and is given the
// Welcome and the commits with the membership log as data; the keys it derives must be the recorded
// ones, until the commit that removes it, after which it derives none.
//
// usage: Replay <libsce_mls_capi> <scenario.json>

package com.sce.mls

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.NativeLong
import com.sun.jna.Pointer
import com.sun.jna.ptr.IntByReference
import com.sun.jna.ptr.LongByReference
import java.io.File
import kotlin.system.exitProcess
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long

private const val OK = 0
private const val ERR_REMOVED = 12

/** What `include/sce_mls.h` declares of the calls a replay makes (a `uint8_t *` is a byte array). */
@Suppress("FunctionName")
interface SceMls : Library {
    fun sce_mls_abi_version(): Int
    fun sce_mls_last_error(): String?
    fun sce_mls_log_new(epoch: Int): Pointer?
    fun sce_mls_log_add_member(log: Pointer, node: Long, key: ByteArray): Int
    fun sce_mls_log_name_commit(log: Pointer, epoch: Int, hash: ByteArray): Int
    fun sce_mls_log_free(log: Pointer)
    fun sce_mls_device_from_bytes(list: String, node: Long, seed: ByteArray, state: ByteArray, len: NativeLong): Pointer?
    fun sce_mls_device_free(device: Pointer)
    fun sce_mls_join(
        device: Pointer,
        welcome: ByteArray,
        welcomeLen: NativeLong,
        commit: ByteArray,
        commitLen: NativeLong,
        log: Pointer,
        epoch: LongByReference,
    ): Int
    fun sce_mls_apply_commit(
        device: Pointer,
        commit: ByteArray,
        commitLen: NativeLong,
        log: Pointer,
        removed: IntByReference,
        epoch: LongByReference,
    ): Int
    fun sce_mls_is_active(device: Pointer): Int
    fun sce_mls_epoch_key(device: Pointer, out: ByteArray): Int
    fun sce_mls_section_key(device: Pointer, name: String, out: ByteArray): Int
}

private fun die(what: String): Nothing {
    System.err.println("replay: $what")
    exitProcess(1)
}

private fun unhex(text: String): ByteArray =
    ByteArray(text.length / 2) { text.substring(2 * it, 2 * it + 2).toInt(16).toByte() }

private fun hex(bytes: ByteArray): String = bytes.joinToString("") { "%02x".format(it) }

fun main(args: Array<String>) {
    if (args.size != 2) {
        System.err.println("usage: Replay <libsce_mls_capi> <scenario.json>")
        exitProcess(2)
    }
    val lib = Native.load(args[0], SceMls::class.java)
    val scenario = Json.parseToJsonElement(File(args[1]).readText()).jsonObject
    if (lib.sce_mls_abi_version() != 1) die("the library is another ABI version")

    fun check(status: Int, what: String) {
        if (status != OK) die("$what failed with $status: ${lib.sce_mls_last_error()}")
    }

    fun makeLog(log: JsonObject): Pointer {
        val handle = lib.sce_mls_log_new(log.getValue("epoch").jsonPrimitive.int) ?: die("no log")
        for (member in log.getValue("members").jsonArray) {
            val row: JsonArray = member.jsonArray
            check(lib.sce_mls_log_add_member(handle, row[0].jsonPrimitive.long, unhex(row[1].jsonPrimitive.content)), "add_member")
        }
        for (commit in log.getValue("commits").jsonArray) {
            val row: JsonArray = commit.jsonArray
            check(lib.sce_mls_log_name_commit(handle, row[0].jsonPrimitive.int, unhex(row[1].jsonPrimitive.content)), "name_commit")
        }
        return handle
    }

    val info = scenario.getValue("device").jsonObject
    val seed = unhex(info.getValue("seed").jsonPrimitive.content)
    val state = unhex(info.getValue("state").jsonPrimitive.content)
    val device = lib.sce_mls_device_from_bytes(
        scenario.getValue("list").jsonPrimitive.content,
        info.getValue("node").jsonPrimitive.long,
        seed,
        state,
        NativeLong(state.size.toLong()),
    ) ?: die("the device does not load: ${lib.sce_mls_last_error()}")

    val key = ByteArray(32)
    var steps = 0
    for (element in scenario.getValue("steps").jsonArray) {
        val step = element.jsonObject
        val log = makeLog(step.getValue("log").jsonObject)
        val commit = unhex(step.getValue("commit").jsonPrimitive.content)
        val epoch = LongByReference(0)
        val removed = IntByReference(-1)
        val op = step.getValue("op").jsonPrimitive.content
        val isRemoval = step["removed"]?.jsonPrimitive?.content == "true"
        if (op == "join") {
            val welcome = unhex(step.getValue("welcome").jsonPrimitive.content)
            check(
                lib.sce_mls_join(
                    device,
                    welcome,
                    NativeLong(welcome.size.toLong()),
                    commit,
                    NativeLong(commit.size.toLong()),
                    log,
                    epoch,
                ),
                "join",
            )
            if (epoch.value != step.getValue("epoch").jsonPrimitive.long) die("the epoch joined is not the recorded one")
            check(lib.sce_mls_epoch_key(device, key), "epoch_key")
            if (hex(key) != step.getValue("epoch_key").jsonPrimitive.content) die("the key of the epoch joined")
            check(lib.sce_mls_section_key(device, step.getValue("section").jsonPrimitive.content, key), "section_key")
            if (hex(key) != step.getValue("section_key").jsonPrimitive.content) die("the key of the section")
        } else if (isRemoval) {
            check(
                lib.sce_mls_apply_commit(device, commit, NativeLong(commit.size.toLong()), log, removed, epoch),
                "apply_commit of the removal",
            )
            if (removed.value != 1) die("the commit did not remove the device")
            if (lib.sce_mls_is_active(device) != 0) die("a removed device is still active")
            if (lib.sce_mls_epoch_key(device, key) != ERR_REMOVED) die("a removed device derived a key")
        } else {
            check(
                lib.sce_mls_apply_commit(device, commit, NativeLong(commit.size.toLong()), log, removed, epoch),
                "apply_commit",
            )
            if (removed.value != 0 || epoch.value != step.getValue("epoch").jsonPrimitive.long) die("the epoch applied")
            check(lib.sce_mls_epoch_key(device, key), "epoch_key")
            if (hex(key) != step.getValue("epoch_key").jsonPrimitive.content) die("the key of the epoch applied")
        }
        lib.sce_mls_log_free(log)
        steps++
    }
    lib.sce_mls_device_free(device)
    println("replayed $steps steps")
    exitProcess(if (steps == 3) 0 else 1)
}
