"""A generation an authoring client begins is held by this server process, and kept.

The workbench asks for a model by a request, and an executor takes it for a lease that
runs out. A client in a person's own terminal is such an executor, and the process that
speaks for it is this server: while the client thinks, drafts and checks, only the server
is there to say the request is still being worked on. These cases hold what that
promises against a stand-in for `sce-work`: which request is taken, that the lease is
renewed by the process and not by the client's remembering to, and that a request the
owner called off is told to the client at its next word and not renewed for ever.

The real `sce-work` and the real tools are driven in `test_the_works_folder_is_reached_
through_the_applications_own_command.py`; here the other side is a script, so that what
the core would answer at each step is the test's to say.
"""

from __future__ import annotations

import threading
import unittest

from sce_author import works

SOURCE = "a" * 64
ANSWERS = "b" * 64


class Core:
    """A stand-in for `sce-work`: it records what it was asked, and answers what the test
    said for the command, or refuses as the test said."""

    def __init__(self, **answers):
        self.calls: list[tuple[str, dict]] = []
        self.answers = {
            "read_work_snapshot": {"source": {"revision": SOURCE, "text": "The door opens."},
                                   "answers": None},
            "read_work_heads": {"request": None},
            "request_generation": {"request": request("req-new", "queued", 0), "created": True},
            "claim_request": {"request": request("req-new", "running", 1)},
            "heartbeat_request": {"request": request("req-new", "running", 1)},
            "save_request_candidate": {"request": request("req-new", "running", 1)},
            "complete_request": {"request": request("req-new", "completed", 1),
                                 "bundle": "c" * 64},
            "fail_request": {"request": request("req-new", "failed", 1)},
        } | answers

    def __call__(self, command: str, args: dict | None = None) -> dict:
        self.calls.append((command, args or {}))
        answer = self.answers[command]
        if isinstance(answer, Exception):
            raise answer
        return answer

    def asked(self, command: str) -> list[dict]:
        return [args for name, args in self.calls if name == command]


def request(identifier: str, state: str, attempt: int) -> dict:
    return {"id": identifier, "state": state, "attempt": attempt,
            "inputs": {"source": SOURCE, "answers": None}}


