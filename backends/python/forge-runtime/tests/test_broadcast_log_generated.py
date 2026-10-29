# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The broadcast-log kind's generated modules (ADR 0062), imported and used.
#
# sce-build's tests read what the generator writes for a log and look for text
# in it, which shows the text is what the template says and nothing about
# whether the names in it are the names the runtime and the element's codec
# have, or whether the bytes it packs into an entry read back as the element
# that was published. Here the modules the generator writes for two logs are
# imported over the element modules they import, and the typed `publish` and
# `read` they define are used, on one thread: a rename in the runtime or the
# codec, a width the template computes wrongly, or a byte packed into the wrong
# word fails the import or an assertion. What a log does under several threads
# is the runtime row's own test (test_broadcast_log_runtime.py) and is not
# repeated here.
#
# A log is generated from its document SET and not document by document: an
# entry is sized from the element document, which only the set compile reads.
# So the documents go to `sce-codegen orchestrate` together. The second log's
# retention is a deploy key, resolved against the deploy this test writes.
#
# Python gives every log the progress `blocking` (one ring under one lock), so
# the logs here declare it. They are derived from the documents the Rust
# conformance crate compiles, which declare `wait-free`; deriving them keeps one
# set of log documents in the tree, and the documents themselves are what the
# refusal test hands the generator unchanged.

from __future__ import annotations

import importlib
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path
from typing import Any, Optional

import _sce_codegen
from sce_forge_runtime.broadcast_log import BroadcastLog, Empty, Lagged
from sce_forge_runtime.codec import BufferOverflow, CodecError, LengthOutOfRange

REPO_ROOT = _sce_codegen.REPO_ROOT
RESOURCE_DIR = REPO_ROOT / "tests" / "forge" / "resources"
PACKAGE = "_broadcast_log_generated"

ELEMENTS = ("broadcast_log_conformance_event", "broadcast_log_conformance_record")
LOGS = ("broadcast_log_conformance_literal", "broadcast_log_conformance_deployed")

# The deploy the set is generated for: machine `mcu_node`, whose limit
# `event_retention` is the retention of `broadcast_log_conformance_deployed`.
DEPLOY = """\
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        limits:
          event_retention: 5
"""


def _orchestrate(
    codegen: Path, documents: list[Path], out_dir: Path, deploy: Optional[Path]
) -> subprocess.CompletedProcess[str]:
    command = [str(codegen), "orchestrate"]
    for document in documents:
        command += ["--forge", str(document)]
    command += ["--language", "python", "--output-dir", str(out_dir)]
    if deploy is not None:
        command += ["--deploy", str(deploy), "--target-machine", "mcu_node"]
    return subprocess.run(command, capture_output=True, text=True)


def _copy(name: str, directory: Path) -> Path:
    path = directory / f"{name}.scxml"
    path.write_text((RESOURCE_DIR / f"{name}.scxml").read_text())
    return path


def _blocking(name: str, directory: Path) -> Path:
    """The conformance log, declaring the progress this backend can give."""
    text = (RESOURCE_DIR / f"{name}.scxml").read_text()
    declared = "<sce:progress>wait-free</sce:progress>"
    assert declared in text, f"{name} no longer declares wait-free: derive it from what it declares"
    path = directory / f"{name}.scxml"
    path.write_text(text.replace(declared, "<sce:progress>blocking</sce:progress>"))
    return path


