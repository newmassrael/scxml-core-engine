"""A request made for a connection is not taken by an authoring client of one's own.

A person who pressed the button in the application chose a connection (which AI, which model),
and the request was made for it: only the executor of that connection takes it. The client in a
person's own terminal that this server speaks for runs for no connection, so the core refuses
it the request (`wrong-connection`), and this server says so in words the client can act on,
which are the owner's to act on: that request is the application's, and the owner calls it off
there if they want this client to write instead. It does not offer a connection it has not got,
and it does not go on to renew or write for a request the core did not give it.

The other side is a script, so that what the core would answer is the test's to say.
"""

from __future__ import annotations

import unittest

from sce_author import works

from tests.test_a_generation_is_kept_alive_by_the_process_that_holds_it import Core, request


def pinned_refusal() -> works.WorksError:
    return works.WorksError(
        "wrong-connection",
        "the request req-pinned is for connection `main` at 5ce2a9ebc304, and the caller runs for "
        "no connection: it is taken by the executor of the connection it was made for and by no "
        "other",
        {"request": "req-pinned", "pinned": {"id": "main", "revision": "5" * 64}, "offered": None},
    )


class ARequestMadeForAConnection(unittest.TestCase):
    def keeper(self) -> tuple[works.Generations, Core]:
        core = Core(
            read_work_heads={"request": request("req-pinned", "queued", 0)},
            claim_request=pinned_refusal(),
        )
        keeper = works.Generations(call=core, interval=3600.0, holder="mcp-test")
        self.addCleanup(keeper.close)
        return keeper, core

    def test_is_refused_with_what_the_client_can_say_to_the_owner(self):
        keeper, _ = self.keeper()

        with self.assertRaises(works.WorksError) as refused:
            keeper.begin("alpha")

        error = refused.exception
        self.assertEqual("wrong-connection", error.kind)
        # What the core said, kept: which connection the request is for.
        self.assertIn("connection `main`", str(error))
        # And what is to be done, which is the owner's to do, in the application.
        self.assertIn("call that request off in the application", str(error))
        self.assertEqual("req-pinned", error.detail["request"])

    def test_is_not_asked_for_by_offering_a_connection_this_server_has_not_got(self):
        keeper, core = self.keeper()

        with self.assertRaises(works.WorksError):
            keeper.begin("alpha")

        claims = core.asked("claim_request")
        self.assertEqual(1, len(claims))
        self.assertNotIn("connection", claims[0])

    def test_is_not_made_another_request_over_and_not_held(self):
        keeper, core = self.keeper()

        with self.assertRaises(works.WorksError):
            keeper.begin("alpha")

        # Nothing was registered in its place, and nothing is held that would be renewed.
        self.assertEqual([], core.asked("request_generation"))
        keeper.beat()
        self.assertEqual([], core.asked("heartbeat_request"))

    def test_another_refusal_of_a_claim_is_not_reworded(self):
        core = Core(
            read_work_heads={"request": request("req-new", "running", 1)},
            claim_request=works.WorksError("request-held", "held by somebody else", {}),
        )
        keeper = works.Generations(call=core, interval=3600.0, holder="mcp-test")
        self.addCleanup(keeper.close)

        with self.assertRaises(works.WorksError) as refused:
            keeper.begin("alpha")

        self.assertEqual("request-held", refused.exception.kind)
        self.assertEqual("held by somebody else", str(refused.exception))


if __name__ == "__main__":
    unittest.main()
