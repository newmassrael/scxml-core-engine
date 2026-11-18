// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

/**
 * The key-value store, in memory, with the quota the embedding set. Keys are ordered by
 * their UTF-8 bytes, the order the specification gives: a `String` compares by UTF-16 code
 * unit, which differs from it past the basic plane.
 */
internal class Store {
    /** A stored value and the version its write gave the key. */
    class Entry(val value: String, val version: ULong)

    val keys = HashMap<String, Entry>()
    var quota: Int? = null

    /**
     * The last number a write or a removal took: versions are numbered from this one
     * counter of the store (docs/SCE_HOST_FEATURES.md, section 8), so a key is never
     * given a version it had, even after it was removed.
     */
    private var lastVersion: ULong = 0uL

    fun entries(): List<Pair<String, String>> =
        keys.entries.map { it.key to it.value.value }.sortedWith { a, b -> compareUtf8(a.first, b.first) }

    /** The counter's next number, taken. */
    fun nextVersion(): ULong {
        lastVersion += 1uL
        return lastVersion
    }

    /** The version [key] is at: `0` for a key the store does not hold. */
    fun versionOf(key: String): ULong = keys[key]?.version ?: 0uL

    /** Whether writing [key] would hold one key more than the quota allows. */
    fun overQuota(key: String): Boolean {
        val bound = quota ?: return false
        return key !in keys && keys.size >= bound
    }

    /** Keep [value] under [key] at the counter's next number; that number. */
    fun write(key: String, value: String): ULong {
        val version = nextVersion()
        keys[key] = Entry(value, version)
        return version
    }
}

internal fun compareUtf8(a: String, b: String): Int {
    val x = a.encodeToByteArray()
    val y = b.encodeToByteArray()
    for (i in 0 until minOf(x.size, y.size)) {
        val d = (x[i].toInt() and 0xff) - (y[i].toInt() and 0xff)
        if (d != 0) return d
    }
    return x.size - y.size
}

/** A key is not empty and holds no U+0000 (its 256-byte bound is the record's own). */
internal fun validKey(key: String): Boolean = key.isNotEmpty() && '\u0000' !in key

/** An expected version is one or more of the digits `0` to `9`, and nothing else. */
internal fun validVersion(text: String): Boolean = text.isNotEmpty() && text.all { it in '0'..'9' }

internal fun ReferenceHost.kvGet(key: String): Outcome {
    val entry = store.keys[key] ?: return record("found" to false, "value" to "", "version" to "0")
    return record("found" to true, "value" to entry.value, "version" to entry.version.toString())
}

internal fun ReferenceHost.kvPut(key: String, value: String): Outcome {
    if (store.overQuota(key)) return failure("sce-host-kv-put", "quota_exceeded")
    val replaced = key in store.keys
    store.write(key, value)
    return record("replaced" to replaced)
}

/**
 * Write [value] under [key] only when the key is at [expected], a text of digits the
 * refusal already judged. It is compared as a number, so leading zeros do not count, and
 * one past the largest 64-bit unsigned integer names no version.
 */
internal fun ReferenceHost.kvPutIf(key: String, value: String, expected: String): Outcome {
    val current = store.versionOf(key)
    if (expected.toULongOrNull() != current) {
        return record("written" to false, "version" to current.toString())
    }
    if (store.overQuota(key)) return failure("sce-host-kv-put-if", "quota_exceeded")
    return record("written" to true, "version" to store.write(key, value).toString())
}

internal fun ReferenceHost.kvDelete(key: String): Outcome {
    val existed = store.keys.remove(key) != null
    if (existed) store.nextVersion()
    return record("existed" to existed)
}

/**
 * The first key, in ascending order of UTF-8 bytes, that is greater than [after] and begins
 * with [prefix]; [after] is excluded, and an empty [after] starts at the least key.
 */
internal fun ReferenceHost.kvNextKey(after: String, prefix: String): Outcome {
    for ((key, _) in store.entries()) {
        if ((after.isEmpty() || compareUtf8(key, after) > 0) && key.startsWith(prefix)) {
            return record("found" to true, "key" to key)
        }
    }
    return record("found" to false, "key" to "")
}
