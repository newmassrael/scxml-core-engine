"""A real number is equal to the platform's own comparison, which is not bit for bit.

A record writes `999.9` where the original component computes `9999 * 0.1`, which is the double
999.9000000000001 one place above it; the host's validator accepts that and still passes its own test.
`verify` compared the two exactly, so it failed cases the host passes, in three components. The
same validator rejects a value that went through a 32-bit real (a relative error near 1.5e-8), and
that must stay a failure: the generator once made such a value in a 64-bit slot and the product
caught it.

  one place apart      equal   (999.9 against 999.9000000000001, 0.4 against 0.39999999999999997)
  32-bit rounding      not equal   (-12.3 against -12.300000190734863)
  other numbers        not equal, and a spelling of the same number is still equal
"""

from __future__ import annotations

import unittest

from sce_author.verify import _same


class RealsAreComparedWithinTheLastPlace(unittest.TestCase):
    def test_a_double_one_place_away_is_the_same_number(self):
        self.assertTrue(_same("999.9", "999.9000000000001"))
        self.assertTrue(_same("0.4", "0.39999999999999997"))
        self.assertTrue(_same("-1553.6", "-1553.6000000000001"))

    def test_a_value_rounded_through_a_32_bit_real_is_not(self):
        """The control: the tolerance does not reach the error the generator once made."""
        self.assertFalse(_same("-12.3", "-12.300000190734863"))
        self.assertFalse(_same("-1553.6", "-1553.5999755859375"))

    def test_different_numbers_are_different(self):
        self.assertFalse(_same("1", "2"))
        self.assertFalse(_same("0.1", "0.2"))
        self.assertFalse(_same("0", "0.000001"))

    def test_one_number_in_two_spellings_is_still_the_same(self):
        self.assertTrue(_same("0.0", "0"))
        self.assertTrue(_same("2", "2.0"))


if __name__ == "__main__":
    unittest.main()
