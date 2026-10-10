# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The queue kind's generated modules (SCE Protocol-Synthesis RFC §synth-5-P),
# imported and used.
#
# sce-build's tests read what the generator writes for a queue and look for text
# in it, which shows the text is what the template says and nothing about
# whether the names in it are the names the runtime has. Here the module the
# generator writes is imported over the element module it imports, and the queue
# it defines is used: a rename in the runtime, or a constant the template
# computes wrongly, fails the import or an assertion.
#
# Python gives every queue the progress `blocking` (one ring under one lock), so
# the documents here declare it. They are derived from the conformance fixtures
# that every other backend compiles, which declare `wait-free` and `lock-free`;
# deriving them keeps one set of documents in the tree, and the fixtures
# themselves are what the refusal test hands the generator unchanged.

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path

import _sce_codegen
from sce_forge_runtime.queue import Progress, PushStatus, Queue, SegmentedQueue
from test_queue_runtime import BudgetAllocator

REPO_ROOT = _sce_codegen.REPO_ROOT
RESOURCE_DIR = REPO_ROOT / "tests" / "forge" / "resources"
PACKAGE = "_queue_generated"


def _generate(codegen: Path, document: Path, out_dir: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(codegen), "generate", str(document), "--language", "python", "--output-dir", str(out_dir)],
        capture_output=True,
        text=True,
    )


