# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A root's own frame is held apart from the worst case below it.

A root is an entry point a host calls (`initialize`, `step`). A buffer meant for one turn of
the engine, built in a function that inlines into the root, moves into the root's frame and,
when the root inlines into the host, into the host's: the worst case is the same either way,
since the bytes stay on one chain, so only the root's own frame can see the move
(docs/adr/0068). These cases hold the cap to that, over disassembly written out by hand in the
form `objdump -d -r -C` prints.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from worst_case_stack import parse, root_frames_over  # noqa: E402

# The same 256-byte workspace twice: once in a turn of its own that the root calls, once
# inlined into the root.
DISASSEMBLY = """
00000000 <Engine::step>:
       0:	push	{r7, lr}
       2:	bl	0 <Engine::run_turn>
			2: R_ARM_THM_CALL	Engine::run_turn

00000000 <Engine::run_turn>:
       0:	push	{r4, r5, r7, lr}
       2:	sub	sp, #256

00000000 <Engine::initialize>:
       0:	push	{r4, r5, r7, lr}
       2:	sub	sp, #256
""".splitlines()


class RootFrames(unittest.TestCase):
    def test_a_workspace_inlined_into_a_root_is_over_the_cap(self):
        functions = parse(DISASSEMBLY)
        over = root_frames_over(functions, ["Engine::initialize", "Engine::step"], 64)
        self.assertEqual(over, [("Engine::initialize", 16 + 256)])

    def test_a_workspace_in_a_turn_of_its_own_is_not_the_roots(self):
        functions = parse(DISASSEMBLY)
        self.assertEqual(root_frames_over(functions, ["Engine::step"], 64), [])

    def test_no_cap_holds_nothing(self):
        functions = parse(DISASSEMBLY)
        self.assertEqual(root_frames_over(functions, ["Engine::initialize"], None), [])


if __name__ == "__main__":
    unittest.main()
