# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# RFC variant-default-uniformity python half — runtime
# round-trip property test. Mirrors
# sce-forge-runtime/rust/tests/forge_default_round_trip.rs for the
# Python backend: compiles the 3 default-marker fixtures into a
# temp output dir and asserts ``T().encode() → T.decode() → same arm``
# with byte-equal re-encode.
#
# These fixtures live outside the numerical-conformance manifest
# because their purpose is contract testing (Default emission), not
# numerical oracle comparison.

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
RESOURCE_DIR = REPO_ROOT / "tests" / "forge" / "resources"
SCE_CODEGEN = REPO_ROOT / "target" / "release" / "sce-codegen"


def _generate(out_dir: Path, fixture: str) -> None:
    """Invoke sce-codegen on a single fixture, writing the .py output
    into ``out_dir``. Same CLI shape as the existing conftest bootstrap;
    we just narrow the manifest scope to a single SCXML."""
    subprocess.run(
        [
            str(SCE_CODEGEN),
            "generate",
            str(RESOURCE_DIR / f"{fixture}.scxml"),
            "--language",
            "python",
            "--output-dir",
            str(out_dir),
        ],
        check=True,
        capture_output=True,
    )


def _load_module(name: str, path: Path, package: types.ModuleType) -> types.ModuleType:
    """Load a generated codec module under ``package`` so the outer
    codec's ``from . import codec_default_marker_arm_a`` relative
    imports resolve."""
    spec = importlib.util.spec_from_file_location(
        f"{package.__name__}.{name}",
        path,
    )
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load spec for {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class TestDefaultRoundTrip(unittest.TestCase):
    """Critical invariants:
      1. ``Outer().encode()`` produces 3 wire bytes (arm B's 1-byte
         header + 2-byte uint16 payload).
      2. ``bytes[0] & 0x03 == 0x02`` — first byte's dispatch slice
         matches arm B's MID.
      3. ``Outer.decode(SceCursor(bytes))`` consumes everything and
         lands in arm B.
      4. ``decoded.encode() == original.encode()`` (byte-stable).
    """

    @classmethod
    def setUpClass(cls):
        if not SCE_CODEGEN.exists():
            raise unittest.SkipTest(
                f"sce-codegen binary not found at {SCE_CODEGEN}; "
                "run `cargo build --bin sce-codegen --features cli "
                "--release -p sce-build`"
            )
        cls._tmp = tempfile.TemporaryDirectory()
        cls._out = Path(cls._tmp.name)
        for fixture in (
            "codec_default_marker_arm_a",
            "codec_default_marker_arm_b",
            "codec_variant_default_marker",
        ):
            _generate(cls._out, fixture)
        # Install a synthetic parent package so the outer's
        # ``from .codec_default_marker_arm_a import …`` relative
        # imports resolve against our generated arm modules.
        pkg_name = "_round_trip_codecs"
        pkg = types.ModuleType(pkg_name)
        pkg.__path__ = [str(cls._out)]
        sys.modules[pkg_name] = pkg
        cls._arm_a = _load_module("codec_default_marker_arm_a", cls._out / "codec_default_marker_arm_a.py", pkg)
        cls._arm_b = _load_module("codec_default_marker_arm_b", cls._out / "codec_default_marker_arm_b.py", pkg)
        cls._outer = _load_module("codec_variant_default_marker", cls._out / "codec_variant_default_marker.py", pkg)

    @classmethod
    def tearDownClass(cls):
        cls._tmp.cleanup()

    def test_round_trip_lands_in_declared_default_arm(self):
        from sce_forge_runtime.codec import (
            BufferOverflow,
            BytearraySink,
            MemoryviewSink,
            SceCursor,
        )

        Outer = self._outer.CodecVariantDefaultMarker
        original = Outer()
        wire = original.encode_to_bytes()

        self.assertEqual(
            len(wire),
            3,
            f"default-emit + arm B (uint16 payload) must produce 3 wire bytes; got {len(wire)}: {wire!r}",
        )
        self.assertEqual(
            wire[0] & 0x03,
            0x02,
            f"first byte low 2 bits must encode arm B's MID (0x02); got 0x{wire[0]:02X}",
        )

        cursor = SceCursor(wire)
        decoded = Outer.decode(cursor)
        self.assertIsNotNone(
            decoded,
            "freshly-constructed codec must decode without returning None",
        )
        self.assertEqual(
            cursor.remaining(),
            0,
            f"decode must consume every emitted byte; {cursor.remaining()} byte(s) leftover",
        )
        # Arm B's @dataclass field on the Variant is named after the
        # imported codec's package: codec_default_marker_arm_b.
        self.assertEqual(
            decoded.body.kind,
            "CodecDefaultMarkerArmB",
            f"round-trip must land in arm B (the marked-default); got kind={decoded.body.kind!r}",
        )

        re_encoded = decoded.encode_to_bytes()
        self.assertEqual(
            wire,
            re_encoded,
            "decode → encode must produce byte-equal output (round-trip stability)",
        )

        # RFC §synth-5-B writer-direct path: BytearraySink-backed encode
        # must produce bytes equal to the facade output.
        dst = bytearray()
        result = decoded.encode(BytearraySink(dst))
        self.assertIsNone(result, "BytearraySink-backed encode returns None")
        self.assertEqual(
            wire,
            bytes(dst),
            "BytearraySink encode must equal facade encode_to_bytes output",
        )

        # MemoryviewSink with sufficient capacity — same bytes.
        buf = bytearray(16)
        ms = MemoryviewSink(memoryview(buf))
        decoded.encode(ms)
        self.assertEqual(ms.position(), len(wire),
                         "MemoryviewSink position must equal wire length")
        self.assertEqual(wire, bytes(buf[:ms.position()]),
                         "MemoryviewSink encode prefix must equal wire bytes")

        # Bounded-buffer BufferOverflow path: a MemoryviewSink sized
        # strictly smaller than the actual wire length must raise.
        if len(wire) > 0:
            tiny = bytearray(len(wire) - 1)
            tiny_sink = MemoryviewSink(memoryview(tiny))
            with self.assertRaises(BufferOverflow):
                decoded.encode(tiny_sink)


if __name__ == "__main__":
    unittest.main()
