"""Show the written document the way a person reads it, for approval.

This is the last hop of the workflow and the only one whose judge is a human.
Everything before it is a machine answering a question a machine can answer:
`check` says the names are real, `verify` says the document behaves on the
pack's examples, `coverage` says the set reaches every position. A document can
satisfy all three and still not be the thing the specification owner asked for,
and nothing here could say so, because "is this what I meant" is not a question
any of them are asking.

So the person has to read it — and handing them XML means they do not. The
product already renders a document as a total pseudocode surface: every field
of the model reaches the page, a document it cannot show in full is refused by
name rather than abbreviated, and a reverse converter proves the page carries
the document rather than a summary of it. That surface was reachable only from
a shell. This module is the seam, so a workflow driven over MCP does not have
to leave it for its last and most important step.

⚠ It consults the PACK for nothing, and does not take one. Rendering needs the
document and the document alone; requiring a pack would be a coupling this
answer does not have, and a caller handed a refusal about their pack when they
asked to read their document would be told about the wrong file.

⚠ The binding names the document rather than the caller naming it twice. The
same file `check` and `verify` were given is the file this shows, which is the
property that makes the approval mean anything: a page rendered from some other
document on disk would read just as well.
"""

from __future__ import annotations

import dataclasses
import pathlib

from .check import read_binding
from .errors import AuthoringError, describe_path
# ⚠ This module does NOT spawn. Exactly one module in this core may run
# another program, and `test_only_one_module_may_run_another_program` holds
# that rule -- so the run lives in `verify` and this module composes it. The
# rule is about how many places can reach outside the pack, and adding this
# file to the allowed list would have answered the test while breaking the
# thing the test is for.
from .verify import pseudo_page


class PseudoError(AuthoringError):
    """A rendering could not be produced. Names the path and what was wrong."""


@dataclasses.dataclass
class Rendering:
    """What the product said, either way.

    ⚠ `text` is kept byte for byte, not stripped. The surface's own contract
    is that a value reaches the page as the author spelled it, and a caller
    that trims the page is the first thing to break that.
    """

    document: pathlib.Path
    text: str = ""
    refusal: str = ""
    deployed: bool = False

    @property
    def produced(self) -> bool:
        return not self.refusal


def render(binding_path, codegen=None, deploy=None) -> Rendering:
    """Render the document this binding names, or carry back the refusal.

    ⚠ The product's own words are passed through untouched, the way `verify`
    passes a generator refusal. A document carrying a construct the surface
    will not abbreviate is refused BY THE PRODUCT, and that refusal names the
    construct — which is exactly what the author needs to hear. Re-phrasing it
    would lose the name.
    """
    binding_path = pathlib.Path(binding_path)
    binding = read_binding(binding_path)

    # ⚠ Presence and type belong to the schema, which `read_binding` has
    # already applied — a binding without `document` is refused there, by the
    # rule rather than by a second copy of it here. What the schema does not
    # say is that the path has to be non-empty, and an empty one resolves to
    # the binding's own directory and would be reported as "is a directory",
    # which is a true sentence about the wrong mistake.
    named = binding.get("document")
    if not named:
        raise PseudoError(
            f"{binding_path}: names its document as an empty path, so there "
            f"is nothing to show")
    document = (binding_path.parent / named).resolve()
    if not document.is_file():
        raise PseudoError(describe_path(document))

    if deploy is not None:
        deploy = pathlib.Path(deploy)
        if not deploy.is_file():
            raise PseudoError(describe_path(deploy))

    page, refusal = pseudo_page(document,
                                pathlib.Path(codegen) if codegen else None,
                                deploy)
    return Rendering(document=document,
                     deployed=deploy is not None,
                     text=page,
                     refusal=refusal)
