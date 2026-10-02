"""A standing rule the OWNER said, made into the profile's `house_rules`.

A house rule is the owner's standing answer to a gap that recurs across
specifications: a draft that applies one does not ask. That authority is the
owner's, and nothing in a draft's way of working can be allowed to mint it. Two
failures sit either side of this module. A client that invents a rule and writes
it into the profile gives itself permission it was never given; and an owner who
has to write the profile's JSON never gets to say a rule at all. So the owner
says the rule in their own words, a client brings those words here, and this
module does the half that needs no trust:

* the rule is saved only with the owner's words beside it, and a quote that is not
  in the owner's words (when they are handed over) is refused, so a rule cannot
  be attributed to an owner who did not say it;
* nothing is saved on the first call. It returns what to show the owner, the
  rule and their words together, and a profile only when the client says the owner
  said yes to THAT wording;
* the yes is recorded for what it is. The product was not in the conversation, so
  it cannot know the owner confirmed anything; a profile made here says
  `relayed`, which is a client's report, and every answer that applies the rule
  repeats it. A rule written into a profile by hand carries no such word and is
  never read as confirmed.

The ids come from here (`H1`, `H2`, ...), the lowest not yet used, unless the
client names one; an id is one token and names one rule. This module never
writes a file and opens none: the profile is handed in as text and handed back as
text, the way `scxml_requirement_set` hands back its manifest.
"""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import re
from dataclasses import dataclass, field

from .errors import AuthoringError
from .structured import RepeatedKey, read_json

RECORD = "sce-authoring-profile"
VERSION = 1

#: What the product accepts as a rule's id: one token.
_ID = re.compile(r"\S+")

#: The word a profile carries for "a client reported that the owner said yes".
RELAYED = "relayed"

#: A request that names more than this many rules is not one owner's one sitting.
MAX_RULES = 20


class HouseRuleError(AuthoringError):
    """The request cannot be made into rules the owner can be shown."""


@dataclass
class Proposed:
    id: str
    rule: str
    quote: str


@dataclass
class Built:
    rules: list[Proposed]
    existing: int
    profile_text: str | None = None
    notes: list[str] = field(default_factory=list)


def _existing(profile_text: str | None) -> dict:
    """The profile the rules join, or the header of a new one."""
    if profile_text is None:
        return {"record": RECORD, "v": VERSION}
    try:
        profile = read_json(profile_text)
    except RepeatedKey as error:
        raise HouseRuleError(f"the profile cannot be extended: {error}") from error
    except ValueError as error:
        raise HouseRuleError(f"the profile is not JSON: {error}") from error
    if not isinstance(profile, dict) or profile.get("record") != RECORD:
        raise HouseRuleError(f"the profile is not a `{RECORD}` record, so a rule cannot be "
                             f"added to it")
    if profile.get("v") != VERSION:
        raise HouseRuleError(f"the profile is version {profile.get('v')!r} and this tool "
                             f"writes version {VERSION}")
    rules = profile.get("house_rules")
    if rules is not None and (not isinstance(rules, list)
                              or not all(isinstance(r, dict) and isinstance(r.get("id"), str)
                                         and isinstance(r.get("rule"), str) for r in rules)):
        raise HouseRuleError("the profile's `house_rules` is not a list of rules with an id "
                             "and a rule, so a rule cannot be added to it")
    return profile


def _words(text: str) -> str:
    """A rule's text with its spacing set aside, to tell one rule from a repeat."""
    return " ".join(text.split()).casefold()


