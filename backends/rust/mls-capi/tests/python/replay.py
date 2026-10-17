# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Replays the recorded scenario of the group key agreement through the C ABI, from Python.

usage: replay.py <libsce_mls_capi> <scenario.json>

A device is loaded as it was before it joined and is given the Welcome and the commits with the
membership log as data; the keys it derives must be the recorded ones, until the commit that removes
it, after which it derives none.
"""

import ctypes
import json
import sys

OK = 0
ERR_REMOVED = 12


class Buf(ctypes.Structure):
    _fields_ = [("ptr", ctypes.POINTER(ctypes.c_uint8)), ("len", ctypes.c_size_t)]


def load(path):
    lib = ctypes.CDLL(path)
    u8p = ctypes.POINTER(ctypes.c_uint8)
    vp = ctypes.c_void_p
    sig = {
        "sce_mls_abi_version": (ctypes.c_uint32, []),
        "sce_mls_last_error": (ctypes.c_char_p, []),
        "sce_mls_log_new": (vp, [ctypes.c_uint32]),
        "sce_mls_log_add_member": (ctypes.c_int32, [vp, ctypes.c_uint64, u8p]),
        "sce_mls_log_name_commit": (ctypes.c_int32, [vp, ctypes.c_uint32, u8p]),
        "sce_mls_log_free": (None, [vp]),
        "sce_mls_device_from_bytes": (
            vp,
            [ctypes.c_char_p, ctypes.c_uint64, u8p, u8p, ctypes.c_size_t],
        ),
        "sce_mls_device_free": (None, [vp]),
        "sce_mls_join": (
            ctypes.c_int32,
            [vp, u8p, ctypes.c_size_t, u8p, ctypes.c_size_t, vp, ctypes.POINTER(ctypes.c_uint64)],
        ),
        "sce_mls_apply_commit": (
            ctypes.c_int32,
            [
                vp,
                u8p,
                ctypes.c_size_t,
                vp,
                ctypes.POINTER(ctypes.c_int32),
                ctypes.POINTER(ctypes.c_uint64),
            ],
        ),
        "sce_mls_is_active": (ctypes.c_int32, [vp]),
        "sce_mls_epoch_key": (ctypes.c_int32, [vp, u8p]),
        "sce_mls_section_key": (ctypes.c_int32, [vp, ctypes.c_char_p, u8p]),
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
    scenario = json.load(open(sys.argv[2]))
    assert lib.sce_mls_abi_version() == 1

    def check(status, what):
        if status != OK:
            raise SystemExit(f"replay: {what} failed with {status}: {lib.sce_mls_last_error()}")

    def make_log(log):
        handle = lib.sce_mls_log_new(log["epoch"])
        for node, key in log["members"]:
            check(lib.sce_mls_log_add_member(handle, node, raw(bytes.fromhex(key))), "add_member")
        for epoch, commit in log["commits"]:
            check(lib.sce_mls_log_name_commit(handle, epoch, raw(bytes.fromhex(commit))), "name_commit")
        return handle

    device_info = scenario["device"]
    seed = bytes.fromhex(device_info["seed"])
    state = bytes.fromhex(device_info["state"])
    device = lib.sce_mls_device_from_bytes(
        scenario["list"].encode(), device_info["node"], raw(seed), raw(state), len(state)
    )
    if not device:
        raise SystemExit(f"replay: the device does not load: {lib.sce_mls_last_error()}")

    key = (ctypes.c_uint8 * 32)()
    steps = 0
    for step in scenario["steps"]:
        log = make_log(step["log"])
        commit = bytes.fromhex(step["commit"])
        epoch = ctypes.c_uint64(0)
        removed = ctypes.c_int32(-1)
        if step["op"] == "join":
            welcome = bytes.fromhex(step["welcome"])
            check(
                lib.sce_mls_join(
                    device, raw(welcome), len(welcome), raw(commit), len(commit), log, ctypes.byref(epoch)
                ),
                "join",
            )
            assert epoch.value == step["epoch"], "the epoch joined is not the recorded one"
            check(lib.sce_mls_epoch_key(device, key), "epoch_key")
            assert bytes(key).hex() == step["epoch_key"], "the key of the epoch joined"
            check(lib.sce_mls_section_key(device, step["section"].encode(), key), "section_key")
            assert bytes(key).hex() == step["section_key"], "the key of the section"
        elif step.get("removed"):
            check(
                lib.sce_mls_apply_commit(
                    device, raw(commit), len(commit), log, ctypes.byref(removed), ctypes.byref(epoch)
                ),
                "apply_commit of the removal",
            )
            assert removed.value == 1, "the commit did not remove the device"
            assert lib.sce_mls_is_active(device) == 0, "a removed device is still active"
            assert lib.sce_mls_epoch_key(device, key) == ERR_REMOVED, "a removed device derived a key"
        else:
            check(
                lib.sce_mls_apply_commit(
                    device, raw(commit), len(commit), log, ctypes.byref(removed), ctypes.byref(epoch)
                ),
                "apply_commit",
            )
            assert removed.value == 0 and epoch.value == step["epoch"], "the epoch applied"
            check(lib.sce_mls_epoch_key(device, key), "epoch_key")
            assert bytes(key).hex() == step["epoch_key"], "the key of the epoch applied"
        lib.sce_mls_log_free(log)
        steps += 1
    lib.sce_mls_device_free(device)
    print(f"replayed {steps} steps")
    return 0 if steps == 3 else 1


if __name__ == "__main__":
    sys.exit(main())