class AGenerationIsHeld(unittest.TestCase):
    def keeper(self, **answers) -> tuple[works.Generations, Core]:
        core = Core(**answers)
        # An interval so long that no background beat lands inside a case: the beat is
        # driven by hand (`beat`), except in the case that is about the thread.
        keeper = works.Generations(call=core, interval=3600.0, holder="mcp-test")
        self.addCleanup(keeper.close)
        return keeper, core

    def test_a_request_the_owner_made_is_the_one_that_is_taken(self):
        keeper, core = self.keeper(read_work_heads={"request": request("req-1", "queued", 0)},
                                   claim_request={"request": request("req-1", "running", 1)})

        held = keeper.begin("door")

        self.assertEqual("req-1", held.request)
        self.assertEqual(1, held.attempt)
        self.assertEqual([], core.asked("request_generation"), "no second request is made")
        self.assertEqual([{"id": "door", "request": "req-1", "holder": "mcp-test",
                           "resume": False}], core.asked("claim_request"))

    def test_a_work_nobody_asked_a_model_for_is_asked_for_one_by_the_client_that_begins(self):
        keeper, core = self.keeper(
            read_work_snapshot={"source": {"revision": SOURCE, "text": "x"},
                                "answers": {"revision": ANSWERS, "entries": {}}})

        keeper.begin("door")

        (made,) = core.asked("request_generation")
        self.assertEqual("mcp", made["origin"])
        self.assertEqual({"source": SOURCE, "answers": ANSWERS}, made["expect"])
        self.assertTrue(made["key"].startswith("mcp-"))
        self.assertEqual("req-new", core.asked("claim_request")[0]["request"])

    def test_an_interrupted_request_is_taken_again_as_the_next_attempt(self):
        keeper, core = self.keeper(
            read_work_heads={"request": request("req-1", "interrupted", 1)},
            claim_request={"request": request("req-1", "running", 2)})

        held = keeper.begin("door")

        self.assertEqual(2, held.attempt)
        self.assertTrue(core.asked("claim_request")[0]["resume"])

    def test_a_request_somebody_else_holds_is_told_as_the_core_told_it(self):
        keeper, _ = self.keeper(
            read_work_heads={"request": request("req-1", "running", 1)},
            claim_request=works.WorksError("request-held", "adapter-a holds it", {"holder": "adapter-a"}))

        with self.assertRaises(works.WorksError) as refused:
            keeper.begin("door")

        self.assertEqual("request-held", refused.exception.kind)
        self.assertEqual({"holder": "adapter-a"}, refused.exception.detail)

    def test_a_work_with_no_text_is_not_begun(self):
        keeper, core = self.keeper(read_work_snapshot={"source": None, "answers": None})

        with self.assertRaises(works.WorksError) as refused:
            keeper.begin("door")

        self.assertEqual("no-text", refused.exception.kind)
        self.assertEqual([], core.asked("claim_request"))

    def test_beginning_again_is_the_generation_already_held(self):
        keeper, core = self.keeper()
        first = keeper.begin("door")

        again = keeper.begin("door")

        self.assertIs(first, again)
        self.assertEqual(1, len(core.asked("claim_request")))

    def test_every_held_generation_is_renewed_by_name_and_attempt(self):
        keeper, core = self.keeper()
        keeper.begin("door")

        keeper.beat()

        self.assertEqual([{"id": "door", "request": "req-new", "holder": "mcp-test",
                           "attempt": 1}], core.asked("heartbeat_request"))

    def test_a_request_the_owner_called_off_is_not_renewed_and_is_told_at_the_next_word(self):
        keeper, core = self.keeper()
        keeper.begin("door")
        core.answers["heartbeat_request"] = works.WorksError(
            "request-ended", "the request was cancelled", {"state": "cancelled"})

        keeper.beat()
        keeper.beat()

        self.assertEqual(1, len(core.asked("heartbeat_request")),
                         "nothing is renewed for a request that ended")
        with self.assertRaises(works.WorksError) as told:
            keeper.get("req-new")
        self.assertEqual("generation-ended", told.exception.kind)
        self.assertEqual({"reason": "request-ended", "state": "cancelled"},
                         told.exception.detail)

    def test_a_core_that_did_not_answer_is_tried_again_and_the_generation_kept(self):
        keeper, core = self.keeper()
        keeper.begin("door")
        core.answers["heartbeat_request"] = works.WorksError("timeout", "no answer")

        keeper.beat()
        core.answers["heartbeat_request"] = {"request": request("req-new", "running", 1)}
        keeper.beat()

        self.assertEqual(2, len(core.asked("heartbeat_request")))
        self.assertEqual("req-new", keeper.get("req-new").request)

    def test_a_generation_this_process_did_not_begin_is_not_spoken_for(self):
        keeper, _ = self.keeper()

        with self.assertRaises(works.WorksError) as refused:
            keeper.get("req-other")

        self.assertEqual("unknown-generation", refused.exception.kind)
        self.assertIn("works_begin_generation", str(refused.exception))

    def test_what_is_written_for_a_request_is_written_as_its_holder(self):
        keeper, core = self.keeper()
        keeper.begin("door")

        keeper.save_candidate("req-new", model={"text": "<scxml/>"})

        (saved,) = core.asked("save_request_candidate")
        self.assertEqual({"id": "door", "request": "req-new", "holder": "mcp-test",
                          "attempt": 1, "text": "<scxml/>"}, saved)

    def test_a_refusal_that_says_the_request_ended_ends_the_generation_here_too(self):
        keeper, core = self.keeper()
        keeper.begin("door")
        core.answers["save_request_candidate"] = works.WorksError(
            "request-ended", "cancelled", {"state": "cancelled"})

        with self.assertRaises(works.WorksError) as refused:
            keeper.save_candidate("req-new", model={"text": "<scxml/>"})

        self.assertEqual("request-ended", refused.exception.kind)
        with self.assertRaises(works.WorksError) as told:
            keeper.get("req-new")
        self.assertEqual("generation-ended", told.exception.kind)

    def test_finishing_publishes_with_the_checks_the_client_ran_and_lets_go(self):
        keeper, core = self.keeper()
        keeper.begin("door")
        keeper.get("req-new").checks.append({"name": "decisions", "verdict": "accepted"})

        done = keeper.finish("req-new")

        self.assertEqual("c" * 64, done["bundle"])
        (asked,) = core.asked("complete_request")
        self.assertEqual({"id": "door", "request": "req-new", "holder": "mcp-test",
                          "attempt": 1,
                          "checks": [{"name": "decisions", "verdict": "accepted"}]}, asked)
        with self.assertRaises(works.WorksError):
            keeper.get("req-new")

    def test_a_refused_publication_keeps_the_generation_to_be_said_again(self):
        keeper, core = self.keeper()
        keeper.begin("door")
        core.answers["complete_request"] = works.WorksError(
            "check-refused", "the core refused the model", {"checks": ["model"]})

        with self.assertRaises(works.WorksError) as refused:
            keeper.finish("req-new")

        self.assertEqual("check-refused", refused.exception.kind)
        self.assertEqual("req-new", keeper.get("req-new").request)

    def test_failing_says_why_and_lets_go(self):
        keeper, core = self.keeper()
        keeper.begin("door")

        keeper.fail("req-new", "the draft could not satisfy the owner's answers")

        (asked,) = core.asked("fail_request")
        self.assertEqual("the draft could not satisfy the owner's answers", asked["reason"])
        with self.assertRaises(works.WorksError):
            keeper.get("req-new")

    def test_the_process_renews_the_lease_without_being_asked(self):
        core = Core()
        beaten = threading.Event()
        beats = []

        def counting(command, args=None):
            answer = core(command, args)
            if command == "heartbeat_request":
                beats.append(args)
                if len(beats) >= 2:
                    beaten.set()
            return answer

        keeper = works.Generations(call=counting, interval=0.01, holder="mcp-test")
        self.addCleanup(keeper.close)
        keeper.begin("door")

        self.assertTrue(beaten.wait(timeout=10), "the lease was not renewed by the process")

    def test_closing_stops_the_renewing(self):
        core = Core()
        keeper = works.Generations(call=core, interval=0.01, holder="mcp-test")
        keeper.begin("door")

        keeper.close()
        seen = len(core.asked("heartbeat_request"))
        keeper.beat()

        self.assertEqual(seen, len(core.asked("heartbeat_request")),
                         "a closed keeper renews nothing")


if __name__ == "__main__":
    unittest.main()