def build(profile_text: str | None, requested, *, owner_words: str | None = None,
          confirmed: bool = False) -> Built:
    """The rules the owner is to be shown, and with `confirmed` the profile that holds them.

    `requested` is a list of objects with `quote` (the owner's own words, copied
    exactly), `rule` (the standing answer in the client's wording) and optionally
    `id`. `owner_words`, when given, is the text the quotes are held to.
    """
    if not isinstance(requested, list) or not requested:
        raise HouseRuleError("'rules' has to be a non-empty list: one object per rule, with "
                             "`quote` (the owner's words) and `rule`")
    if len(requested) > MAX_RULES:
        raise HouseRuleError(f"'rules' names {len(requested)} rules; put at most {MAX_RULES} "
                             f"to the owner at a time")
    profile = _existing(profile_text)
    held = list(profile.get("house_rules") or [])
    used = {rule["id"] for rule in held}
    said = {_words(rule["rule"]): rule["id"] for rule in held}

    proposed: list[Proposed] = []
    for index, raw in enumerate(requested, 1):
        where = f"rules[{index}]"
        if not isinstance(raw, dict):
            raise HouseRuleError(f"{where} has to be an object with `quote` and `rule`")
        unknown = sorted(set(raw) - {"quote", "rule", "id"})
        if unknown:
            raise HouseRuleError(f"{where} has {', '.join(map(repr, unknown))}, which a rule "
                                 f"does not take; it takes `quote`, `rule` and `id`")
        quote, rule = raw.get("quote"), raw.get("rule")
        if not isinstance(quote, str) or not quote.strip():
            raise HouseRuleError(f"{where} has no `quote`: a rule is saved with the owner's "
                                 f"words beside it, and without them nothing says the owner "
                                 f"said it")
        if not isinstance(rule, str) or not rule.strip():
            raise HouseRuleError(f"{where} has no `rule`")
        if owner_words is not None and quote not in owner_words:
            raise HouseRuleError(
                f"{where}: the quote {quote!r} is not in the owner's words, word for word. "
                f"A rule is attributed to the owner only by words they said; copy the quote "
                f"exactly, or put the question to the owner again")
        repeat = said.get(_words(rule))
        if repeat is not None:
            raise HouseRuleError(f"{where}: the rule says what {repeat} already says; a rule "
                                 f"held twice is two things to keep in step")
        identifier = raw.get("id")
        if identifier is None:
            number = 1
            while f"H{number}" in used:
                number += 1
            identifier = f"H{number}"
        elif not isinstance(identifier, str) or not _ID.fullmatch(identifier):
            raise HouseRuleError(f"{where}: an id is one token, since a draft writes it in "
                                 f"`sce:assumed`, where a marker id holds no space")
        elif identifier in used:
            raise HouseRuleError(f"{where}: the id {identifier!r} names a rule the profile "
                                 f"already holds")
        used.add(identifier)
        said[_words(rule)] = identifier
        proposed.append(Proposed(identifier, rule, quote))

    built = Built(rules=proposed, existing=len(held))
    if owner_words is None:
        built.notes.append("the quotes were not held to the owner's words, because none were "
                           "handed over: the profile will say `relayed` of each, which is "
                           "a client's report and not something this tool saw")
    if confirmed:
        made = dict(profile)
        made["house_rules"] = held + [
            {"id": p.id, "rule": p.rule, "quote": p.quote, "confirmation": RELAYED}
            for p in proposed]
        built.profile_text = json.dumps(made, indent=2, ensure_ascii=False) + "\n"
    return built


def standing(applied_rules) -> list[str]:
    """On what authority each rule a design applied stands, one sentence each, for
    the owner who is asked to accept the design.

    ⚠ Said of every rule, not only the doubtful ones: a list that mentions the
    weak cases alone reads, to someone skimming it, as the whole list. A rule
    with the owner's words and a relayed yes says both; a rule without words says
    that nothing records where it came from, which is not the same as the owner
    having said it."""
    lines = []
    for rule in applied_rules or ():
        said, vouched = rule.get("quote"), rule.get("confirmation")
        if said and vouched == RELAYED:
            lines.append(f"{rule['id']}: made from the owner's words \"{said}\"; a client "
                         f"reported that the owner said yes, which the product did not see")
        elif said:
            lines.append(f"{rule['id']}: carries the owner's words \"{said}\" and no record "
                         f"that the owner confirmed the wording")
        else:
            lines.append(f"{rule['id']}: written into the profile; nothing records where it "
                         f"came from or that the owner said it")
    return lines


