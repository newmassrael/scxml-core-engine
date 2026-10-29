# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A call through a trait object is followed into every implementation it may reach.

The runtime's Appendix D procedures are compiled once for every program and reach each
machine through `&mut dyn` hooks (docs/adr/0064), so on a two-machine image the deepest chain
runs through an indirect call. The stack instrument resolves such a call the way
class-hierarchy analysis resolves a virtual call: to every implementation the caller may
reach, the worst of them taken. These cases hold it to that, over disassembly written out by
hand in the form `objdump -d -r -C` prints.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from worst_case_stack import Analysis, Refusal, parse  # noqa: E402

# `core::walk` takes 32 bytes and calls through a register; the two hooks of one machine take
# 72 and 8. `core::loop` calls a hook of the other trait, which calls back into the walk.
DISASSEMBLY = """
00000000 <core::walk>:
       0:	push	{r4, r5, r7, lr}
       2:	sub	sp, #16
       4:	blx	r3

00000000 <<Host as Behaviour>::enter_state>:
       0:	push	{r4, lr}
       2:	sub	sp, #64

00000000 <<Host as Behaviour>::exit_state>:
       0:	push	{r7, lr}

00000000 <core::loop>:
       0:	push	{r7, lr}
       2:	blx	r2

00000000 <<Host as MainLoop>::end_run>:
       0:	push	{r7, lr}
       2:	bl	0 <core::walk>
			2: R_ARM_THM_CALL	core::walk
""".splitlines()

BEHAVIOUR = r"as Behaviour>::"
MAIN_LOOP = r"as MainLoop>::"


def analysis(dispatch, indirect=None):
    return Analysis(parse(DISASSEMBLY), {}, [], {}, indirect, dispatch)


class TraitObjectCalls(unittest.TestCase):
    def test_the_worst_implementation_is_charged(self):
        found, chain = analysis([("core::walk", BEHAVIOUR)]).worst("core::walk", [], {})
        self.assertEqual(found, 32 + 72)
        self.assertIn("<Host as Behaviour>::enter_state [72]", chain)

    def test_an_indirect_call_nothing_states_is_refused(self):
        with self.assertRaises(Refusal):
            analysis([]).worst("core::walk", [], {})

    def test_a_host_callback_allowance_still_stands_beside_the_implementations(self):
        found, _ = analysis([("core::walk", BEHAVIOUR)], indirect=200).worst("core::walk", [], {})
        self.assertEqual(found, 32 + 200)

    def test_the_dispatch_is_scoped_by_the_caller(self):
        # `core::loop` reaches only `MainLoop` implementations, and `core::walk` only
        # `Behaviour` ones, so the chain through `end_run` back into the walk is followed
        # once and is not mistaken for a cycle.
        found, chain = analysis(
            [("core::loop", MAIN_LOOP), ("core::walk", BEHAVIOUR)]
        ).worst("core::loop", [], {})
        self.assertEqual(found, 8 + 8 + 32 + 72)
        self.assertEqual(chain[0], "core::loop [8]")


if __name__ == "__main__":
    unittest.main()
