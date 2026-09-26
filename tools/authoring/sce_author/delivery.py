"""What an input rule hands the document, and whether the document can take it.

Two declarations already say what an input is. The DOCUMENT says what type
each input has (`sce:type`), and the INTERFACE MODEL says what each address
carries -- an enumeration, a number, a text, a truth value. A binding does not
say either a third time. It states only what neither declaration can: a
comparison that turns what an address carries into a truth value, a bound on a
number, what absence reads as. A rule that names an address and nothing else
hands the document that address's own value, and the two declarations decide
how it is read -- or refuse together when they disagree.

⚠ Why this is the rule rather than a key per type. `number: true` was such a
key, and it restated the document: measured 2026-09-22 over 24 bindings, all
24 uses fed an `int32` input, and nothing compared the two. The one input no
key could express -- a text read as it is -- had been written as a rule that
could never produce it, and the case it failed was reported as a refuted
guess about something else. A third declaration is a third place to disagree.

⚠ The document's type names are the PRODUCT's, read from the product's own
grammar rather than copied here. A type the product starts accepting that this
module has no reading for is refused by name the first time it is met, not
treated as the nearest thing it resembles.
"""

from __future__ import annotations

import functools
import pathlib
import re
import xml.etree.ElementTree as ET

_XS = "{http://www.w3.org/2001/XMLSchema}"
# Where the product declares its scalar types. `SceType::from_attr` is the
# source of truth and this grammar mirrors it; a gate keeps the two together.
_GRAMMAR = pathlib.Path(__file__).resolve().parents[3] / "schemas" / "sce-forge-ext.xsd"

# What a value of each product type IS, for the purpose of handing one over.
_CLASSES = (
    (re.compile(r"u?int(8|16|32|64)|float(32|64)"), "number"),
    (re.compile(r"bool"), "bool"),
    (re.compile(r"string"), "text"),
    (re.compile(r"bytes"), "bytes"),
)

# Keys whose rule hands the document a truth value.
_TRUTH = ("equals", "equals_any", "not_equals", "absent", "variant_is", "protocol")

# The pairs that hand a value over unchanged: (document class, slot kind).
# ⚠ An enumeration input takes the platform's NUMBER, not its spelling. An
# enumeration document declares a number for every variant, and the model
# declares one for every symbol, so the two meet on the number -- and where
# they both name a value they must agree on it (`refusal`).
_AS_IS = {("number", "number"), ("number", "enum"), ("text", "text"),
          ("bool", "boolean"), ("enum", "enum"), ("enum", "number")}


class DeliveryError(Exception):
    """This rule cannot hand the document the value this case would need."""


@functools.lru_cache(maxsize=1)
def product_types() -> dict[str, str]:
    """Every scalar `sce:type` the product accepts, to what kind of value it is."""
    try:
        root = ET.parse(_GRAMMAR).getroot()
    except (OSError, ET.ParseError) as exc:
        raise DeliveryError(
            f"the product's grammar could not be read at {_GRAMMAR} ({exc}), "
            f"so no document input's type can be known") from exc
    declared = [e.get("value")
                for simple in root.iter(f"{_XS}simpleType")
                if simple.get("name") == "sceType"
                for e in simple.iter(f"{_XS}enumeration")]
    if not declared:
        raise DeliveryError(f"{_GRAMMAR} declares no `sceType`, so no document "
                            f"input's type can be known")
    out = {}
    for name in declared:
        kind = next((k for pattern, k in _CLASSES if pattern.fullmatch(name)), None)
        if kind is None:
            raise DeliveryError(
                f"the product accepts `sce:type=\"{name}\"`, and this module has "
                f"no reading for it. Say here what a value of that type is "
                f"before a binding hands one over")
        out[name] = kind
    return out


def document_class(sce_type: str | None) -> str | None:
    """What kind of value a document input of this declared type takes."""
    if not sce_type:
        return None
    if sce_type.startswith("enum:"):
        return "enum"
    return product_types().get(sce_type)


def slot_kind(field) -> str | None:
    """What the interface model says an address carries, or None if unsaid."""
    if field is None:
        return None
    if field.values:
        return "enum"
    # A whole number is a number for what a rule can hand a document; the
    # distinction is for what a document may declare (`scaffold`).
    return "number" if field.type == "integer" else field.type


def hands(rule: dict) -> str:
    """What kind of value this rule gives the document, whatever it reads."""
    if rule.get("event"):
        return "event"
    if rule.get("previous_of") or rule.get("state_of"):
        return "memory"
    if any(key in rule for key in _TRUTH):
        return "bool"
    if rule.get("clock"):
        return "number"
    return "as-is"


