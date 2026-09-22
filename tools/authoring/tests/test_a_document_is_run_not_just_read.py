"""The loop closes here: the document is RUN against the pack's examples.

Everything else in this package reads. `questions` reads a specification and
says what it failed to answer; `check` reads a document and says whether its
names are real. A document can satisfy `check` completely and compute the
wrong answer at every address, because `check` never executes anything.

So until this existed, an author -- or a model writing for one -- produced a
document and got no verdict on whether it worked. The only judge of behaviour
was a subject-matter-specific harness outside this package, which means the
general product shipped a workflow whose last step was "and hope".

⚠ These cases assert the REFUSALS as hard as the runs. A verifier that quietly
skips a document it cannot drive, an input it cannot evaluate, or an output the
binding does not place reports a clean run for something it never executed,
and a green light nobody earned is worse than no light.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.verify import CALLABLE_KINDS, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
OPEN_BINDING = CROSSING / "controller.binding.yaml"
CLOSED_BINDING = CROSSING / "controller_resolved.binding.yaml"


def codegen_is_built() -> bool:
    from sce_author.verify import _default_codegen
    return _default_codegen().exists()


@unittest.skipUnless(codegen_is_built(),
                     "the product's code generator is not built in this tree")
class ADocumentIsRunNotJustRead(unittest.TestCase):
    def setUp(self):
        self.pack = load_pack(CROSSING)

    # ------------------------------------------------------------- running

    def test_the_closed_document_runs_against_the_records(self):
        result = verify(self.pack, CLOSED_BINDING)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertEqual(4, result.passed)

    def test_the_one_failure_is_the_planted_memory_gap(self):
        """⚠ The whole point, end to end.

        `questions` says `example-shows-memory` at `plant/out/bell.value`:
        two records drive every input identically and require the bell to do
        two different things. This proves the claim by execution -- no document
        of a kind that answers from its inputs alone can satisfy both, and the
        one case that fails is that one, at that address.

        A change that makes this pass has almost certainly stopped running the
        cases rather than fixed the document.
        """
        result = verify(self.pack, CLOSED_BINDING)
        self.assertEqual(1, result.failed)
        failing = [case for case in result.results if not case.passed]
        self.assertEqual(1, len(failing))
        self.assertEqual(
            [("plant/out/bell.value", "SILENT", "RINGING")],
            failing[0].failures)

    def test_every_expected_address_is_actually_written(self):
        """An address nothing writes is reported, never quietly passed."""
        result = verify(self.pack, CLOSED_BINDING)
        self.assertEqual([], result.unbound)
        for case in result.results:
            self.assertEqual([], case.unchecked, f"{case.name} left addresses unread")

    def test_a_position_no_case_expects_is_named_beside_the_pass(self):
        """⚠ THE OTHER HALF, and the one a pass needs.

        `unbound` says the cases reach for something the binding never writes.
        This says the binding writes something no case ever looks at -- where
        nothing failed because nothing looked. Two runs that judged two of
        nine positions and nine of nine print the same count otherwise, and
        the difference between them is the whole value of having run anything.

        Measured over 127 packs with examples: 3,272 of 3,593 output positions
        are expected by some case, and one pack expects none of its own.
        """
        # The closed pack expects all four, which is what makes the mutation
        # below mean something rather than being one more empty list.
        self.assertEqual([], verify(self.pack, CLOSED_BINDING).unasserted)

        def drop_the_bell(examples):
            for case in examples["cases"]:
                case["expect"].pop("plant/out/bell.value", None)

        pack, binding = self.staged(lambda b: None,
                                    mutate_examples=drop_the_bell)
        result = verify(pack, binding)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertEqual(["plant/out/bell.value"], result.unasserted)
        # ⚠ And it now PASSES everything, because the one failing case was the
        # planted memory gap at that very address. A report that stopped at
        # the count would say this document got better.
        self.assertEqual(0, result.failed)

    # ------------------------------------------------------------ refusing

    def test_an_open_decision_reaches_the_caller_with_its_authors_reason(self):
        """The placeholder AND the sentence the author wrote beside it reach
        the caller -- the feedback that says what to go and ask.

        ⚠ This used to assert a REFUSAL, because the refusal was how those
        words travelled. The run now builds a copy that withholds the open
        value and judges the rest, so the words travel in the report beside
        the counts instead. What is asserted is the part that mattered: the
        handle and the reason both arrive, and nothing is judged on the value
        the copy had to stand in with.
        """
        result = verify(self.pack, OPEN_BINDING)
        self.assertTrue(result.ran, result.refusal)
        said = "\n".join(result.unresolved_outputs.values())
        self.assertIn("CROSSING_TRAIN_SIGNAL_UNDECIDED", said)
        self.assertIn("never says what the train signal shows", said)
        # Arity floor: the open value is written somewhere the cases look.
        self.assertGreater(result.undetermined, 0)
        for case in result.results:
            if case.undetermined:
                self.assertFalse(case.passed, case.name)

    def test_a_kind_this_cannot_drive_is_refused_by_name(self):
        """Driving another kind means driving a different generated shape."""
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="verify_kind_"))
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        for name in ("interface-model.yaml", "conventions.yaml", "examples.yaml"):
            shutil.copy(CROSSING / name, tmp / name)
        document = (CROSSING / "controller_resolved.scxml").read_text(encoding="utf-8")
        (tmp / "doc.scxml").write_text(
            document.replace('sce:kind="transform"', 'sce:kind="observer"'),
            encoding="utf-8")
        binding = yaml.safe_load(CLOSED_BINDING.read_text(encoding="utf-8"))
        binding["document"] = "doc.scxml"
        (tmp / "b.yaml").write_text(yaml.safe_dump(binding), encoding="utf-8")

        result = verify(load_pack(tmp), tmp / "b.yaml")
        self.assertFalse(result.ran)
        self.assertIn("observer", result.refusal)
        for kind in CALLABLE_KINDS:
            self.assertIn(kind, result.refusal)

    def test_a_refuted_assumption_is_named_as_one(self):
        """⚠ The most useful sentence a failing run can print.

        `sce:assumed` is the product's non-blocking marker -- the pair of
        `sce:unresolved`, which refuses the build. An assumption COMPILES, so
        nothing downstream ever mentioned it again, and a case refuting one
        read as "your document is wrong" when the author had already written
        down that this exact value was a guess.

        ⚠⚠ And the assumption is followed TRANSITIVELY. It is rarely on the
        value that lands at an address; on the corpus that prompted this it
        was one step upstream every time, and attributing only to the direct
        writer found none of them.
        """
        document = (CROSSING / "controller_resolved.scxml").read_text(
            encoding="utf-8")
        guessed = document.replace(
            '<data id="roadSignal"',
            '<data id="guessed" sce:type="int32" sce:direction="out"\n'
            '          sce:assumed="CROSSING_ROAD_SIGNAL_ORDER"\n'
            '          sce:assumed-reason="the records never say which shows '
            'when both hold"\n'
            '          expr="approaching ? 1 : 0"/>\n'
            '    <data id="roadSignal"').replace(
            'expr="override ? 0 : (approaching ? 1 : (occupied ? 2 : 0))"',
            'expr="override ? 0 : (guessed !== 0 ? 9 : (occupied ? 2 : 0))"')

        def use_it(binding):
            binding["document"] = "guessed.scxml"
            binding["outputs"]["guessed"] = {"internal": True}

        pack, path = self.staged(use_it)
        (path.parent / "guessed.scxml").write_text(guessed, encoding="utf-8")

        result = verify(pack, path)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertTrue(result.failed, "the altered document has to fail")
        # ⚠ TWO hops: the bell is computed from the road signal, which is
        # computed from the assumed value. Neither the failing address nor the
        # one beside it carries the marker, and following only the direct
        # writer would have found nothing.
        self.assertIn("plant/out/bell.value", result.refuted)
        self.assertIn("never say which shows",
                      result.refuted["plant/out/bell.value"])

    def test_a_document_with_no_assumptions_refutes_none(self):
        """The discriminator: every failure would otherwise look assumed."""
        result = verify(self.pack, CLOSED_BINDING)
        self.assertTrue(result.failed)
        self.assertEqual({}, result.refuted)

    def test_a_record_may_say_not_reporting_with_a_token(self):
        """⚠ Absence is not always a missing key.

        A record commonly writes a TOKEN in the value's place -- and the
        conventions already declare which tokens those are, and the schema for
        them already said "a binding expresses them with `absent: true`". The
        core simply never read the list, so a source the record had explicitly
        marked as not reporting counted as present, and twelve telltales
        stayed dark across four components while the documents were blamed.
        """
        def watch_for_absence(binding):
            binding["inputs"]["barrierDown"] = {
                "address": "plant/in/barrier-position", "absent": True}

        def the_record_says_so(examples):
            for case in examples["cases"]:
                case["given"]["plant/in/barrier-position"] = "NO_REPORT"

        # Without the token declared, a value in place of a reading is a
        # reading, and absence is false.
        pack, path = self.staged(watch_for_absence, the_record_says_so)
        self.assertNotIn("NO_REPORT", pack.conventions.absence_tokens)
        before = verify(pack, path)
        self.assertTrue(before.ran)

        def declare_the_token(conventions):
            conventions["absence_tokens"] = ["NO_REPORT"]

        pack, path = self.staged(watch_for_absence, the_record_says_so,
                                 declare_the_token)
        self.assertIn("NO_REPORT", pack.conventions.absence_tokens)
        after = verify(pack, path)
        self.assertTrue(after.ran)
        self.assertNotEqual(
            [c.failures for c in before.results],
            [c.failures for c in after.results],
            "declaring the token has to change what the input reads as")

    def test_a_case_with_no_time_cannot_drive_a_clock(self):
        """⚠ Time is its own CATEGORY, not a signal with an address.

        It was the one thing a whole subject matter's bindings needed and the
        published vocabulary could not say, so it is its own key on both
        sides: `elapsed_ms` on the case, `clock` on the input. A record that
        did not say how long the situation had held cannot drive a document
        that watches the clock, and reading zero instead would make every
        duration answer as though no time had passed -- true of no round that
        ever happened.
        """
        def watch_the_clock(binding):
            binding["inputs"]["barrierDown"] = {"clock": True}

        pack, path = self.staged(watch_the_clock)
        result = verify(pack, path)
        self.assertTrue(result.ran)
        self.assertEqual(len(result.results), result.unjudged)
        self.assertIn("elapsed_ms", result.results[0].refusal)

        pack, path = self.staged(
            watch_the_clock,
            lambda ex: [case.update(elapsed_ms=4123) for case in ex["cases"]])
        result = verify(pack, path)
        self.assertEqual(0, result.unjudged,
                         "with the time recorded, every case can be driven")

    def test_a_duration_that_restarts_is_not_a_broken_record(self):
        """⚠ A monotonicity check was written here and the data refused it.

        `elapsed_ms` was first published as `at_ms`, a moment on a timeline,
        and a clock running backwards through an ordered record was refused as
        broken. But the field is how long the SITUATION had held -- on one
        corpus the cases a specification called 4 seconds and 70 seconds
        carried 4123 and 70123 -- so it restarts whenever the situation does.
        The check refused a whole component's records. There is none now, and
        this case is why.
        """
        def watch_the_clock(binding):
            binding["inputs"]["barrierDown"] = {"clock": True}

        def a_duration_that_restarts(examples):
            examples["ordered"] = True
            for case, held in zip(examples["cases"], (4123, 70123, 500, 9000, 12)):
                case["elapsed_ms"] = held

        pack, path = self.staged(watch_the_clock, a_duration_that_restarts)
        result = verify(pack, path)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertEqual(0, result.unjudged)

    def test_a_pack_whose_cases_withhold_values_cannot_run_anything(self):
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="verify_novalues_"))
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, tmp / name)
        for name in ("controller_resolved.scxml",):
            shutil.copy(CROSSING / name, tmp / name)
        shutil.copy(CLOSED_BINDING, tmp / "b.yaml")
        examples = yaml.safe_load((CROSSING / "examples.yaml").read_text(encoding="utf-8"))
        for case in examples["cases"]:
            case["given"] = {k: None for k in case["given"]}
            case["expect"] = {k: None for k in case["expect"]}
        (tmp / "examples.yaml").write_text(yaml.safe_dump(examples), encoding="utf-8")

        result = verify(load_pack(tmp), tmp / "b.yaml")
        self.assertFalse(result.ran)
        self.assertIn("values", result.refusal)

    def staged(self, mutate_binding, mutate_examples=None,
               mutate_conventions=None):
        """A copy of the crossing pack with parts of it changed."""
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="verify_stage_"))
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        for name in ("interface-model.yaml", "controller_resolved.scxml"):
            shutil.copy(CROSSING / name, tmp / name)
        conventions = yaml.safe_load(
            (CROSSING / "conventions.yaml").read_text(encoding="utf-8"))
        if mutate_conventions:
            mutate_conventions(conventions)
        (tmp / "conventions.yaml").write_text(yaml.safe_dump(conventions),
                                              encoding="utf-8")
        examples = yaml.safe_load(
            (CROSSING / "examples.yaml").read_text(encoding="utf-8"))
        if mutate_examples:
            mutate_examples(examples)
        (tmp / "examples.yaml").write_text(yaml.safe_dump(examples),
                                           encoding="utf-8")
        binding = yaml.safe_load(CLOSED_BINDING.read_text(encoding="utf-8"))
        mutate_binding(binding)
        (tmp / "b.yaml").write_text(yaml.safe_dump(binding), encoding="utf-8")
        return load_pack(tmp), tmp / "b.yaml"

    def test_an_input_needing_a_previous_round_needs_an_order(self):
        """⚠ Without an order there is no round before. Reading file order as
        a timeline would be reading a promise nobody made, and the verdict
        would be about that reading.
        """
        def use_previous(binding):
            binding["inputs"]["barrierDown"] = {
                "previous_of": "override",
                "caller_keeps": "this fixture is about the ORDER the cases "
                                "declare, not about where the memory lives",
            }

        pack, path = self.staged(use_previous)
        self.assertFalse(pack.examples.ordered)
        result = verify(pack, path)
        self.assertFalse(result.ran)
        self.assertIn("ordered", result.refusal)

    def test_the_same_binding_runs_once_the_cases_declare_an_order(self):
        """The other half: the refusal is about the EXAMPLES, not the rule."""
        def use_previous(binding):
            binding["inputs"]["barrierDown"] = {
                "previous_of": "override",
                "caller_keeps": "this fixture is about the ORDER the cases "
                                "declare, not about where the memory lives",
            }

        pack, path = self.staged(use_previous,
                                 lambda ex: ex.update(ordered=True))
        self.assertTrue(pack.examples.ordered)
        result = verify(pack, path)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")

    def test_feeding_back_an_output_never_produced_yet_is_refused(self):
        """⚠ On the first round there is nothing to feed back.

        `previous_of` may fall back to what the input reads now -- nothing had
        changed before observation began. `state_of` has no such fallback: an
        output read before it has ever been computed is a number nobody
        produced, so it takes a declared `initial` or it refuses.
        """
        def feed_back(binding):
            binding["inputs"]["barrierDown"] = {
                "state_of": "barrier",
                "caller_keeps": "the fixture is about the FIRST round having "
                                "no round before it",
            }

        pack, path = self.staged(feed_back, lambda ex: ex.update(ordered=True))
        result = verify(pack, path)
        # ⚠ EVERY round is unjudged here, and permanently so: the input is fed
        # from an output whose own computation needs that input. Nothing can
        # break the circle except a declared starting value, which is exactly
        # what `initial` is for. The refusal is per case rather than fatal, so
        # the report says how many rounds it covers.
        self.assertTrue(result.ran)
        self.assertEqual(len(result.results), result.unjudged)
        self.assertIn("initial", result.results[0].refusal)

        def feed_back_with_initial(binding):
            binding["inputs"]["barrierDown"] = {
                "state_of": "barrier",
                "initial": 0,
                "caller_keeps": "the fixture is about the FIRST round having "
                                "no round before it",
            }

        pack, path = self.staged(feed_back_with_initial,
                                 lambda ex: ex.update(ordered=True))
        result = verify(pack, path)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")


if __name__ == "__main__":
    unittest.main()
