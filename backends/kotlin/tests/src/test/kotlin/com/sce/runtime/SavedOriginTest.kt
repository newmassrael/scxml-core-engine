// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0090, section 7 — saved-state format 3 holds an event's origin as one
// pair, processor then address, and its sender beside it, `null` where there is
// none. A reader takes both as they are written and refuses, naming the record,
// an origin that is neither `null` nor a whole pair, a sender that is not a
// session id, a queued event's sender beside no origin, a session's own send
// with no sender, with another pair than its sender's or naming another session
// than the one whose record holds it, and an internal act that holds either
// member. A queued event's sender is not tied to its address.
// A format-2 send that stored no origin is the send of the session holding it,
// and a format-1 or format-2 send whose origin is the SCXML prefix alone is
// refused.
// A text of any other format is refused by its `format` alone. A format-1 text,
// which has no place for the pair, is written back as format 1.
//
// The shared instances and the format-2 reading fixtures are held by
// `com.sce.integration.StaticDatamodelTest`; these are the texts no run writes.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class SavedOriginTest {

    private val scxml = IoProcessors.SCXML_PROCESSOR

    /** The SCXML processor's pair for the session [id], as format 3 writes it. */
    private fun pair(id: String): String =
        """{"processor":"$scxml","address":"${IoProcessors.scxmlLocation(id)}"}"""

    /** A host's own pair, which names no session. */
    private val host = """{"processor":"x-sce-host","address":"motor"}"""

    /** A format-3 tree of the one session `root`, waiting [pending] and holding [external] in its queue. */
    private fun tree(
        pending: List<String> = emptyList(),
        external: List<String> = emptyList(),
        invokes: List<String> = emptyList(),
    ): String =
        """{"format":3,"shape":"s","sessionid":"root","configuration":["s"],"current":"s","variables":{},""" +
            "\"history\":{},\"pending\":[${pending.joinToString(",")}]," +
            "\"invokes\":[${invokes.joinToString(",")}],\"invokestarts\":{}," +
            "\"hostinvokes\":[],\"hostinvoketoken\":\"0\",\"sendseq\":\"0\"," +
            "\"external\":[${external.joinToString(",")}]}"

    /** The id of the one child session [child] runs under `root`. */
    private val kid = "root.s.1.kid"

    /** A running child of `root`, the session [kid], waiting [pending]. */
    private fun child(pending: List<String>): String =
        """{"id":"kid","start":"1","invokeid":"s.1.kid","document":"kid","session":{"shape":"s",""" +
            "\"sessionid\":\"$kid\",\"configuration\":[\"s\"],\"current\":\"s\",\"variables\":{},\"history\":{}," +
            "\"pending\":[${pending.joinToString(",")}],\"invokes\":[],\"invokestarts\":{},\"hostinvokes\":[]," +
            "\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[]}}"

    /** A queued event whose `origin` and `sender` are the JSON texts [origin] and [sender]. */
    private fun event(origin: String, sender: String, name: String = "ping"): String =
        """{"name":"$name","data":"","type":"external","sendid":"","origin":$origin,"sender":$sender,""" +
            "\"invokeid\":\"\",\"hostinvoketoken\":null}"

    /** A waiting send of [act] whose `origin` and `sender` are the JSON texts [origin] and [sender]. */
    private fun send(act: String, origin: String, sender: String): String =
        """{"due":"1700000001000","act":"$act","event":"ping","data":"","sendid":"s","origin":$origin,""" +
            "\"sender\":$sender}"

    private fun refusal(text: String): String =
        assertFailsWith<StateRefusal> { SavedState.fromJson(text) }.message ?: ""

    @Test
    fun `a format-3 text is read as it is written and written back byte for byte`() {
        val text = tree(
            pending = listOf(
                send("raise", pair("root"), "\"root\""),
                send("internal", "null", "null"),
                send("parent", pair("root"), "\"root\""),
            ),
            external = listOf(
                event(pair("peer:1"), "\"peer:1\"", "answer"),
                event(host, "null", "rest"),
                // An SCXML pair at an address that names no session has no sender.
                event("""{"processor":"$scxml","address":"mesh://ecu"}""", "null", "arm"),
                event("null", "null", "swap"),
            ),
        )
        val saved = SavedState.fromJson(text)
        assertEquals(SavedState.FORMAT_3, saved.format)
        assertEquals(
            listOf(
                IoProcessors.scxmlOrigin("peer:1") to "peer:1",
                EventOrigin("x-sce-host", "motor") to null,
                EventOrigin(scxml, "mesh://ecu") to null,
                null to null,
            ),
            saved.external.map { it.origin to it.sender },
        )
        assertEquals(
            listOf(
                SavedAct.Raise("ping", "", "s", "root"),
                SavedAct.Internal("ping", "", "s"),
                SavedAct.Parent("ping", "", "s", "root"),
            ),
            saved.pending.map { it.act },
        )
        assertEquals(text, saved.toJson())
        // The pair is written processor first, and the reserved `:` of an id is
        // percent-encoded in its address.
        assertTrue(
            text.contains(""""origin":{"processor":"$scxml","address":"sce://scxml/peer%3A1"},"sender":"peer:1""""),
            text,
        )
    }

    @Test
    fun `an origin that is neither null nor a whole pair is refused naming its record`() {
        val cases = listOf(
            "\"motor\"" to "'external[0].origin' is neither null nor an origin",
            "[]" to "'external[0].origin' is neither null nor an origin",
            """{"processor":"x-sce-host"}""" to "'external[0].origin' has no 'address'",
            """{"address":"motor"}""" to "'external[0].origin' has no 'processor'",
            """{"processor":"","address":"motor"}""" to "'external[0].origin.processor' is empty",
            """{"processor":"x-sce-host","address":""}""" to "'external[0].origin.address' is empty",
            """{"processor":7,"address":"motor"}""" to "'external[0].origin.processor' is not a text",
            """{"processor":"x-sce-host","address":"motor","port":"1"}""" to "'external[0].origin' holds 'port'",
        )
        for ((origin, reason) in cases) {
            val message = refusal(tree(external = listOf(event(origin, "null"))))
            assertTrue(message.contains(reason), "$origin: $message")
        }
    }

    @Test
    fun `a sender that is not a session id or stands beside no origin is refused naming its record`() {
        val cases = listOf(
            Triple(pair("root"), "5", "'external[0].sender' is neither a text nor null"),
            Triple(pair("a b"), "\"a b\"", "'external[0].sender' (a b) is not a session id"),
            // A session that sends an event always has a reply address.
            Triple("null", "\"root\"", "'external[0]' names the sender 'root' beside no origin"),
        )
        for ((origin, sender, reason) in cases) {
            val message = refusal(tree(external = listOf(event(origin, sender))))
            assertTrue(message.contains(reason), "$origin, $sender: $message")
        }
    }

    @Test
    fun `a queued event's sender is read apart from its reply address`() {
        // A remote child's event names the child as its sender and its Mesh
        // endpoint as the address; any pair stands beside a sender.
        val endpoint = """{"processor":"$scxml","address":"mesh://dev/machine/root.kid"}"""
        val text = tree(
            external = listOf(
                event(endpoint, "\"root.kid\"", "answer"),
                event(pair("other"), "\"root.kid\"", "report"),
                event(host, "\"motor\"", "rest"),
            ),
        )
        val saved = SavedState.fromJson(text)
        assertEquals(
            listOf(
                EventOrigin(scxml, "mesh://dev/machine/root.kid") to "root.kid",
                IoProcessors.scxmlOrigin("other") to "root.kid",
                EventOrigin("x-sce-host", "motor") to "motor",
            ),
            saved.external.map { it.origin to it.sender },
        )
        assertEquals(text, saved.toJson())
    }

    @Test
    fun `a format-2 waiting send that stored no origin is the send of the session that holds it`() {
        val text = tree(
            pending = listOf(
                """{"due":"1700000001000","act":"raise","event":"ping","data":"","sendid":"r","origin":""}""",
                """{"due":"1700000002000","act":"parent","event":"ping","data":"","sendid":"p","origin":""}""",
            ),
        ).replace("{\"format\":3,", "{\"format\":2,")
        val saved = SavedState.fromJson(text)
        assertEquals(
            listOf(SavedAct.Raise("ping", "", "r", "root"), SavedAct.Parent("ping", "", "p", "root")),
            saved.pending.map { it.act },
        )
        // Written back as format 3 with the session's pair, which reads again.
        val written = saved.toJson()
        assertTrue(written.contains("\"sendid\":\"r\",\"origin\":${pair("root")},\"sender\":\"root\""), written)
        assertEquals(saved, SavedState.fromJson(written))
    }

    @Test
    fun `a record of format 3 that leaves out the origin or the sender is refused`() {
        val noOrigin = event("null", "null").replace("\"origin\":null,", "")
        assertTrue(refusal(tree(external = listOf(noOrigin))).contains("'external[0]' has no 'origin'"))
        val noSender = event("null", "null").replace("\"sender\":null,", "")
        assertTrue(refusal(tree(external = listOf(noSender))).contains("'external[0]' has no 'sender'"))
        val internal = send("internal", "null", "null").replace(",\"sender\":null", "")
        assertTrue(refusal(tree(pending = listOf(internal))).contains("'pending[0]' has no 'sender'"))
    }

    @Test
    fun `a session's own waiting send names its sender, and an internal one names neither`() {
        val cases = listOf(
            send("raise", "null", "null") to "'pending[0]' is a raise act with no sender",
            send("parent", "null", "null") to "'pending[0]' is a parent act with no sender",
            send("raise", host, "null") to "'pending[0]' is a raise act with no sender",
            send("raise", pair("other"), "\"root\"") to
                "'pending[0]' is a raise act whose origin is not the SCXML pair of its sender 'root'",
            send("parent", host, "\"root\"") to
                "'pending[0]' is a parent act whose origin is not the SCXML pair of its sender 'root'",
            send("internal", pair("root"), "null") to "'pending[0]' is an internal act and holds an origin or a sender",
            send("internal", "null", "\"root\"") to "'pending[0]' is an internal act and holds an origin or a sender",
            send("internal", pair("root"), "\"root\"") to "'pending[0]' is an internal act",
        )
        for ((act, reason) in cases) {
            val message = refusal(tree(pending = listOf(act)))
            assertTrue(message.contains(reason), "$act: $message")
        }
    }

    @Test
    fun `a session's own waiting send names the session whose record holds it`() {
        // At the root and in a child: each session's act is its own send.
        val own = tree(
            pending = listOf(send("raise", pair("root"), "\"root\"")),
            invokes = listOf(child(listOf(send("parent", pair(kid), "\"$kid\"")))),
        )
        assertEquals(own, SavedState.fromJson(own).toJson())
        val atRoot = refusal(tree(pending = listOf(send("raise", pair("y"), "\"y\""))))
        assertTrue(
            atRoot.contains("'pending[0].sender' is 'y', and this session is 'root': a raise act is this session's own send"),
            atRoot,
        )
        // A child's act that names its parent is refused, naming the child's path.
        val inChild = refusal(
            tree(invokes = listOf(child(listOf(send("parent", pair("root"), "\"root\""))))),
        )
        assertTrue(inChild.contains("invokes[0].session"), inChild)
        assertTrue(
            inChild.contains("'pending[0].sender' is 'root', and this session is '$kid': a parent act"),
            inChild,
        )
    }

    @Test
    fun `a stored waiting send origin that is the SCXML prefix alone is refused, and a queued one is not`() {
        val prefix = IoProcessors.SCXML_LOCATION_PREFIX
        val raise = """{"due":"1700000001000","act":"raise","event":"ping","data":"","sendid":"r","origin":"$prefix"}"""
        // Format 2, at the root of a tree, and format 1: it names no session in either.
        val format2 = tree(pending = listOf(raise)).replace("{\"format\":3,", "{\"format\":2,")
        val format1 = """{"format":1,"shape":"s","configuration":["s"],"current":"s","variables":{},""" +
            "\"history\":{},\"pending\":[$raise],\"invokes\":[],\"hostinvokes\":[],\"hostinvoketoken\":\"0\"," +
            "\"sendseq\":\"0\",\"external\":[]}"
        for (text in listOf(format2, format1)) {
            val refused = refusal(text)
            assertTrue(
                refused.contains("'pending[0].origin' is '$prefix', an SCXML location that names no session"),
                refused,
            )
        }
        // A queued event that stored it is the pair as saved, with no sender.
        val queued = """{"name":"go","data":"","type":"external","sendid":"","origin":"$prefix",""" +
            "\"origintype\":\"$scxml\",\"invokeid\":\"\",\"hostinvoketoken\":null}"
        val read = SavedState.fromJson(tree(external = listOf(queued)).replace("{\"format\":3,", "{\"format\":2,"))
        assertEquals(listOf(EventOrigin(scxml, prefix) to null), read.external.map { it.origin to it.sender })
    }

    @Test
    fun `a text of another format is refused by its format before anything it holds`() {
        assertEquals(
            "the saved state is format 4, and this runtime reads formats 1, 2 and 3",
            refusal("""{"format":4}"""),
        )
        assertEquals(
            "the saved state is format 0, and this runtime reads formats 1, 2 and 3",
            refusal("""{"format":0,"shape":7}"""),
        )
        assertEquals(
            "the saved state is format \"3\", and this runtime reads formats 1, 2 and 3",
            refusal("""{"format":"3"}"""),
        )
    }

    @Test
    fun `a format-1 text is written back as format 1, each origin as format 2 wrote it`() {
        val text = """{"format":1,"shape":"s","configuration":["s"],"current":"s","variables":{},"history":{},""" +
            "\"pending\":[" +
            "{\"due\":\"1700000001000\",\"act\":\"internal\",\"event\":\"ping\",\"data\":\"\",\"sendid\":\"i\",\"origin\":\"\"}," +
            "{\"due\":\"1700000002000\",\"act\":\"raise\",\"event\":\"ping\",\"data\":\"\",\"sendid\":\"r\",\"origin\":\"root\"}," +
            "{\"due\":\"1700000003000\",\"act\":\"raise\",\"event\":\"ping\",\"data\":\"\",\"sendid\":\"q\",\"origin\":\"\"}" +
            "],\"invokes\":[],\"hostinvokes\":[],\"hostinvoketoken\":\"0\",\"sendseq\":\"0\",\"external\":[" +
            """{"name":"ping","data":"","type":"external","sendid":"","origin":"kid","origintype":"$scxml",""" +
            "\"invokeid\":\"\",\"hostinvoketoken\":null}," +
            """{"name":"ping","data":"","type":"external","sendid":"","origin":"motor","origintype":"x-sce-host",""" +
            "\"invokeid\":\"\",\"hostinvoketoken\":null}," +
            """{"name":"ping","data":"","type":"external","sendid":"","origin":"","origintype":"",""" +
            "\"invokeid\":\"\",\"hostinvoketoken\":null}]}"
        val saved = SavedState.fromJson(text)
        assertEquals(SavedState.FORMAT_1, saved.format)
        assertEquals(
            listOf(
                IoProcessors.scxmlOrigin("kid") to "kid",
                EventOrigin("x-sce-host", "motor") to null,
                null to null,
            ),
            saved.external.map { it.origin to it.sender },
        )
        assertEquals(
            listOf(
                SavedAct.Internal("ping", "", "i"),
                SavedAct.Raise("ping", "", "r", "root"),
                SavedAct.Raise("ping", "", "q", null),
            ),
            saved.pending.map { it.act },
        )
        assertEquals(text, saved.toJson())
        // An internal act's stored origin is not read: the event it delivers is an
        // internal one, which names no sender, so it is written back empty.
        val stamped = text.replace("\"sendid\":\"i\",\"origin\":\"\"", "\"sendid\":\"i\",\"origin\":\"root\"")
        assertEquals(text, SavedState.fromJson(stamped).toJson())
    }
}