def refusal(name: str, rule: dict, sce_type: str | None, field,
            remembered_type: str | None = None, variants: dict | None = None) -> str:
    """Why this rule cannot feed a document input of this type, or ''.

    `remembered_type` is the declared type of what a `previous_of` or
    `state_of` rule reads back, and `variants` the enumeration an `enum:`
    input names, variant to number. `field` is the model's for the rule's
    address; None means the model does not declare it, which `check` reports
    on its own, so it is not a second reason here.
    """
    doc = document_class(sce_type)
    given = hands(rule)
    if given == "event" or doc is None:
        return ""
    if given == "memory":
        if remembered_type and remembered_type != sce_type:
            which = "previous_of" if rule.get("previous_of") else "state_of"
            return (f"reads back {rule[which]!r}, declared `{remembered_type}`, "
                    f"into an input the document declares `{sce_type}`")
        return ""
    if "range" in rule and not (given == "as-is" and doc == "number"):
        return (f"`range` bounds a number, and this rule hands the document "
                f"{'a truth value' if given == 'bool' else 'no number'} for an "
                f"input declared `{sce_type}`")
    if given == "bool":
        if doc != "bool":
            key = next(k for k in _TRUTH if k in rule)
            return (f"`{key}` hands the document a truth value, and it declares "
                    f"{name!r} `{sce_type}`")
        return ""
    if given == "number":
        return "" if doc == "number" else (
            f"`clock` hands the document a number, and it declares {name!r} "
            f"`{sce_type}`")
    if field is None:
        return ""
    slot = slot_kind(field)
    if slot is None:
        # ⚠ Declared, and silent about what it carries -- a platform that
        # gives an address no type. Reading it as a number was the pack's
        # guess, and a guess here is a type nobody declared.
        return (f"the interface model declares {rule.get('address')} without "
                f"saying what it carries, so nothing says how to hand it to "
                f"an input declared `{sce_type}`")
    if doc == "enum" and slot in ("enum", "number") and variants is None:
        return (f"the document declares {name!r} `{sce_type}`, and no "
                f"enumeration of that name is imported, so nothing says what "
                f"its numbers mean")
    if (doc, slot) == ("enum", "enum"):
        # ⚠ One value named by both sides with two numbers is two meanings
        # for one handoff. Names only one side has are not compared: a
        # platform list carries values -- a timeout, a reserved code -- that a
        # document's enumeration need not, and meeting one is refused when it
        # is read rather than presumed here.
        clash = sorted(s for s, n in field.values.items()
                       if s in variants and variants[s] != n)
        if clash:
            return (f"the address and the document's enumeration give "
                    + ", ".join(f"{s} as {field.values[s]} and {variants[s]}"
                                for s in clash)
                    + " -- the handoff is by number, so one of them is wrong")
        return ""
    if (doc, slot) in _AS_IS:
        return ""
    if doc == "bool":
        return (f"names an address and nothing else, which hands the document "
                f"what the address carries -- {_carries(slot)} -- and it "
                f"declares {name!r} `bool`. Say which value is true with "
                f"`equals`")
    return (f"names an address and nothing else, which hands the document what "
            f"the address carries -- {_carries(slot)} -- and it declares "
            f"{name!r} `{sce_type}`")


def _carries(slot: str) -> str:
    return {"enum": "a symbol of an enumeration", "number": "a number",
            "text": "a text", "boolean": "a truth value"}.get(slot, slot)


def read_as_is(name: str, rule: dict, value, present: bool, sce_type: str | None,
               field, absence_tokens=(), variants: dict | None = None):
    """The address's own value, as the document input's type reads it.

    ⚠ Absence is DECLARED or it is refused, for every kind alike. A missing
    key and a token the conventions list as "not reporting" are one fact, and
    folding either into zero, an empty text or False made seven cases on one
    corpus look as though the specification had been misread.
    """
    doc = document_class(sce_type)
    slot = slot_kind(field)
    if doc is None:
        raise DeliveryError(
            f"input {name!r}: the document declares no type the product knows "
            f"(`{sce_type}`), so nothing says how the address is read")
    if field is None:
        raise DeliveryError(
            f"input {name!r}: the interface model does not declare "
            f"{rule.get('address')}, so nothing says how to read it")
    why = refusal(name, rule, sce_type, field, variants=variants)
    if why:
        raise DeliveryError(f"input {name!r}: {why}")
    tokens = {str(t).strip().lower() for t in absence_tokens}
    if not present or value is None or str(value).strip().lower() in tokens:
        if "when_absent" in rule:
            return rule["when_absent"]
        raise DeliveryError(
            f"input {name!r}: the case drives no value at {rule.get('address')}, "
            f"and absence has no safe silent reading. Say what it reads as "
            f"with `when_absent`")
    if doc == "text":
        return str(value)
    if doc == "bool":
        text = str(value).strip().lower()
        if text in ("true", "1"):
            return True
        if text in ("false", "0"):
            return False
        raise DeliveryError(f"input {name!r}: {value!r} at {rule.get('address')} "
                            f"is not a truth value")
    number = _number(name, rule, value, field)
    if doc == "enum" and number not in variants.values():
        # A value the platform can carry and the document's enumeration has
        # no variant for. Handing it over would give the document a number
        # its own type says cannot occur.
        raise DeliveryError(
            f"input {name!r}: {value!r} at {rule.get('address')} is "
            f"{number}, and the document's enumeration `{sce_type}` has no "
            f"variant numbered so")
    return number


def _number(name: str, rule: dict, value, field):
    text = str(value).strip()
    # ⚠ Through the value space first. A record writes `ON` where the
    # document wants the platform's number for it, and the model is the
    # authority on which number a symbol is.
    if field.values and text in field.values:
        number = float(field.values[text])
    else:
        try:
            number = float(text)
        except ValueError:
            if "when_absent" in rule:
                return rule["when_absent"]
            raise DeliveryError(f"input {name!r}: {value!r} at "
                                f"{rule.get('address')} is not a number") from None
    low, high = rule.get("range", (None, None))
    if low is not None and not (low <= number <= high):
        # A reading the platform cannot represent is not a reading.
        if "when_absent" in rule:
            return rule["when_absent"]
        raise DeliveryError(
            f"input {name!r}: {number} at {rule.get('address')} is outside the "
            f"declared range {low}..{high}, which is a different fact from any "
            f"value in it")
    return int(number) if number.is_integer() else number
