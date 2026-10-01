"""A design's machine is played in a process of its own, and only data comes back.

`verify` and `compare` used to import the module a design was lowered to and call
it in the server's process. They now reach it through `sandbox`: a worker process
holds the module and the objects it makes, under the kernel's limits and a clock,
and the server holds references. These cases hold the boundary itself, with a
module written by hand in the shape the generator writes (`<stem>_sm.py`), so
they need no generator and run in the domain-free job too.

What is held here:

  * every kind of value that may cross comes back as the same value, and the
    kinds that may not (an enumeration member, an engine) come back as references;
  * an exception the design raises is raised here under the same name;
  * a design that never returns, spins in native code, allocates without end or
    prints without end costs its own worker and nothing else;
  * once a worker is stopped it is not asked again;
  * a worker never outlives the call that started it, nor the server.
"""

from __future__ import annotations

import gc
import math
import os
import pathlib
import subprocess
import sys
import tempfile
import time
import unittest

from sce_author import process, sandbox, wire
from sce_author.errors import VerifyError

TOOLS = pathlib.Path(__file__).resolve().parents[1]

# A module in the shape the generator writes, with the behaviours a design can
# have that a boundary has to survive.
TOY = '''
import dataclasses
import enum


class Color(enum.IntEnum):
    RED = 1


class Odd(Exception):
    pass


@dataclasses.dataclass
class Rec:
    a: int
    b: str
    c: tuple


class Counter:
    def __init__(self):
        self.n = 0
        self.tags = {"k": 1}

    def bump(self, by=1):
        self.n += by
        return self.n

    @property
    def value(self):
        return self.n

    def boom(self):
        raise ValueError("no thanks")

    def odd(self):
        raise Odd("custom")

    def same(self, other):
        return other is self


def add(a, b):
    return a + b


def make():
    return Counter()


def rec():
    return Rec(1, "x", (2, 3))


def mixed():
    return (1, (2, 3), {4, 5}, {"k": b"by", "$t": 7}, {1: "one"}, [True, None, 1.5])


def floats():
    return [float("nan"), float("inf"), float("-inf"), -0.0]


def color():
    return Color.RED


def noisy():
    print("this must not reach the protocol")
    return "quiet"


def echo(value):
    return value


def hang():
    while True:
        pass


def spin_native():
    b = b"a" * 50_000_000
    while True:
        b.find(b"z")


def eat():
    x = []
    while True:
        x.append(bytearray(10_000_000))


def big(n):
    return "x" * n


def deep(n):
    value = []
    for _ in range(n):
        value = [value]
    return value


SCE_HOST_NAMES = {"readers": {"n": "bump"}}
'''


def alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


def gone_within(pid: int, seconds: float) -> bool:
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if not alive(pid):
            return True
        time.sleep(0.05)
    return not alive(pid)


