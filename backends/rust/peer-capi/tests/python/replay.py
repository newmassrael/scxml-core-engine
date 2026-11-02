# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Replays the recorded scenario of the peer core through the C ABI, from Python (ctypes).

usage: replay.py <libsce_peer_capi> <scenario.json>

The cores of two devices are made, given the administrator's acts, the machine's sends, the rounds'
answers and a save, with no relay, and every text a core gives (each request, each outcome, each
standing, the genesis, the save, the reason of a refusal) must be the recorded one.
"""

import ctypes
import json
import sys

ABI_VERSION = 2
OK = 0
KEPT = {"taken": 0, "held": 1, "not_signed": 2, "refused": 3}
KINDS = {"sync": 0, "resync": 1, "reread": 2}
PROFILES = {"untrusted": 0, "trusted_hub": 1}


class Buf(ctypes.Structure):
    _fields_ = [("ptr", ctypes.POINTER(ctypes.c_uint8)), ("len", ctypes.c_size_t)]


def load(path):
    lib = ctypes.CDLL(path)
    u8p = ctypes.POINTER(ctypes.c_uint8)
    vp = ctypes.c_void_p
    cp = ctypes.c_char_p
    bufp = ctypes.POINTER(Buf)
    sig = {
        "sce_peer_abi_version": (ctypes.c_uint32, []),
        "sce_peer_last_error": (cp, []),
        "sce_peer_buf_free": (None, [Buf]),
        "sce_peer_open": (vp, [cp]),
        "sce_peer_invited": (vp, [cp, u8p]),
        "sce_peer_free": (None, [vp]),
        "sce_peer_set_identity": (ctypes.c_int32, [vp, ctypes.c_uint64, u8p]),
        "sce_peer_set_administrator": (ctypes.c_int32, [vp, u8p]),
        "sce_peer_open_list": (ctypes.c_int32, [vp, ctypes.c_int32, u8p]),
        "sce_peer_grant": (ctypes.c_int32, [vp, ctypes.c_uint64, ctypes.c_uint32, u8p]),
        "sce_peer_remove_member": (ctypes.c_int32, [vp, ctypes.c_uint64]),
        "sce_peer_standing": (ctypes.c_int32, [vp, bufp]),
        "sce_peer_outbound": (
            ctypes.c_int32,
            [vp, cp, cp, ctypes.POINTER(ctypes.c_int32), bufp],
        ),
        "sce_peer_start": (ctypes.c_int32, [vp, ctypes.c_int32]),
        "sce_peer_poll_request": (ctypes.c_int32, [vp, bufp]),
        "sce_peer_respond": (ctypes.c_int32, [vp, cp]),
        "sce_peer_fail": (ctypes.c_int32, [vp, cp]),
        "sce_peer_take_outcome": (ctypes.c_int32, [vp, bufp]),
        "sce_peer_save": (ctypes.c_int32, [vp, bufp]),
        "sce_peer_restore": (ctypes.c_int32, [vp, cp, ctypes.POINTER(ctypes.c_uint64)]),
    }
    for name, (restype, argtypes) in sig.items():
        function = getattr(lib, name)
        function.restype = restype
        function.argtypes = argtypes
    return lib


def raw(data):
    return (ctypes.c_uint8 * max(len(data), 1)).from_buffer_copy(data or b"\0")


def main():
    lib = load(sys.argv[1])
    with open(sys.argv[2], encoding="utf-8") as file:
        scenario = json.load(file)
    assert lib.sce_peer_abi_version() == ABI_VERSION, "the library is another ABI version"
    steps = 0

    def die(what):
        raise SystemExit(f"replay: step {steps}: {what} ({lib.sce_peer_last_error()})")

    def check(status, what):
        if status != OK:
            die(f"{what} failed with {status}")

    def taken(buf):
        """The text of a buffer the library gave, given back; None for an empty buffer."""
        if not buf.ptr:
            return None
        text = ctypes.string_at(buf.ptr, buf.len).decode("utf-8")
        lib.sce_peer_buf_free(buf)
        return text

    def expect(got, want, what):
        if (got or "") != want:
            raise SystemExit(
                f"replay: step {steps}: {what} is not the recorded one\n  got:  {got}\n  want: {want}"
            )

    list_name = scenario["list"].encode()
    devices = scenario["devices"]
    peers = [None] * len(devices)
    for step in scenario["steps"]:
        at = step["device"]
        op = step["op"]
        if op == "make":
            device = devices[at]
            if peers[at]:
                lib.sce_peer_free(peers[at])
            if device["genesis"] is None:
                peers[at] = lib.sce_peer_open(list_name)
            else:
                peers[at] = lib.sce_peer_invited(list_name, raw(bytes.fromhex(device["genesis"])))
            if not peers[at]:
                die("the core is not made")
            check(
                lib.sce_peer_set_identity(peers[at], device["node"], raw(bytes.fromhex(device["seed"]))),
                "set_identity",
            )
            if device.get("administrator") is not None:
                check(
                    lib.sce_peer_set_administrator(peers[at], raw(bytes.fromhex(device["administrator"]))),
                    "set_administrator",
                )
            steps += 1
            continue
        peer = peers[at]
        if not peer:
            die("the device's core is not made")
        out = Buf()
        if op == "open":
            genesis = (ctypes.c_uint8 * 32)()
            check(lib.sce_peer_open_list(peer, PROFILES[step["profile"]], genesis), "open_list")
            expect(bytes(genesis).hex(), step["genesis"], "the genesis")
        elif op == "grant":
            key = raw(bytes.fromhex(step["key"]))
            check(lib.sce_peer_grant(peer, step["node"], step["role"], key), "grant")
        elif op == "remove":
            check(lib.sce_peer_remove_member(peer, step["node"]), "remove_member")
        elif op == "standing":
            check(lib.sce_peer_standing(peer, ctypes.byref(out)), "standing")
            expect(taken(out), step["text"], "the standing")
        elif op == "restore":
            count = ctypes.c_uint64(2**64 - 1)
            check(lib.sce_peer_restore(peer, step["text"].encode(), ctypes.byref(count)), "restore")
            if count.value != step["count"]:
                die("the restore kept another number of operations than was recorded")
        elif op == "outbound":
            kept = ctypes.c_int32(-1)
            check(
                lib.sce_peer_outbound(
                    peer, step["event"].encode(), step["data"].encode(), ctypes.byref(kept), ctypes.byref(out)
                ),
                "outbound",
            )
            if kept.value != KEPT[step["kept"]]:
                die("the core did not keep the send as recorded")
            expect(taken(out), step["reason"], "the reason of the refusal")
        elif op == "start":
            check(lib.sce_peer_start(peer, KINDS[step["kind"]]), "start")
        elif op == "request":
            check(lib.sce_peer_poll_request(peer, ctypes.byref(out)), "poll_request")
            request = taken(out)
            if request is None:
                die("the core asks nothing where a request was recorded")
            expect(request, step["text"], "the request")
        elif op == "respond":
            check(lib.sce_peer_respond(peer, step["text"].encode()), "respond")
        elif op == "fail":
            check(lib.sce_peer_fail(peer, step["reason"].encode()), "fail")
        elif op == "outcome":
            check(lib.sce_peer_poll_request(peer, ctypes.byref(out)), "poll_request")
            if taken(out) is not None:
                die("the core still asks where the exchange was over")
            ended = Buf()
            check(lib.sce_peer_take_outcome(peer, ctypes.byref(ended)), "take_outcome")
            outcome = taken(ended)
            if outcome is None:
                die("no exchange ended where an outcome was recorded")
            expect(outcome, step["text"], "the outcome")
        elif op == "save":
            check(lib.sce_peer_save(peer, ctypes.byref(out)), "save")
            expect(taken(out), step["text"], "the save")
        else:
            die(f"a step the replay does not know: {op}")
        steps += 1
    for peer in peers:
        if peer:
            lib.sce_peer_free(peer)
    print(f"replayed {steps} steps")
    return 0


if __name__ == "__main__":
    sys.exit(main())