def _load(name: str, path: Path, package: types.ModuleType) -> types.ModuleType:
    spec = importlib.util.spec_from_file_location(f"{package.__name__}.{name}", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _blocking(fixture: str, name: str, directory: Path) -> Path:
    """The conformance fixture, renamed and declaring the progress this backend
    can give."""
    text = (RESOURCE_DIR / f"{fixture}.scxml").read_text()
    for declared in ("wait-free", "lock-free"):
        text = text.replace(f"<sce:progress>{declared}</sce:progress>", "<sce:progress>blocking</sce:progress>")
    # A segmented fixture also declares what its allocator gives: this backend's
    # push is blocking, and the allocator the tests inject says so.
    for declared in ("wait-free", "lock-free"):
        text = text.replace(f'allocator-progress="{declared}"', 'allocator-progress="blocking"')
    text = text.replace(f'name="{fixture}"', f'name="{name}"')
    path = directory / f"{name}.scxml"
    path.write_text(text)
    return path


class TestGeneratedQueues(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.codegen = _sce_codegen.require()
        cls._tmp = tempfile.TemporaryDirectory()
        sources = Path(cls._tmp.name) / "sources"
        out = Path(cls._tmp.name) / "out"
        sources.mkdir()
        out.mkdir()
        documents = [
            RESOURCE_DIR / "queue_conformance_event.scxml",
            _blocking("queue_conformance_spsc", "queue_blocking_one_one", sources),
            _blocking("queue_conformance_scq", "queue_blocking_many_many", sources),
            _blocking("queue_conformance_segmented", "queue_blocking_segmented_one_one", sources),
            _blocking("queue_conformance_segmented_many", "queue_blocking_segmented_many", sources),
        ]
        for document in documents:
            done = _generate(cls.codegen, document, out)
            assert done.returncode == 0, f"{document.name}: {done.stderr}"
        package = types.ModuleType(PACKAGE)
        package.__path__ = [str(out)]
        sys.modules[PACKAGE] = package
        cls.event = _load("queue_conformance_event", out / "queue_conformance_event.py", package)
        cls.one_one = _load("queue_blocking_one_one", out / "queue_blocking_one_one.py", package)
        cls.many_many = _load("queue_blocking_many_many", out / "queue_blocking_many_many.py", package)
        cls.segmented_one_one = _load(
            "queue_blocking_segmented_one_one", out / "queue_blocking_segmented_one_one.py", package
        )
        cls.segmented_many = _load("queue_blocking_segmented_many", out / "queue_blocking_segmented_many.py", package)

    @classmethod
    def tearDownClass(cls) -> None:
        cls._tmp.cleanup()

    def test_the_module_states_what_the_document_required_and_what_it_gives(self) -> None:
        module = self.one_one
        self.assertEqual(4, module.CAPACITY, "the capacity is the document's: four")
        self.assertEqual((1, 1), (module.PRODUCER_PLACES, module.CONSUMER_PLACES))
        for progress in (module.DECLARED_PROGRESS, module.PUSH_PROGRESS, module.POP_PROGRESS):
            self.assertEqual("blocking", progress)
        self.assertIn("lock", module.ALGORITHM)
        queue = module.new_queue_blocking_one_one()
        self.assertIsInstance(queue, Queue)
        self.assertEqual(module.CAPACITY, queue.capacity)

    def test_a_many_side_has_the_documents_participants_as_its_places(self) -> None:
        module = self.many_many
        self.assertEqual(6, module.CAPACITY)
        self.assertEqual(3, module.PARTICIPANTS)
        self.assertEqual((3, 3), (module.PRODUCER_PLACES, module.CONSUMER_PLACES))
        queue = module.new_queue_blocking_many_many()
        handles = [queue.producer() for _ in range(module.PARTICIPANTS)]
        self.assertTrue(all(handle is not None for handle in handles))
        self.assertIsNone(queue.producer(), "a fourth producer has no place")

    def test_the_queue_hands_elements_over_in_order_and_refuses_when_full(self) -> None:
        for module, name in (
            (self.one_one, "new_queue_blocking_one_one"),
            (self.many_many, "new_queue_blocking_many_many"),
        ):
            with self.subTest(module=module.__name__):
                queue = getattr(module, name)()
                producer, consumer = queue.producer(), queue.consumer()
                self.assertIsNone(consumer.try_pop(), "a new queue is empty")
                for i in range(module.CAPACITY):
                    status = producer.try_push(self.event.QueueConformanceEvent(sensor_id=1, value=i))
                    self.assertIs(PushStatus.OK, status, f"push {i} of {module.CAPACITY} must fit")
                full = producer.try_push(self.event.QueueConformanceEvent(sensor_id=9, value=99))
                self.assertIs(PushStatus.FULL, full, "a full queue refuses the push")
                for i in range(module.CAPACITY):
                    got = consumer.try_pop()
                    self.assertIsNotNone(got, f"pop {i}")
                    self.assertEqual(i, got.value, f"pop {i}: first in, first out")
                self.assertIsNone(consumer.try_pop(), "drained")

    def test_the_segmented_modules_state_the_shape_and_the_allocator_they_need(self) -> None:
        one, many = self.segmented_one_one, self.segmented_many
        self.assertEqual(3, one.SEGMENT)
        self.assertEqual(3, many.SEGMENT)
        self.assertEqual(2, many.PARTICIPANTS)
        self.assertEqual((1, 1), (one.PRODUCER_PLACES, one.CONSUMER_PLACES))
        self.assertEqual((2, 2), (many.PRODUCER_PLACES, many.CONSUMER_PLACES))
        for module in (one, many):
            self.assertEqual("blocking", module.ALLOCATOR_PROGRESS_WORD)
            self.assertIs(Progress.BLOCKING, module.ALLOCATOR_PROGRESS)
            self.assertEqual("blocking", module.PUSH_PROGRESS)
            self.assertIn("lock", module.ALGORITHM)
            self.assertFalse(hasattr(module, "CAPACITY"), "a segmented queue has no capacity")

    def test_a_segmented_queue_hands_over_across_segments_and_reports_a_refused_segment(self) -> None:
        for module, name in (
            (self.segmented_one_one, "new_queue_blocking_segmented_one_one"),
            (self.segmented_many, "new_queue_blocking_segmented_many"),
        ):
            with self.subTest(module=module.__name__):
                # Two segments of three: six elements, then the allocator refuses.
                queue = getattr(module, name)(BudgetAllocator(2))
                self.assertIsInstance(queue, SegmentedQueue)
                producer, consumer = queue.producer(), queue.consumer()
                self.assertIsNone(consumer.try_pop(), "a new queue is empty")
                for i in range(2 * module.SEGMENT):
                    status = producer.try_push(self.event.QueueConformanceEvent(sensor_id=1, value=i))
                    self.assertIs(PushStatus.OK, status, f"push {i} fits the two segments")
                refused = producer.try_push(self.event.QueueConformanceEvent(sensor_id=9, value=99))
                self.assertIs(PushStatus.OUT_OF_MEMORY, refused, "the allocator refuses a third segment")
                for i in range(2 * module.SEGMENT):
                    got = consumer.try_pop()
                    self.assertIsNotNone(got, f"pop {i}")
                    self.assertEqual(i, got.value, f"pop {i}: first in, first out")
                self.assertIsNone(consumer.try_pop(), "drained")

    def test_a_segmented_module_refuses_an_allocator_that_gives_less_than_declared(self) -> None:
        # The fixtures declare a blocking allocator, which every allocator meets;
        # a module that declared more is checked through the runtime directly.
        queue = self.segmented_one_one.new_queue_blocking_segmented_one_one(BudgetAllocator(0))
        self.assertIsNone(queue, "an allocator with no segment to give")

    def test_a_document_declaring_more_than_blocking_is_refused_for_python_by_name(self) -> None:
        for fixture, declared in (("queue_conformance_spsc", "wait-free"), ("queue_conformance_scq", "lock-free")):
            with self.subTest(fixture=fixture), tempfile.TemporaryDirectory() as out:
                done = _generate(self.codegen, RESOURCE_DIR / f"{fixture}.scxml", Path(out))
                self.assertNotEqual(0, done.returncode, "the document is valid, and refused for this backend")
                self.assertIn("cannot be met on the python backend", done.stderr)
                self.assertIn(f"<sce:progress>{declared}</sce:progress>", done.stderr)


if __name__ == "__main__":
    unittest.main()