def save(out: pathlib.Path, text: str, base_sha256: str | None) -> dict:
    """Write the profile to `out`, whole or not at all, and say where it is.

    `base_sha256` is the digest of the bytes the rules were added to (the profile
    the caller gave), or None when none was given. A file already at `out` is
    overwritten only when it still holds exactly those bytes: anything else is
    somebody's edit since the profile was read, and writing over it would lose it
    without a word. A path that holds no file yet is written new.

    The write goes to a temporary file in the same directory and is renamed over
    `out`, so a failure leaves the old file or no file, never half of one, and a
    second identical request is refused upstream (a rule the profile already holds
    is not added twice) before it can write a second revision.

    The profile's revision is its digest: an acceptance record pins the profile by
    it, so a rule changed here is found by every record that applied it
    (`acceptance-impact`) and no counter is kept beside it that could disagree."""
    previous = None
    if out.exists():
        if not out.is_file():
            raise HouseRuleError(f"{out}: is not a file, so a profile cannot be written there")
        previous = hashlib.sha256(out.read_bytes()).hexdigest()
        if base_sha256 is None:
            raise HouseRuleError(
                f"{out}: already exists. Give it as `profile` so the rules join what it "
                f"holds, or name another `out`; nothing was written")
        if previous != base_sha256:
            raise HouseRuleError(
                f"{out}: holds sha256 {previous[:12]}, and the profile you gave is "
                f"{base_sha256[:12]}. It changed since you read it, and writing over it "
                f"would lose that change; read it again and ask again. Nothing was written")
    if not out.parent.is_dir():
        raise HouseRuleError(f"{out.parent}: is not a directory, so the profile cannot be "
                             f"written there")
    data = text.encode("utf-8")
    temporary = out.with_name(f".{out.name}.{os.getpid()}.tmp")
    try:
        with open(temporary, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, out)
    except OSError as error:
        raise HouseRuleError(f"{out}: could not be written ({error}); the profile that was "
                             f"there, if any, is as it was") from error
    finally:
        if temporary.exists():
            temporary.unlink()
    return {"path": str(out), "sha256": hashlib.sha256(data).hexdigest(),
            "previous_sha256": previous}


def answer(built: Built, saved: dict | None = None) -> dict:
    """What the client says to the owner, and what it does next."""
    shown = [{"id": p.id, "rule": p.rule, "your_words": p.quote} for p in built.rules]
    reply: dict = {
        "status": "ready-to-save" if built.profile_text is not None else "proposal",
        "rules": shown,
        "tell_the_owner": (
            "These would become standing rules: every specification drafted under this "
            "profile will have them applied without asking, until you remove them. Each is "
            "shown with the words of yours it was made from. Say yes to keep them as "
            "worded, or tell me what to change. "
            + " ".join(f"{p.id}: {p.rule} (your words: \"{p.quote}\")" for p in built.rules)),
    }
    if built.notes:
        reply["notes"] = built.notes
    if built.profile_text is None:
        reply["next"] = ("nothing was saved. Show `tell_the_owner` to the owner and wait. Only "
                         "if they say yes to these rules as worded, call this tool again with "
                         "the same `rules` and `owner_confirmed: true`; never set it on your "
                         "own say-so")
    elif saved is not None:
        reply["status"] = "saved"
        reply["saved"] = {**saved, "rules": [p.id for p in built.rules]}
        reply["next"] = (f"the profile is at `{saved['path']}` (sha256 {saved['sha256'][:12]}): "
                         "pass that path as `profile` to the tools that take one. The rules "
                         "say `relayed`: you reported that the owner said yes, and every "
                         "answer that applies a rule repeats that. Tell the owner where it is")
    else:
        reply["profile_text"] = built.profile_text
        reply["next"] = ("save `profile_text` as the owner's profile (this tool wrote no "
                         "file; on a local server `out` writes it) and pass it as `profile` "
                         "to the tools that take one. The rules say `relayed`: you reported "
                         "that the owner said yes, and every answer that applies a rule "
                         "repeats that")
    return reply
