// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

/**
 * The key-value store, in memory, with the quota the embedding set. Keys are ordered by
 * their UTF-8 bytes, the order the specification gives: a `String` compares by UTF-16 code
 * unit, which differs from it past the basic plane.
 */
internal class Store {
    val keys = HashMap<String, String>()
    var quota: Int? = null

    fun entries(): List<Pair<String, String>> =
        keys.entries.map { it.key to it.value }.sortedWith { a, b -> compareUtf8(a.first, b.first) }
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

internal fun ReferenceHost.kvGet(key: String): Outcome {
    val value = store.keys[key] ?: return record("found" to false, "value" to "")
    return record("found" to true, "value" to value)
}

internal fun ReferenceHost.kvPut(key: String, value: String): Outcome {
    val replaced = key in store.keys
    val quota = store.quota
    if (!replaced && quota != null && store.keys.size >= quota) return failure("sce-host-kv-put", "quota_exceeded")
    store.keys[key] = value
    return record("replaced" to replaced)
}

internal fun ReferenceHost.kvDelete(key: String): Outcome = record("existed" to (store.keys.remove(key) != null))

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
