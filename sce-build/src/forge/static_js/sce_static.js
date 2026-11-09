(function () {
  /* SceStatic: the checked arithmetic and the typed event data of a
     datamodel="sce-static" document, for the script engine the Interpreter
     runs it on (docs/SCE_ACCEPTED_SUBSET.md 2.15).

     A Number holds an integer exactly only up to 2^53 - 1, and a generated
     backend's int64 holds more. So every operation is computed exactly, as a
     BigInt, and an integer has one form: a Number where it is one a Number
     holds exactly, a BigInt where it is not. One value is then always one
     JavaScript value, and === between two integers is the comparison the
     document wrote. A result the declared type does not hold throws: the
     script engine's failure channel is the one a generated backend records,
     so a document that raises error.execution there raises it here. A signed
     minimum divided or reduced by -1 is one such throw: the remainder is
     mathematically 0, and it still fails overflow, since the division it comes
     from traps on the hardware and the contract makes that one answer on every
     backend (SCE_FORGE.md 3.4.1).

     The engine the Interpreter embeds has two defects in BigInt that this
     library does not rely on, measured against Node on the same cases: it
     orders a BigInt against a Number wrongly when both are negative (the
     lowering compares two integers as BigInts, so none is asked), and
     BigInt.asUintN gives a negative result for a width of 32 or 64 whose top
     bit is set (a result is masked here instead, and sign-extended by hand).

     A BigInt is a value only inside the expressions that compute it. The
     Interpreter's data model holds a Number, and a BigInt handed to it is lost
     without a word, so a value leaves the expressions through out(), which
     throws for one. Bitwise operations and shifts are outside the integer
     contract and wrap at their width (SCE_FORGE.md 3.4.1). A shift by the
     width or more shifts every bit out, and a shift by a negative count is out
     of range.

     An event's data reaches the Interpreter as untyped JSON, and a generated
     machine reads it through its event-schema. field() is that reading: the
     refusals are the generated backends' (no data, a bare value, a missing
     field, a value of another type or beyond its width), each thrown, so the
     expression that read the field fails the way an overflow does. A field a
     later revision of the schema added is read with a fourth argument, the
     value its default spells as JSON: a delivery that lacks the field, or
     carries no data at all, reads that value in its place and holds it to the
     field's type as a delivered one, which is what the generated lifts read.

     A value that may be absent (docs/adr/0055) is null when it is absent and
     the value itself when it is present, in a variable, a record's field, a
     payload and the wire alike. field() reads an optional field when its type
     is written {optional: T}: a delivered null is absent, a delivered value is
     held to T, and a delivery that lacks the key is refused unless the read is
     handed a default, which a field added by a later revision is; there a
     default of null is absent, and no fourth argument at all means no default.
     otherwise() is x ?? d: it answers x when x is present and calls d, the
     function the default is written in, only when x is absent, so the default
     is evaluated, and fails, only then. mapPresent() hands a present value to
     the function it is given and answers what that answers, and answers null
     for an absent one: how a present text is held to the bound of the place it
     is written to, and how a present value is written as it crosses the wire
     (a 64-bit integer as the text of its digits), absence staying null.

     A failure names itself. The thrown Error carries sceFailure, one of the
     names a generated backend reports (SCE_FORGE.md 3.4.1): overflow,
     divide-by-zero, precondition, out-of-range, capacity-exceeded. A caller that
     compares this engine with a backend reads that name, not the message. The one
     name no backend has is unrepresentable: the integer is defined, and the
     Interpreter's data model cannot hold it, or an operand is a Number that is
     not an integer a Number holds exactly. A read of an event's data carries
     none, since a backend answers that with error.execution as well, and only
     ever for the shape of the event; it carries sceRefusal instead, the code,
     the pointer and the bound of the refusal every backend's reader names.

     A hybrid invoke names the document it starts by a value its srcexpr
     computes, and the document is one its sce:candidates declares, each lowered
     beside the invoking one under the name its stem gives it. candidate() reduces
     the value to its stem by the rule every engine reads from the one table
     (tests/document_stem/document_stem.json) and answers the file name of the
     lowered candidate, which the Interpreter loads beside the invoking document.
     A value that names no declared document throws, so the srcexpr cannot be
     evaluated: error.execution, and nothing starts.

     A value of an enum is the name its variant declares. number() answers the
     number the variant of a value declares, from the table of the enum's names
     and numbers it is handed, and variant() the name of the variant a number
     declares, failing out-of-range for a number no variant declares: what
     Alias.toNumber and Alias.fromNumber are lowered to.

     A text a statechart holds is read as a record of a schema by lift()
     (docs/adr/0087, section 2.1), which answers {value, refusal}: the record
     and a null refusal, or a null value and the refusal of the first place
     that is not one, its code, pointer and bound, which the statement keeps as
     a record. The text is read by this library's own reader of JSON text,
     parseText(), which holds it to RFC 8259 alone as every runtime's reader
     does (tests/json_text/payload_refusal.json): whitespace of the four kinds
     between tokens, a number in its grammar, no raw control character in a
     text, no half of a UTF-16 pair in an escape, no repeated name. A number
     is kept as the text it was written in (a JsonNumber), so a whole number is
     read exactly and a fraction or an exponent where a whole number is is
     refused, as every runtime refuses it. Before the text is read, an empty
     one is refused, and so is a container opened deeper than the schema's
     values go, over the text's brackets alone (tooDeep()).

     The library is embedded in one attribute and its newlines collapse, so it
     holds no line comment and no statement that relies on a line break. */
  var SAFE = BigInt(Number.MAX_SAFE_INTEGER);
  var ZERO = BigInt(0);
  var ONE = BigInt(1);
  var FLOAT32_MAX = 3.4028234663852886e38;
  var RANGES = {};

  function fail(reason, name) {
    var error = new Error('sce-static: ' + reason);
    error.sceFailure = name;
    throw error;
  }

  /* A refusal of an event's data: the sentence, and beside it what a statechart
     reads as data (docs/adr/0087, section 2.4) in sceRefusal: the code, a name
     of sce:std/lift/refusal_code.scxml; the place as a strict RFC 6901 pointer,
     empty for the whole data; and the bound the value broke, for a text, a byte
     string or a list past it, null for every other code. It names no failure
     of sceFailure's, since a backend answers it with error.execution. */
  function refuse(code, pointer, reason, bound) {
    var error = new Error('sce-static: ' + reason);
    error.sceRefusal = { code: code, pointer: pointer, bound: bound === undefined ? null : bound };
    throw error;
  }

  /* The strict RFC 6901 pointer to the token below the pointer prefix: a slash,
     and the token with a tilde written ~0 and a slash written ~1. */
  function pointerBelow(prefix, token) {
    return prefix + '/' + String(token).split('~').join('~0').split('/').join('~1');
  }

  /* A number of a JSON text this library read, in the text it was written in:
     a whole number is read from it exactly, and one written with a fraction or
     an exponent is told from a whole one. */
  function JsonNumber(text) {
    this.text = text;
  }

  var ESCAPES = { '"': '"', '\\': '\\', '/': '/', b: '\b', f: '\f', n: '\n', r: '\r', t: '\t' };

  /* The one JSON value of text, read by RFC 8259 alone (sections 2, 6 and 7):
     whitespace of the four kinds between tokens; a number in its grammar, kept
     as a JsonNumber; a text with no raw control character and no half of a
     UTF-16 pair in an escape; an object, of no prototype, whose names do not
     repeat. A text that breaks any of it throws a SyntaxError saying where. */
  function parseText(text) {
    var at = 0;
    function malformed(what) {
      throw new SyntaxError(what);
    }
    function space() {
      while (at < text.length && ' \t\n\r'.indexOf(text.charAt(at)) >= 0) {
        at++;
      }
    }
    function isDigit(c) {
      return c >= '0' && c <= '9';
    }
    function digits() {
      while (isDigit(text.charAt(at))) {
        at++;
      }
    }
    function hex4() {
      var four = text.substr(at, 4);
      if (!/^[0-9a-fA-F]{4}$/.test(four)) {
        malformed('a \\u escape is not four hex digits');
      }
      at += 4;
      return parseInt(four, 16);
    }
    function string() {
      var out = '';
      at++;
      for (;;) {
        if (at >= text.length) {
          malformed('a text ends unclosed');
        }
        var c = text.charAt(at);
        at++;
        if (c === '"') {
          return out;
        }
        if (c === '\\') {
          var e = text.charAt(at);
          at++;
          if (e === 'u') {
            var code = hex4();
            if (code >= 0xD800 && code <= 0xDBFF) {
              if (text.substr(at, 2) !== '\\u') {
                malformed('a \\u escape is half of a character');
              }
              at += 2;
              var low = hex4();
              if (low < 0xDC00 || low > 0xDFFF) {
                malformed('a \\u escape is half of a character');
              }
              out += String.fromCharCode(code, low);
            } else if (code >= 0xDC00 && code <= 0xDFFF) {
              malformed('a \\u escape is half of a character');
            } else {
              out += String.fromCharCode(code);
            }
          } else if (Object.prototype.hasOwnProperty.call(ESCAPES, e)) {
            out += ESCAPES[e];
          } else {
            malformed('unknown escape \\' + e);
          }
        } else if (c < ' ') {
          malformed('a control character stands in a text unescaped');
        } else {
          out += c;
        }
      }
    }
    function number() {
      var start = at;
      if (text.charAt(at) === '-') {
        at++;
      }
      if (text.charAt(at) === '0') {
        at++;
      } else if (isDigit(text.charAt(at))) {
        digits();
      } else {
        malformed('a number has no digits');
      }
      if (text.charAt(at) === '.') {
        at++;
        if (!isDigit(text.charAt(at))) {
          malformed('a fraction has no digits');
        }
        digits();
      }
      if (text.charAt(at) === 'e' || text.charAt(at) === 'E') {
        at++;
        if (text.charAt(at) === '+' || text.charAt(at) === '-') {
          at++;
        }
        if (!isDigit(text.charAt(at))) {
          malformed('an exponent has no digits');
        }
        digits();
      }
      if (isDigit(text.charAt(at))) {
        malformed('a number has a leading zero');
      }
      return new JsonNumber(text.slice(start, at));
    }
    function container(close) {
      var object = close === '}';
      var out = object ? Object.create(null) : [];
      at++;
      space();
      if (text.charAt(at) === close) {
        at++;
        return out;
      }
      for (;;) {
        if (object) {
          space();
          if (text.charAt(at) !== '"') {
            malformed('a field name was expected');
          }
          var name = string();
          if (Object.prototype.hasOwnProperty.call(out, name)) {
            malformed('the field ' + name + ' appears twice');
          }
          space();
          if (text.charAt(at) !== ':') {
            malformed('a colon was expected');
          }
          at++;
          out[name] = value();
        } else {
          out.push(value());
        }
        space();
        var next = text.charAt(at);
        at++;
        if (next === close) {
          return out;
        }
        if (next !== ',') {
          malformed('a comma or the end of the container was expected');
        }
      }
    }
    function value() {
      space();
      var c = text.charAt(at);
      if (c === '{') {
        return container('}');
      }
      if (c === '[') {
        return container(']');
      }
      if (c === '"') {
        return string();
      }
      if (text.substr(at, 4) === 'true') {
        at += 4;
        return true;
      }
      if (text.substr(at, 5) === 'false') {
        at += 5;
        return false;
      }
      if (text.substr(at, 4) === 'null') {
        at += 4;
        return null;
      }
      if (c === '-' || isDigit(c)) {
        return number();
      }
      return malformed(at >= text.length ? 'it ends where a value was expected' : 'a value was expected');
    }
    var read = value();
    space();
    if (at < text.length) {
      malformed('it carries more than one JSON value');
    }
    return read;
  }

  /* The refusal of a container text opens deeper than depth, the deepest a value
     of a lift's schema holds (docs/adr/0087, section 2.1, item 4), or null when
     it opens none. Decided over the brackets alone, before the text is read as
     JSON, so that every runtime decides it alike: a text is skipped from its
     opening quote to the next quote no backslash escapes, each opening bracket
     outside one opens a level and each closing one closes one, and the first
     opened past depth is refused, whatever the text holds after it. A text that
     never closes ends the walk with nothing refused. The bound is depth.

     The pointer names the container refused (docs/adr/0087, section 2.4): for
     each container it stands in, outermost first, the element's index in an
     array (the commas the scan met at that level since the array opened) or the
     member's name in an object (the last text closed at that level before a
     colon, since the object opened or its last comma), each an RFC 6901 token.
     It stops before the first place it cannot write: a member with no name
     before its colon, a name that is no text (no JSON string, or one holding
     U+0000), or a token that would take the pointer past pathBound UTF-8 bytes,
     the room of the refusal record's path. The scan keeps one entry per open
     container, at most depth of them. */
  function tooDeep(text, depth, pathBound) {
    var open = [];
    for (var at = 0; at < text.length; at++) {
      var c = text.charAt(at);
      var top = open.length > 0 ? open[open.length - 1] : null;
      if (c === '"') {
        var start = at;
        at++;
        while (at < text.length && text.charAt(at) !== '"') {
          at += text.charAt(at) === '\\' ? 2 : 1;
        }
        if (at >= text.length) {
          return null;
        }
        if (top !== null) {
          top.lastText = text.slice(start, at + 1);
        }
      } else if (c === ':' && top !== null) {
        top.member = top.lastText;
        top.lastText = null;
      } else if (c === ',' && top !== null) {
        top.member = null;
        top.lastText = null;
        top.index++;
      } else if (c === '{' || c === '[') {
        if (open.length >= depth) {
          try {
            refuse('tooDeep', scanPointer(open, pathBound), 'the text opens a container deeper than the ' + String(depth) + ' levels a value of its schema holds', depth);
          } catch (refusal) {
            return refusal;
          }
        }
        open.push({ array: c === '[', index: 0, lastText: null, member: null });
      } else if ((c === '}' || c === ']') && top !== null) {
        open.pop();
      }
    }
    return null;
  }

  /* The UTF-8 bytes of text, or -1 for a text holding half of a UTF-16 pair,
     which no UTF-8 spells. */
  function utf8Length(text) {
    var n = 0;
    for (var i = 0; i < text.length; i++) {
      var c = text.charCodeAt(i);
      var next = i + 1 < text.length ? text.charCodeAt(i + 1) : 0;
      if (c < 0x80) {
        n += 1;
      } else if (c < 0x800) {
        n += 2;
      } else if (c >= 0xD800 && c <= 0xDBFF && next >= 0xDC00 && next <= 0xDFFF) {
        n += 4;
        i++;
      } else if (c >= 0xD800 && c <= 0xDFFF) {
        return -1;
      } else {
        n += 3;
      }
    }
    return n;
  }

  /* The pointer to the place open stands at, as tooDeep() writes it. */
  function scanPointer(open, pathBound) {
    var pointer = '';
    for (var i = 0; i < open.length; i++) {
      var token;
      if (open[i].array) {
        token = String(open[i].index);
      } else {
        var name = null;
        try {
          name = open[i].member === null ? null : parseText(open[i].member);
        } catch (e) {
          name = null;
        }
        if (typeof name !== 'string' || name.indexOf(String.fromCharCode(0)) >= 0 || utf8Length(name) < 0) {
          break;
        }
        token = name;
      }
      var next = pointerBelow(pointer, token);
      if (utf8Length(next) > pathBound) {
        break;
      }
      pointer = next;
    }
    return pointer;
  }

  /* The text a lift reads, as the value of type, a record of its schema
     (docs/adr/0087, section 2.1), or the refusal thrown of the first place
     that is not one: an empty text, a container past depth, a text that is
     not one JSON object, then what a delivery of the schema is refused for. */
  function liftValue(text, type, depth, pathBound) {
    if (typeof text !== 'string') {
      fail('expected the text a lift reads, read ' + String(text));
    }
    if (text.replace(/[ \t\n\r]/g, '') === '') {
      refuse('notJson', '', 'the text is empty');
    }
    var deep = tooDeep(text, depth, pathBound);
    if (deep !== null) {
      throw deep;
    }
    var data;
    try {
      data = parseText(text);
    } catch (e) {
      refuse('notJson', '', 'the text is not JSON (' + String(e.message) + ')');
    }
    if (data === null || typeof data !== 'object' || Array.isArray(data) || data instanceof JsonNumber) {
      refuse('notObject', '', 'the text is one JSON value, and its schema declares named fields');
    }
    var value = readValue(data, type, '', '');
    held(value, '');
    return value;
  }

  /* Refuses the first integer of value the data model cannot hold: a 64-bit one
     past 2^53 - 1, which a delivery's reader answers as a BigInt for the
     expressions that compute with it, and which no variable of this engine
     holds. It is a whole number past the width this engine gives the type, so
     it is refused as one is (outOfRange) at its pointer, where a generated
     backend, whose int64 holds it, reads it. A lift raises nothing, so it
     answers the refusal rather than failing as unrepresentable. */
  function held(value, pointer) {
    if (typeof value === 'bigint') {
      refuse('outOfRange', pointer, '\'' + pointer + '\' is ' + String(value) + ', past the integers a Number holds exactly (2^53 - 1), which a variable of this engine holds');
    }
    if (value !== null && typeof value === 'object') {
      Object.keys(value).forEach(function (key) { held(value[key], pointerBelow(pointer, key)); });
    }
  }

  /* <sce:lift>: {value, refusal}, one of them null. It throws nothing: every
     text is a record of the type or a refusal of one, an integer the data model
     cannot hold included (held()). */
  function lift(text, type, depth, pathBound) {
    try {
      return { value: liftValue(text, type, depth, pathBound), refusal: null };
    } catch (e) {
      if (e.sceRefusal === undefined) {
        throw e;
      }
      return { value: null, refusal: e.sceRefusal };
    }
  }

  function exact(value) {
    if (typeof value === 'bigint') {
      return value;
    }
    if (!Number.isSafeInteger(value)) {
      fail('expected an integer a Number holds exactly, read ' + String(value), 'unrepresentable');
    }
    return BigInt(value);
  }

  function canonical(value) {
    return value > SAFE || value < -SAFE ? value : Number(value);
  }

  function fromReal(value) {
    return Number.isInteger(value) && !Number.isSafeInteger(value) ? BigInt(value) : value;
  }

  function check(value) {
    if (typeof value === 'bigint') {
      fail('the integer ' + String(value) + ' is beyond the integers a Number holds exactly, which a variable of the machine holds', 'unrepresentable');
    }
    if (value !== null && typeof value === 'object') {
      Object.keys(value).forEach(function (key) { check(value[key]); });
    }
  }

  function integer(signed, bits) {
    var span = ONE << BigInt(bits);
    var min = signed ? -(span >> ONE) : ZERO;
    var max = signed ? (span >> ONE) - ONE : span - ONE;
    var name = (signed ? 'int' : 'uint') + bits;
    var width = BigInt(bits);
    var mask = span - ONE;
    RANGES[name] = { min: min, max: max };

    function hold(value) {
      if (value < min || value > max) {
        fail('the value does not fit ' + name, 'overflow');
      }
      return canonical(value);
    }

    function wrap(value) {
      var low = value & mask;
      return canonical(signed && low > max ? low - span : low);
    }

    /* docs/adr/0063: a saturating operation is its exact result held to
       the width's bounds, as a wrapping one is the same result taken modulo
       the width (`wrap`). Both are total. */
    function clamp(value) {
      return canonical(value < min ? min : value > max ? max : value);
    }

    function count(value) {
      var n = exact(value);
      if (n < ZERO) {
        fail('a shift by the negative count ' + String(n), 'out-of-range');
      }
      return n;
    }

    function divisor(value) {
      var d = exact(value);
      if (d === ZERO) {
        fail('division by zero', 'divide-by-zero');
      }
      return d;
    }

    function remainder(a, b) {
      var x = exact(a);
      var d = divisor(b);
      if (signed && x === min && d === -ONE) {
        fail('the remainder of the minimum of ' + name + ' by -1 is not a value', 'overflow');
      }
      return hold(x % d);
    }

    return {
      add: function (a, b) { return hold(exact(a) + exact(b)); },
      sub: function (a, b) { return hold(exact(a) - exact(b)); },
      mul: function (a, b) { return hold(exact(a) * exact(b)); },
      div: function (a, b) { return hold(exact(a) / divisor(b)); },
      rem: function (a, b) { return remainder(a, b); },
      neg: function (a) { return hold(-exact(a)); },
      narrow: function (a) { return hold(exact(a)); },
      wrappingAdd: function (a, b) { return wrap(exact(a) + exact(b)); },
      wrappingSub: function (a, b) { return wrap(exact(a) - exact(b)); },
      wrappingMul: function (a, b) { return wrap(exact(a) * exact(b)); },
      wrappingNeg: function (a) { return wrap(-exact(a)); },
      saturatingAdd: function (a, b) { return clamp(exact(a) + exact(b)); },
      saturatingSub: function (a, b) { return clamp(exact(a) - exact(b)); },
      saturatingMul: function (a, b) { return clamp(exact(a) * exact(b)); },
      saturatingNeg: function (a) { return clamp(-exact(a)); },
      and: function (a, b) { return wrap(exact(a) & exact(b)); },
      or: function (a, b) { return wrap(exact(a) | exact(b)); },
      xor: function (a, b) { return wrap(exact(a) ^ exact(b)); },
      not: function (a) { return wrap(~exact(a)); },
      shl: function (a, n) {
        var c = count(n);
        return wrap(c >= width ? ZERO : exact(a) << c);
      },
      shr: function (a, n) {
        var c = count(n);
        var x = exact(a);
        return wrap(c >= width ? (x < ZERO ? -ONE : ZERO) : x >> c);
      },
      ushr: function (a, n) {
        var c = count(n);
        var u = exact(a) & mask;
        return wrap(c >= width ? ZERO : u >> c);
      }
    };
  }

  /* The event's data reaches field() already read by the script engine, as the
     value its JSON spells, or as the text itself where it spells none: a text
     that is not JSON, or holds an object that repeats a name, is the engine's
     to refuse, and is not seen here as the text it was. */
  function field(data, name, type, fallback) {
    var revised = fallback !== undefined;
    if (data === undefined || data === null || (revised && data === '')) {
      if (!revised) {
        refuse('notJson', '', 'the event carries no data');
      }
      data = {};
    }
    if (typeof data !== 'object' || Array.isArray(data) || data instanceof JsonNumber) {
      refuse('notObject', '', 'the event data is a bare value, and its schema declares named fields');
    }
    var value;
    var pointer = pointerBelow('', name);
    if (Object.prototype.hasOwnProperty.call(data, name)) {
      value = data[name];
    } else if (revised) {
      value = fallback;
    } else {
      refuse('missing', pointer, 'the event data has no \'' + name + '\'');
    }
    return readValue(value, type, name, pointer);
  }

  /* The data of an event whose schema holds a composed value (docs/adr/0086,
     section 6), read whole as type, the record of its schema, describes: each
     field as field() reads it, and a field a later revision added as its
     default where the delivery lacks it. So a delivery outside its type is
     refused whole, at the first place that is not one, by any expression that
     reads any part of it, before anything of it is kept. The parts are walked
     into the data model, as a foreach binds an element of a list to its item,
     and the data model holds no integer past 2^53 - 1: one is refused at its
     pointer (held()), as a lift refuses it, rather than lost on the way. */
  function payload(data, type) {
    var read = {};
    Object.keys(type.record).forEach(function (id) {
      var revised = type.defaults !== undefined && Object.prototype.hasOwnProperty.call(type.defaults, id);
      read[id] = revised ? field(data, id, type.record[id], type.defaults[id]) : field(data, id, type.record[id]);
    });
    held(read, '');
    return read;
  }

  /* The door (docs/adr/0086, sections 4.2 and 4.4). A generated machine lifts the
     payload of an event it reads when it takes the event, before it selects a
     transition, and refuses a delivery outside its type whole: no transition is
     selected for it, and an error.execution that carries the refusal is taken in
     its place. door() is handed, for each event whose payload the document reads,
     the record its schema declares ({record: {...}} with the defaults of the
     fields a later revision added), and installs dequeued() as __sceDequeued,
     which the Interpreter calls with _event each time it binds one, before it
     selects a transition. dequeued() reads every field of the record as field()
     reads one, and answers the sentence of the first refusal, on which the
     Interpreter takes error.execution in the event's place, or the empty text.
     A field past its bound so refuses the delivery even on a route that reads no
     such field. The door of an event whose schema holds a composed value is
     handed composed: it reads the data whole, as payload() does, and keeps the
     value it read for the event being processed, which every expression of the
     content then reads through taken(): one walk for each delivery. */
  var doors = {};
  var kept = null;
  function dequeued(event) {
    kept = null;
    if (event === undefined || event === null || !Object.prototype.hasOwnProperty.call(doors, event.name)) {
      return '';
    }
    var door = doors[event.name];
    try {
      if (door.composed) {
        kept = { name: event.name, value: payload(event.data, door.type) };
        return '';
      }
      Object.keys(door.type.record).forEach(function (id) {
        var defaulted = door.type.defaults !== undefined && Object.prototype.hasOwnProperty.call(door.type.defaults, id);
        field(event.data, id, door.type.record[id], defaulted ? door.type.defaults[id] : undefined);
      });
      return '';
    } catch (refusal) {
      var why = refusal !== null && typeof refusal === 'object' && refusal.message !== undefined ? refusal.message : refusal;
      return '`' + String(event.name) + '` payload: ' + String(why);
    }
  }

  /* The composed payload of the event name names, as its door read it when the
     event was taken. One that arrived under a longer name the transition's
     descriptor matches was taken by no door of that name, and is read here,
     whole, from data, as the door would have read it. */
  function taken(name, data) {
    if (kept !== null && kept.name === name) {
      return kept.value;
    }
    return payload(data, doors[name].type);
  }

  /* What a type descriptor describes. A scalar is named by its type, an enum by
     the array of its names, and a composed type by an object with one key that
     names its constructor: {optional: T}, {list: T, max: B}, with min: M beside
     a list that states a lower count, {record: {id: T, ...}, defaults: {...}},
     {string: B}, {bytes: B} (docs/adr/0086, sections 4.3 and 15), and
     {union: {tag: K, cases: {variant: null or a record's descriptor, ...}}},
     the tag's enum the cases' names (docs/adr/0092, section 4). */
  function kindOf(type) {
    if (Array.isArray(type)) {
      return 'enum';
    }
    if (type !== null && typeof type === 'object') {
      var keys = ['optional', 'list', 'record', 'union', 'string', 'bytes'];
      for (var i = 0; i < keys.length; i++) {
        if (Object.prototype.hasOwnProperty.call(type, keys[i])) {
          return keys[i] === 'string' ? 'bounded-string' : keys[i] === 'bytes' ? 'bounded-bytes' : keys[i];
        }
      }
      fail('a type descriptor names no constructor');
    }
    return type;
  }

  function below(path, segment) {
    return path + '/' + String(segment);
  }

  /* One value of an event's data as its type declares it, at the path through
     every depth that names it: a field's name, then each index and field name
     below it, joined by a slash (docs/adr/0086, section 4). A value outside its
     type is refused before anything of it is kept, the refusal naming the path;
     a list past its bound is refused before any element of it is read, and a key
     a record's schema does not declare is stepped over. A field of a record a
     later revision of its schema added is read out of the record's defaults.
     pointer is the strict RFC 6901 pointer to the same place, which a refusal
     carries beside its sentence. */
  function readValue(value, type, path, pointer) {
    var kind = kindOf(type);
    var where = '\'' + path + '\' ';
    if (kind === 'optional') {
      return value === null ? null : readValue(value, type.optional, path, pointer);
    }
    if (value === null) {
      refuse('nullNotOptional', pointer, where + 'is null, and its type is not optional');
    }
    if (kind === 'list') {
      if (!Array.isArray(value)) {
        refuse('wrongKind', pointer, where + 'is not a list');
      }
      if (value.length > type.max) {
        refuse('tooMany', pointer, where + 'holds ' + String(value.length) + ' elements, past the ' + String(type.max) + ' its type declares', type.max);
      }
      /* A lower count, where the type states one (docs/adr/0086, section 15),
         held before any element is read as the bound is. */
      if (type.min !== undefined && value.length < type.min) {
        refuse('tooFew', pointer, where + 'holds ' + String(value.length) + ' elements, fewer than the ' + String(type.min) + ' its type declares', type.min);
      }
      return value.map(function (item, index) { return readValue(item, type.list, below(path, index), pointerBelow(pointer, index)); });
    }
    if (kind === 'record') {
      if (typeof value !== 'object' || Array.isArray(value) || value instanceof JsonNumber) {
        refuse('wrongKind', pointer, where + 'is not a record');
      }
      var held = {};
      Object.keys(type.record).forEach(function (id) {
        var member;
        if (Object.prototype.hasOwnProperty.call(value, id)) {
          member = value[id];
        } else if (type.defaults !== undefined && Object.prototype.hasOwnProperty.call(type.defaults, id)) {
          member = type.defaults[id];
        } else {
          refuse('missing', pointerBelow(pointer, id), 'the event data has no \'' + below(path, id) + '\'');
        }
        held[id] = readValue(member, type.record[id], below(path, id), pointerBelow(pointer, id));
      });
      return held;
    }
    if (kind === 'union') {
      /* The tag first, wherever it stands in the object, read as its enum is read;
         then the record of the case it names, from the same object, a key the case
         does not declare stepped over as a record steps over one. The value is the
         wire's form: the case's fields beside its tag (docs/adr/0092, section 4). */
      if (typeof value !== 'object' || Array.isArray(value) || value instanceof JsonNumber) {
        refuse('wrongKind', pointer, where + 'is not a union');
      }
      var tag = type.union.tag;
      var tagPointer = pointerBelow(pointer, tag);
      if (!Object.prototype.hasOwnProperty.call(value, tag)) {
        refuse('missing', tagPointer, 'the event data has no \'' + below(path, tag) + '\'');
      }
      var variant = readValue(value[tag], Object.keys(type.union.cases), below(path, tag), tagPointer);
      var chosen = type.union.cases[variant];
      var named = chosen === null ? {} : readValue(value, chosen, path, pointer);
      named[tag] = variant;
      return named;
    }
    if (kind === 'enum') {
      text(value, where, pointer);
      if (type.indexOf(value) < 0) {
        refuse('unknownVariant', pointer, where + '(' + value + ') is not a variant of its enum');
      }
      return value;
    }
    var range = RANGES[kind];
    if (range !== undefined) {
      var whole;
      if ((kind === 'int64' || kind === 'uint64') && typeof value === 'string') {
        /* A 64-bit integer crosses as the text of its digits: a number past
           2^53 - 1 is not exact in the data a Number carries, so it is written
           as text on every engine and read from it here. */
        if (!(kind === 'int64' ? /^(0|-?[1-9][0-9]*)$/ : /^(0|[1-9][0-9]*)$/).test(value)) {
          refuse('badNumber', pointer, where + 'is not a whole number written as digits');
        }
        whole = BigInt(value);
      } else if (value instanceof JsonNumber) {
        /* A number this library read is read from the text it was written in,
           exactly: a fraction, an exponent and -0 are no spelling of a whole
           number, and a 64-bit one past 2^53 - 1 is not exact everywhere. */
        if (value.text === '-0' || !/^-?(0|[1-9][0-9]*)$/.test(value.text)) {
          refuse('badNumber', pointer, where + 'is not a whole number (' + value.text + ')');
        }
        whole = BigInt(value.text);
        if ((kind === 'int64' || kind === 'uint64') && (whole > SAFE || whole < -SAFE)) {
          refuse('badNumber', pointer, where + 'is a number past 2^53 - 1, which is not exact everywhere');
        }
      } else {
        if (typeof value !== 'number') {
          refuse('wrongKind', pointer, where + 'is not a number');
        }
        /* -0 is a JSON number and no spelling of a whole number. */
        if (Object.is(value, -0)) {
          refuse('badNumber', pointer, where + 'is not a whole number (-0)');
        }
        if (!Number.isSafeInteger(value)) {
          /* A whole number past every Number's exact integers is past every
             width but a 64-bit one's, whose number is held to 2^53 - 1; any
             other number is in no whole spelling. */
          refuse(Number.isInteger(value) && kind !== 'int64' && kind !== 'uint64' ? 'outOfRange' : 'badNumber', pointer, where + 'is not an integer a Number holds exactly');
        }
        whole = BigInt(value);
      }
      if (whole < range.min || whole > range.max) {
        /* Below zero is no spelling of an unsigned number. */
        refuse(range.min === ZERO && whole < ZERO ? 'badNumber' : 'outOfRange', pointer, where + 'does not fit ' + kind);
      }
      return canonical(whole);
    }
    if (kind === 'float32' || kind === 'float64') {
      if (value instanceof JsonNumber) {
        value = Number(value.text);
      }
      if (typeof value !== 'number') {
        refuse('wrongKind', pointer, where + 'is not a number');
      }
      /* A number past the largest double (1e999), which a reader rounds to an
         infinity no runtime holds, does not fit, as one past the largest single
         does not fit a float32. */
      if (!isFinite(value)) {
        refuse('outOfRange', pointer, where + 'does not fit ' + kind);
      }
      if (kind === 'float32') {
        /* A Number is a binary64: the field a schema declares float32 holds the
           binary32 nearest it, and a number past the range of a single does not
           fit it, as an integer past its width does not. */
        if (Math.abs(value) > FLOAT32_MAX) {
          refuse('outOfRange', pointer, where + 'does not fit float32');
        }
        return Math.fround(value);
      }
      return value;
    }
    if (kind === 'bool') {
      if (typeof value !== 'boolean') {
        refuse('wrongKind', pointer, where + 'is not a truth value');
      }
      return value;
    }
    if (kind === 'string' || kind === 'bounded-string') {
      text(value, where, pointer);
      /* A text is held to the bound its type declares wherever it stands, the
         top level of a payload included (docs/adr/0086, section 4.4). */
      var size = utf8Length(value);
      if (kind === 'bounded-string' && size > type.string) {
        refuse('tooLong', pointer, where + 'is ' + String(size) + ' bytes, past the ' + String(type.string) + ' its type declares', type.string);
      }
      return value;
    }
    if (kind === 'bytes' || kind === 'bounded-bytes') {
      /* The wire spells a byte string as its byte-exact Latin-1 text, one
         character to a byte, so the value is the text, held to the bound its
         type declares when the type is written with one. */
      if (typeof value !== 'string') {
        refuse('wrongKind', pointer, where + 'is not a byte string');
      }
      for (var i = 0; i < value.length; i++) {
        if (value.charCodeAt(i) > 0xFF) {
          refuse('notByteText', pointer, where + 'carries a character above U+00FF, which no single byte spells');
        }
      }
      if (kind === 'bounded-bytes' && value.length > type.bytes) {
        refuse('tooLong', pointer, where + 'is ' + String(value.length) + ' bytes, past the ' + String(type.bytes) + ' its type declares', type.bytes);
      }
      return value;
    }
    fail('a value of type ' + String(type) + ' has no reader');
  }

  /* A text of an event's data, held to no bound: what a text field holds and
     what an enum's variant is named by. A text holding U+0000 is refused, since
     a backend that holds a text NUL-terminated would end it there, so no backend
     holds one (docs/adr/0087, section 2.3); a byte string may hold the byte
     zero. */
  function text(value, where, pointer) {
    if (typeof value !== 'string') {
      refuse('wrongKind', pointer, where + 'is not a text');
    }
    if (value.indexOf('\u0000') >= 0) {
      refuse('nulInText', pointer, where + 'carries U+0000, which no text holds');
    }
    return value;
  }

  /* A byte string is the text of its bytes, one character to a byte: a value that
     is not text, or holds a character past U+00FF, is no byte string. */
  function byteText(value, notText) {
    if (typeof value !== 'string') {
      fail(notText);
    }
    for (var i = 0; i < value.length; i++) {
      if (value.charCodeAt(i) > 0xFF) {
        fail('the byte string holds a character past U+00FF, which is no byte');
      }
    }
    return value;
  }

  /* The UTF-8 bytes a string is, which is what a string's bound counts and what
     `len` of a string reads on every backend: a unit below U+0080 is one byte, below
     U+0800 two, a surrogate pair four and any other unit three. */
  function utf8Length(value) {
    if (typeof value !== 'string') {
      fail('expected a string, read ' + String(value));
    }
    var bytes = 0;
    for (var i = 0; i < value.length; i++) {
      var unit = value.charCodeAt(i);
      if (unit < 0x80) {
        bytes += 1;
      } else if (unit < 0x800) {
        bytes += 2;
      } else if (unit >= 0xD800 && unit <= 0xDBFF && i + 1 < value.length && value.charCodeAt(i + 1) >= 0xDC00 && value.charCodeAt(i + 1) <= 0xDFFF) {
        bytes += 4;
        i++;
      } else {
        bytes += 3;
      }
    }
    return bytes;
  }

  /* The fence of a text template's untrusted hole (docs/adr/0087, section 3.1,
     item 4): every occurrence of either marker removed from the value, the
     leftmost first (the longer where both start at one place), until neither
     occurs, so a value cannot rebuild a marker from two halves. The markers are
     printable ASCII, each one UTF-16 unit as it is one UTF-8 byte, so removing
     them from a string's units removes what the generated backends remove from
     its bytes. An empty marker is no marker. */
  function removeMarkers(value, open, close) {
    var text = value;
    var longest = Math.max(open.length, close.length);
    var from = 0;
    for (;;) {
      var at = -1;
      var marker = 0;
      for (var i = from; i < text.length && at < 0; i++) {
        var opens = open.length > 0 && text.startsWith(open, i);
        var closes = close.length > 0 && text.startsWith(close, i);
        if (opens || closes) {
          at = i;
          marker = opens && closes ? longest : (opens ? open.length : close.length);
        }
      }
      if (at < 0) {
        return text;
      }
      text = text.slice(0, at) + text.slice(at + marker);
      from = Math.max(0, at - (longest - 1));
    }
  }

  return {
    /* A text template rendered from its parts, in order (docs/adr/0087,
       section 3.1, item 7): a string as it is, and an untrusted value,
       `{fenced: value}`, between the markers with every marker removed. */
    render: function (parts, open, close) {
      var text = '';
      for (var i = 0; i < parts.length; i++) {
        var part = parts[i];
        if (typeof part === 'string') {
          text += part;
        } else {
          text += open + removeMarkers(String(part.fenced), open, close) + close;
        }
      }
      return text;
    },
    I8: integer(true, 8),
    I16: integer(true, 16),
    I32: integer(true, 32),
    I64: integer(true, 64),
    U8: integer(false, 8),
    U16: integer(false, 16),
    U32: integer(false, 32),
    U64: integer(false, 64),
    field: field,
    payload: payload,
    door: function (name, type, composed) {
      doors[name] = { type: type, composed: composed === true };
      globalThis.__sceDequeued = dequeued;
    },
    taken: taken,
    lift: lift,
    parseText: parseText,
    otherwise: function (value, fallback) {
      return value === null ? fallback() : value;
    },
    mapPresent: function (value, map) {
      return value === null ? null : map(value);
    },
    out: function (value) {
      check(value);
      return value;
    },
    append: function (list, capacity, value) {
      if (!Array.isArray(list)) {
        fail('expected a list, read ' + String(list));
      }
      if (list.length >= capacity) {
        fail('the list already holds its capacity of ' + String(capacity), 'capacity-exceeded');
      }
      return list.concat([value]);
    },
    utf8Length: utf8Length,
    bounded: function (value, capacity) {
      var bytes = utf8Length(value);
      if (bytes > capacity) {
        fail('the string holds ' + String(bytes) + ' UTF-8 bytes, past its capacity of ' + String(capacity), 'capacity-exceeded');
      }
      return value;
    },
    boundedBytes: function (value, capacity) {
      byteText(value, 'expected a byte string, read ' + String(value));
      if (value.length > capacity) {
        fail('the byte string holds ' + String(value.length) + ' bytes, past its capacity of ' + String(capacity), 'capacity-exceeded');
      }
      return value;
    },
    within: function (list, capacity) {
      if (!Array.isArray(list)) {
        fail('expected a list, read ' + String(list));
      }
      if (list.length > capacity) {
        fail('the list holds ' + String(list.length) + ', past its capacity of ' + String(capacity), 'capacity-exceeded');
      }
      return list;
    },
    extend: function (list, capacity, values) {
      if (!Array.isArray(list) || !Array.isArray(values)) {
        fail('expected two lists, read ' + String(list) + ' and ' + String(values));
      }
      if (list.length + values.length > capacity) {
        fail('the list holds ' + String(list.length) + ' and takes ' + String(values.length) + ' more than its capacity of ' + String(capacity), 'capacity-exceeded');
      }
      return list.concat(values);
    },
    set: function (record, field, value) {
      if (record === null || typeof record !== 'object' || Array.isArray(record)) {
        fail('expected a record, read ' + String(record));
      }
      var next = {};
      Object.keys(record).forEach(function (key) { next[key] = record[key]; });
      next[field] = value;
      return next;
    },
    require: function (holds, what) {
      if (holds !== true) {
        fail('the precondition ' + what + ' does not hold', 'precondition');
      }
    },
    algorithms: {},
    route: function (value, entries) {
      if (typeof value !== 'string') {
        fail('expected the string a computed target is, read ' + String(value));
      }
      return entries.indexOf(value) < 0 ? '' : value;
    },
    processor: function (value, entries) {
      if (typeof value !== 'string') {
        fail('expected the string a computed type is, read ' + String(value));
      }
      if (entries.indexOf(value) < 0) {
        fail('the type ' + value + ' is not one the send declares in sce:types');
      }
      return value;
    },
    candidate: function (value, stems) {
      if (typeof value !== 'string') {
        fail('expected the string a hybrid invoke names its document by, read ' + String(value));
      }
      var name = value.split('/').pop().split('\\').pop();
      while (name.indexOf('file:') === 0) {
        name = name.slice(5);
      }
      var dot = name.lastIndexOf('.');
      var stem = dot > 0 ? name.slice(0, dot) : name;
      if (stems.indexOf(stem) < 0) {
        fail('the document ' + value + ' is not one the invoke declares in sce:candidates');
      }
      return stem + '.scxml';
    },
    at: function (collection, index) {
      if (!Number.isSafeInteger(index) || index < 0 || index >= collection.length) {
        fail('the index ' + String(index) + ' is outside the collection', 'out-of-range');
      }
      return collection[index];
    },
    number: function (numbers, value) {
      if (typeof value !== 'string' || !Object.prototype.hasOwnProperty.call(numbers, value)) {
        fail('expected a variant of the enum, read ' + String(value));
      }
      return numbers[value];
    },
    variant: function (names, number) {
      var key = String(number);
      if (!Object.prototype.hasOwnProperty.call(names, key)) {
        fail('no variant of the enum declares the number ' + key, 'out-of-range');
      }
      return names[key];
    },
    fromReal: fromReal,
    round: function (value) {
      return fromReal((value >= 0 ? Math.floor(value + 0.5) : Math.ceil(value - 0.5)) + 0);
    }
  };
})()