class Toy(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.built = pathlib.Path(self.directory.name) / "built"
        self.built.mkdir()
        (self.built / "toy_sm.py").write_text(TOY, encoding="utf-8")
        self.reaping = process.reaping()
        self.reaping.__enter__()
        self.addCleanup(self.reaping.__exit__, None, None, None)

    def load(self, **limits) -> sandbox.RemoteModule:
        bounds = process.Limits(**{**dict(wall_seconds=20.0, cpu_seconds=60, memory_mb=1024,
                                          output_mb=4), **limits})
        return sandbox.load_module(self.built, pathlib.Path("toy.scxml"), limits=bounds)


class TestWhatCrossesAsAValue(Toy):
    def test_the_kinds_of_value_come_back_as_they_went(self):
        module = self.load()
        self.assertEqual(3, module.add(1, 2))
        self.assertEqual("ab", module.add("a", "b"))
        got = module.mixed()
        self.assertEqual(
            (1, (2, 3), {4, 5}, {"k": b"by", "$t": 7}, {1: "one"}, [True, None, 1.5]), got)
        self.assertIs(tuple, type(got[1]))
        self.assertIs(set, type(got[2]))
        self.assertIs(bool, type(got[5][0]))

    def test_a_number_that_is_not_finite_survives(self):
        nan, inf, minus, zero = self.load().floats()
        self.assertTrue(math.isnan(nan))
        self.assertEqual((math.inf, -math.inf), (inf, minus))
        self.assertEqual("-0.0", repr(zero))

    def test_a_dataclass_is_copied_field_by_field(self):
        copy = self.load().rec()
        self.assertEqual((1, "x", (2, 3)), (copy.a, copy.b, copy.c))

    def test_a_dictionary_with_a_key_that_looks_like_a_tag_is_not_taken_for_one(self):
        """A design's dictionary may have any keys. One named like a protocol tag
        would otherwise be read as a tuple, a set or a reference."""
        sent = {"$ref": 1, "$t": [1], "$m": ["x", "y"]}
        self.assertEqual(sent, self.load().echo(sent))

    def test_an_enumeration_member_is_a_reference_not_a_number(self):
        """`Color.RED` is an `int` by inheritance and an object by identity. It
        stays where it is, and the same member is the same reference."""
        module = self.load()
        first = module.color()
        self.assertIsInstance(first, sandbox.Remote)
        self.assertEqual(first, module.color())
        self.assertEqual(hash(first), hash(module.color()))
        self.assertNotEqual(first, module.make())

    def test_a_counter_is_driven_through_references_and_methods(self):
        module = self.load()
        counter = module.make()
        self.assertEqual(1, counter.bump())
        self.assertEqual(3, counter.bump(2))
        self.assertEqual(3, counter.value)
        self.assertEqual({"n": 3, "tags": {"k": 1}}, sandbox.state_of(counter))
        self.assertTrue(counter.same(counter))
        self.assertFalse(counter.same(module.make()))

    def test_stdout_of_the_design_does_not_reach_the_protocol(self):
        module = self.load()
        self.assertEqual("quiet", module.noisy())
        self.assertEqual(7, module.add(3, 4))

    def test_the_names_a_module_publishes_come_across_whole(self):
        self.assertEqual({"readers": {"n": "bump"}}, self.load().SCE_HOST_NAMES)


class TestWhatIsRaised(Toy):
    def test_a_builtin_the_design_raises_is_raised_here_under_the_same_name(self):
        counter = self.load().make()
        with self.assertRaises(ValueError) as caught:
            counter.boom()
        self.assertEqual("no thanks", str(caught.exception))

    def test_an_exception_of_the_designs_own_keeps_its_name(self):
        counter = self.load().make()
        with self.assertRaises(sandbox.RemoteError) as caught:
            counter.odd()
        self.assertEqual("Odd", type(caught.exception).__name__)
        self.assertEqual("custom", str(caught.exception))

    def test_a_missing_attribute_is_an_attribute_error_so_hasattr_works(self):
        module = self.load()
        self.assertFalse(hasattr(module, "no_such_thing"))
        self.assertIsNone(getattr(module, "no_such_thing", None))
        self.assertTrue(hasattr(module, "add"))

    def test_a_value_that_cannot_be_sent_is_refused_and_says_so(self):
        module = self.load()
        with self.assertRaises(VerifyError) as caught:
            module.echo(object())
        self.assertIn("cannot be sent", str(caught.exception))

    def test_a_module_that_does_not_import_is_the_loaders_words(self):
        (self.built / "toy_sm.py").write_text("x = 1\ntarget = State.A B\n", encoding="utf-8")
        with self.assertRaises(VerifyError) as caught:
            self.load()
        said = str(caught.exception)
        for words in ("toy_sm.py:2", "SyntaxError", "does not import", "defect in the generator"):
            self.assertIn(words, said)

    def test_a_reference_the_parent_gave_up_is_dropped_by_the_worker(self):
        """The worker holds an object only as long as the parent does. Dropping
        the last reference rides on the next request, and the number is then
        unknown to the worker."""
        module = self.load()
        counter = module.make()
        number = counter._number
        worker = counter._worker
        del counter
        gc.collect()
        module.add(1, 1)
        with self.assertRaises(VerifyError) as caught:
            worker.request("getattr", ref=number, name="n")
        self.assertIn("does not hold", str(caught.exception))


class TestAWorkerThatIsStopped(Toy):
    def test_a_design_that_never_returns_is_stopped_by_the_clock(self):
        module = self.load(wall_seconds=1.5)
        started = time.monotonic()
        with self.assertRaises(sandbox.WorkerStopped) as caught:
            module.hang()
        self.assertLess(time.monotonic() - started, 10)
        self.assertEqual("wall-clock", caught.exception.stopped_by)
        self.assertEqual("environment", caught.exception.cause)

    def test_a_stopped_worker_is_not_asked_again_and_says_the_same(self):
        module = self.load(wall_seconds=1.5)
        with self.assertRaises(sandbox.WorkerStopped):
            module.hang()
        started = time.monotonic()
        with self.assertRaises(sandbox.WorkerStopped) as again:
            module.add(1, 2)
        self.assertLess(time.monotonic() - started, 1)
        self.assertEqual("wall-clock", again.exception.stopped_by)

    def test_a_stopped_worker_is_a_verify_error_and_not_a_value_the_design_raised(self):
        module = self.load(wall_seconds=1.5)
        with self.assertRaises(VerifyError):
            module.hang()

    @unittest.skipUnless(sys.platform.startswith("linux"), "the kernel limits are Linux's here")
    def test_native_spin_is_stopped_by_processor_time(self):
        """The loop never returns to the interpreter, so no timer in that process
        could end it. The kernel's processor limit does."""
        module = self.load(wall_seconds=30.0, cpu_seconds=1)
        with self.assertRaises(sandbox.WorkerStopped) as caught:
            module.spin_native()
        self.assertEqual("cpu", caught.exception.stopped_by)

    @unittest.skipUnless(sys.platform.startswith("linux"), "the kernel limits are Linux's here")
    def test_allocating_without_end_is_stopped_by_the_memory_limit(self):
        module = self.load(memory_mb=300)
        try:
            module.eat()
        except (MemoryError, sandbox.WorkerStopped) as stopped:
            # Either the interpreter reports it as an exception the design raised,
            # or the kernel ends the process; neither is an answer to be believed.
            if isinstance(stopped, sandbox.WorkerStopped):
                self.assertIn(stopped.stopped_by, ("memory", "crash"))
        else:
            self.fail("allocating without end was not stopped")

    @unittest.skipUnless(sys.platform.startswith("linux"), "the kernel limits are Linux's here")
    def test_an_answer_larger_than_may_be_sent_stops_the_worker(self):
        module = self.load(output_mb=1)
        with self.assertRaises(sandbox.WorkerStopped) as caught:
            module.big(5_000_000)
        self.assertEqual("output", caught.exception.stopped_by)

    def test_a_value_nested_without_end_is_refused_not_followed(self):
        module = self.load()
        with self.assertRaises(VerifyError) as caught:
            module.deep(wire.MAX_DEPTH + 50)
        self.assertIn("deep", str(caught.exception))
        self.assertEqual(2, module.add(1, 1))


class TestWhereAWorkerLives(Toy):
    def test_a_worker_is_closed_when_the_block_that_started_it_ends(self):
        with process.reaping():
            module = self.load()
            pid = module._worker.session.child.pid
            self.assertTrue(alive(pid))
        self.assertTrue(gone_within(pid, 3))

    def test_the_server_does_not_import_what_the_worker_imported(self):
        before = set(sys.modules)
        module = self.load()
        module.add(1, 2)
        module.color()
        new = {name for name in set(sys.modules) - before}
        self.assertEqual([], sorted(name for name in new
                                    if "toy_sm" in name or name.startswith("sce_runtime")
                                    or name.split(".")[0] == "lupa"))

    @unittest.skipUnless(sys.platform.startswith("linux"), "die-with-the-parent is Linux's")
    def test_a_worker_does_not_outlive_a_server_that_dies_without_cleaning_up(self):
        """The server may be killed. The worker asked to die with it, so a design
        is never left running by a process that is no longer there to stop it."""
        script = (
            "import os, pathlib, sys\n"
            f"sys.path.insert(0, {str(TOOLS)!r})\n"
            "from sce_author import process, sandbox\n"
            f"module = sandbox.load_module(pathlib.Path({str(self.built)!r}), pathlib.Path('toy.scxml'))\n"
            "print(module._worker.session.child.pid, flush=True)\n"
            "os._exit(0)\n")
        done = subprocess.run([sys.executable, "-c", script], capture_output=True, text=True,
                              timeout=60)
        self.assertEqual(0, done.returncode, done.stderr)
        pid = int(done.stdout.split()[0])
        self.assertTrue(gone_within(pid, 5), "the worker was still running")


class TestTheCodec(unittest.TestCase):
    """The codec without a process, for the shapes that are hard to provoke."""

    def hook(self, obj):
        return {"$ref": 1}

    def round(self, value):
        return wire.decode(wire.encode(value, self.hook), lambda tag, data: data)

    def test_equal_values_survive(self):
        for value in (None, True, 0, 7, -3, 2 ** 80, 1.5, "", "ü", [], [1, [2]], (), (1, (2,)),
                      {1, 2}, frozenset(), {}, {"a": 1}, {1: 2}, {(1, 2): [3]}, b"", b"\x00\xff"):
            with self.subTest(value=value):
                got = self.round(value)
                self.assertEqual(value if not isinstance(value, frozenset) else set(), got)
                self.assertEqual(type(value) if type(value) is not frozenset else set, type(got))

    def test_a_bool_is_not_turned_into_a_number_nor_the_reverse(self):
        self.assertIs(bool, type(self.round(True)))
        self.assertIs(int, type(self.round(1)))

    def test_a_subclass_of_a_plain_type_is_an_object_not_a_copy(self):
        class Loud(str):
            pass

        self.assertEqual({"$ref": 1}, wire.encode(Loud("x"), self.hook))

    def test_nesting_past_the_limit_is_refused(self):
        value = []
        for _ in range(wire.MAX_DEPTH + 5):
            value = [value]
        with self.assertRaises(wire.WireError):
            wire.encode(value, self.hook)

    def test_a_tag_the_protocol_does_not_have_is_refused(self):
        with self.assertRaises(wire.WireError):
            wire.decode({"$eval": "1+1"}, lambda tag, data: data)

    def test_something_json_does_not_carry_is_refused(self):
        with self.assertRaises(wire.WireError):
            wire.decode(object(), lambda tag, data: data)


if __name__ == "__main__":
    unittest.main()
