"""A domain-free core for turning a prose specification into a document.

What it holds: the relation between a text, an interface model and a binding.
What it must never hold: anything about what the text is about. A pack supplies
that, as data, and `tests/test_core_is_domain_free.py` measures the claim on
every run instead of trusting it.
"""

__all__ = ["pack", "prose", "questions", "brief", "check"]