class TestGeneratedBroadcastLogs(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.codegen = _sce_codegen.require()
        cls._tmp = tempfile.TemporaryDirectory()
        sources = Path(cls._tmp.name) / "sources"
        out = Path(cls._tmp.name) / "out"
        sources.mkdir()
        out.mkdir()
        deploy = sources / "deploy.yaml"
        deploy.write_text(DEPLOY)
        documents = [_copy(name, sources) for name in ELEMENTS] + [_blocking(name, sources) for name in LOGS]
        done = _orchestrate(cls.codegen, documents, out, deploy)
        assert done.returncode == 0, f"orchestrate: {done.stdout}{done.stderr}"
        package = types.ModuleType(PACKAGE)
        package.__path__ = [str(out)]
        sys.modules[PACKAGE] = package
        importlib.invalidate_caches()
        cls.event = importlib.import_module(f"{PACKAGE}.broadcast_log_conformance_event")
        cls.record = importlib.import_module(f"{PACKAGE}.broadcast_log_conformance_record")
        cls.literal = importlib.import_module(f"{PACKAGE}.broadcast_log_conformance_literal")
        cls.deployed = importlib.import_module(f"{PACKAGE}.broadcast_log_conformance_deployed")

    @classmethod
    def tearDownClass(cls) -> None:
        cls._tmp.cleanup()

    def make_event(self, sensor_id: int, value: int) -> Any:
        return self.event.BroadcastLogConformanceEvent(sensor_id=sensor_id, value=value)

    def make_record(self, tag: int, payload: bytes) -> Any:
        return self.record.BroadcastLogConformanceRecord(tag=tag, payload=payload)

    def literal_item(self, seq: int, element: Any) -> Any:
        return self.literal.BroadcastLogConformanceLiteralItem(seq, element)

    def deployed_item(self, seq: int, element: Any) -> Any:
        return self.deployed.BroadcastLogConformanceDeployedItem(seq, element)

    # The contract the modules state.

    def test_the_literal_log_states_its_retention_and_sizes_its_entries_from_its_element(self) -> None:
        module = self.literal
        self.assertEqual(3, module.RETENTION, '<sce:retention entries="3"/>')
        self.assertEqual(4, module.SLOTS, "one more than the retention: the slot being overwritten is in flux")
        # The element is an 8-bit id and a 16-bit value: 1 + 2 = 3 bytes.
        self.assertEqual(3, module.MAX_ENCODED_BYTES)
        # One length word, then ceil(3 / 8) = 1 word of bytes: 1 + 1 = 2.
        self.assertEqual(2, module.WORDS)
        self.assertEqual("blocking", module.DECLARED_PROGRESS)
        log = module.new_broadcast_log_conformance_literal()
        self.assertIsInstance(log, BroadcastLog)
        self.assertEqual((module.WORDS, module.SLOTS, module.RETENTION), (log.words, log.slots, log.retention))

    def test_the_deployed_log_takes_its_retention_from_the_deploy_and_its_size_from_the_element(self) -> None:
        module = self.deployed
        # The deploy gives machine mcu_node the limit `event_retention: 5`.
        self.assertEqual(5, module.RETENTION)
        self.assertEqual(6, module.SLOTS)
        # A one-byte tag and a payload of at most eight bytes: 1 + 8 = 9 bytes.
        self.assertEqual(9, module.MAX_ENCODED_BYTES)
        # One length word, then ceil(9 / 8) = 2 words of bytes: 1 + 2 = 3.
        self.assertEqual(3, module.WORDS)
        self.assertEqual("blocking", module.DECLARED_PROGRESS)
        log = module.new_broadcast_log_conformance_deployed()
        self.assertEqual((module.WORDS, module.SLOTS), (log.words, log.slots))

    # The log it defines is the log the runtime has.

    def test_entries_come_back_whole_and_in_order_through_the_typed_functions(self) -> None:
        module = self.literal
        log = module.new_broadcast_log_conformance_literal()
        publisher = log.publisher()
        self.assertIsNotNone(publisher)
        reader = log.reader_oldest()
        self.assertEqual(Empty(), module.read(log, reader))

        # Values that fill every byte of the packed word and differ in each.
        sent = [self.make_event(1, 0x0102), self.make_event(2, 0xFFEE), self.make_event(255, 0)]
        for n, element in enumerate(sent):
            self.assertEqual(n, module.publish(publisher, element), "an entry takes the next sequence number")
        for n, element in enumerate(sent):
            self.assertEqual(
                self.literal_item(n, element),
                module.read(log, reader),
                f"entry {n} comes back whole, in the order it was published",
            )
        self.assertEqual(Empty(), module.read(log, reader))
        self.assertEqual(0, reader.lost, "a reader that kept up lost nothing")

    def test_a_reader_the_publisher_laps_is_told_how_many_it_missed_and_resumes_at_the_oldest(self) -> None:
        module = self.literal
        log = module.new_broadcast_log_conformance_literal()
        publisher = log.publisher()
        slow = log.reader_oldest()

        # Six entries, sequence numbers 0 to 5, into a log that retains three: the
        # window is [6 - 3, 6) = [3, 6), so the reader at 0 missed 3 - 0 = 3.
        for n in range(6):
            module.publish(publisher, self.make_event(9, n))
        self.assertEqual(Lagged(3), module.read(log, slow))
        self.assertEqual(3, slow.cursor, "it resumes at the oldest entry held")
        self.assertEqual(3, slow.lost)

        for n in range(3, 6):
            self.assertEqual(self.literal_item(n, self.make_event(9, n)), module.read(log, slow))
        self.assertEqual(Empty(), module.read(log, slow))
        self.assertEqual(3, slow.lost, "reading what is held loses nothing more")

    def test_two_readers_keep_separate_cursors(self) -> None:
        module = self.literal
        log = module.new_broadcast_log_conformance_literal()
        publisher = log.publisher()
        module.publish(publisher, self.make_event(1, 10))
        module.publish(publisher, self.make_event(2, 20))

        # Opened at the oldest entry held (0, as nothing was forgotten), at
        # sequence 1, and at the head (2).
        first = log.reader_oldest()
        second = log.reader_at(1)
        late = log.reader_head()

        self.assertEqual(self.literal_item(0, self.make_event(1, 10)), module.read(log, first))
        self.assertEqual(1, first.cursor)
        self.assertEqual(1, second.cursor, "reading with one reader moves no other")

        self.assertEqual(self.literal_item(1, self.make_event(2, 20)), module.read(log, second))
        self.assertEqual(
            self.literal_item(1, self.make_event(2, 20)),
            module.read(log, first),
            "the entry the second reader took is still there for the first",
        )
        self.assertEqual(Empty(), module.read(log, first))
        self.assertEqual(Empty(), module.read(log, second))

        # A reader opened at the head sees only what is published after it.
        self.assertEqual(Empty(), module.read(log, late))
        module.publish(publisher, self.make_event(3, 30))
        self.assertEqual(self.literal_item(2, self.make_event(3, 30)), module.read(log, late))

    def test_an_entry_is_its_length_and_then_its_bytes_eight_to_a_word_little_endian(self) -> None:
        module = self.deployed
        log = module.new_broadcast_log_conformance_deployed()
        publisher = log.publisher()
        module.publish(publisher, self.make_record(0x5A, bytes([1, 2, 3, 4, 5, 6, 7, 8])))
        module.publish(publisher, self.make_record(0x5B, bytes([7, 7, 7])))

        reader = log.reader_oldest()
        words = [0] * module.WORDS
        log.read(reader, words)
        # 9 bytes: 5A 01 02 03 04 05 06 07 | 08. The first word holds the first
        # eight, the first byte least significant; the second holds the ninth.
        self.assertEqual([9, 0x070605040302015A, 0x08], words)
        log.read(reader, words)
        # 4 bytes: 5B 07 07 07, then zeros: the word is 0x0707075B and the second
        # word stays zero.
        self.assertEqual([4, 0x0707075B, 0], words)

    def test_an_element_larger_than_its_bound_is_refused_by_the_codec_and_not_truncated(self) -> None:
        module = self.deployed
        log = module.new_broadcast_log_conformance_deployed()
        publisher = log.publisher()

        # 1 + 20 = 21 bytes into a log whose entries hold at most 9.
        with self.assertRaises(BufferOverflow):
            module.publish(publisher, self.make_record(1, bytes([0xAA] * 20)))
        self.assertEqual(0, log.head(), "a refused element publishes nothing")

        # Exactly the bound, and under it: the length word keeps what was written.
        full = self.make_record(0x5A, bytes([1, 2, 3, 4, 5, 6, 7, 8]))
        short = self.make_record(0x5B, bytes([7, 7, 7]))
        self.assertEqual(0, module.publish(publisher, full))
        self.assertEqual(1, module.publish(publisher, short))

        reader = log.reader_oldest()
        self.assertEqual(self.deployed_item(0, full), module.read(log, reader))
        self.assertEqual(self.deployed_item(1, short), module.read(log, reader))
        self.assertEqual(Empty(), module.read(log, reader))

    def test_a_stored_length_above_the_bound_is_a_codec_error_and_never_a_cut(self) -> None:
        module = self.literal
        # The runtime row takes any words, so an entry whose length word is not
        # one this log wrote (a corrupt slot, a log of another size) can be
        # offered to the typed read. A length of MAX_ENCODED_BYTES + 1 is the
        # nearest; the largest 64-bit value is the farthest.
        for stored in (module.MAX_ENCODED_BYTES + 1, 2**64 - 1):
            with self.subTest(stored=stored):
                log = module.new_broadcast_log_conformance_literal()
                publisher = log.publisher()
                publisher.publish([stored] + [0] * (module.WORDS - 1))
                reader = log.reader_oldest()
                with self.assertRaises(LengthOutOfRange):
                    module.read(log, reader)
                self.assertEqual(1, reader.cursor, "the reader moved past the entry")

    def test_stored_bytes_the_codec_does_not_decode_are_a_codec_error(self) -> None:
        module = self.literal
        log = module.new_broadcast_log_conformance_literal()
        publisher = log.publisher()
        # Two bytes are within the bound and too few for the three the element
        # takes, so the codec's decode returns nothing.
        publisher.publish([2, 0x0102])
        reader = log.reader_oldest()
        with self.assertRaises(CodecError) as raised:
            module.read(log, reader)
        self.assertIs(CodecError, type(raised.exception), "the length was within the bound, so it is not that error")
        self.assertEqual(1, reader.cursor, "the reader moved past the entry")

    def test_a_document_declaring_more_than_blocking_is_refused_for_python_by_name(self) -> None:
        # The documents as the Rust conformance crate compiles them declare
        # `wait-free`, which one ring under one lock cannot give.
        with tempfile.TemporaryDirectory() as scratch:
            sources = Path(scratch) / "sources"
            out = Path(scratch) / "out"
            sources.mkdir()
            out.mkdir()
            documents = [
                _copy("broadcast_log_conformance_event", sources),
                _copy("broadcast_log_conformance_literal", sources),
            ]
            done = _orchestrate(self.codegen, documents, out, None)
            self.assertNotEqual(0, done.returncode, "the document is valid, and refused for this backend")
            said = done.stdout + done.stderr
            self.assertIn("cannot be met on the python backend", said)
            self.assertIn("<sce:progress>wait-free</sce:progress>", said)


if __name__ == "__main__":
    unittest.main()
