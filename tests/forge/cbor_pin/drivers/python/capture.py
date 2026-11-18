#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The Python driver of the CBOR pin (tests/forge/cbor_pin/pin.py, docs/adr/0101 step 0).

    capture.py GENERATED INPUTS OUTPUT

GENERATED holds the Python the generator wrote for the pinned codecs and every
document they import (`sce-codegen generate --with-imports -l python`). Each line
of INPUTS, `<codec>/<case> <hex>`, is decoded by that codec and the value it read
encoded again; OUTPUT gets `<id> ok <hex>` with what the encode wrote, or
`<id> refused <words>` naming the refusal the decode or the encode raised.
"""

from __future__ import annotations

import importlib
import sys
import types
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[5]
RUNTIME_SRC = REPO_ROOT / "backends" / "python" / "forge-runtime"

# The package the generated modules are loaded under, so that the relative
# imports the generator writes (`from . import causal_entry`) resolve, as the
# conformance harness loads its fixtures (backends/python/forge-runtime/tests/conftest.py).
PACKAGE = "_sce_cbor_pin"

# Each pinned codec by its document's name, and the class its module declares.
CODECS = {
    "codec_cbor_map": "CodecCborMap",
    "broadcast_log_conformance_text_event": "BroadcastLogConformanceTextEvent",
    "codec_cbor_open_enum": "CodecCborOpenEnum",
    "codec_cbor_signed_enum": "CodecCborSignedEnum",
    "codec_cbor_record_list": "CodecCborRecordList",
    "codec_cbor_record_list_bytes": "CodecCborRecordListBytes",
    "codec_cbor_record_list_least": "CodecCborRecordListLeast",
    "envelope": "Envelope",
}


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    generated, inputs, output = argv
    sys.path.insert(0, str(RUNTIME_SRC))
    from sce_forge_runtime.codec import SceCursor

    package = types.ModuleType(PACKAGE)
    package.__path__ = [generated]
    sys.modules[PACKAGE] = package
    classes = {
        codec: getattr(importlib.import_module(f"{PACKAGE}.{codec}"), name)
        for codec, name in CODECS.items()
    }
    with open(inputs, encoding="utf-8") as source, open(output, "w", encoding="utf-8") as out:
        for line in source:
            if not line.strip():
                continue
            case_id, hexed = line.split()
            codec = case_id.split("/", 1)[0]
            if codec not in classes:
                print(f"capture: {case_id} names no pinned codec", file=sys.stderr)
                return 1
            try:
                value = classes[codec].decode(SceCursor(bytes.fromhex(hexed)))
            except Exception as refusal:  # the decode's refusal, whatever it raises
                out.write(f"{case_id} refused decode {type(refusal).__name__}\n")
                continue
            try:
                encoded = value.encode_to_bytes()
            except Exception as refusal:  # the encode's refusal, whatever it raises
                out.write(f"{case_id} refused encode {type(refusal).__name__}\n")
                continue
            out.write(f"{case_id} ok {encoded.hex()}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
