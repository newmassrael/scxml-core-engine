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
    if (typeof data !== 'object' || Array.isArray(data)) {
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

  /* What a type descriptor describes. A scalar is named by its type, an enum by
     the array of its names, and a composed type by an object with one key that
     names its constructor: {optional: T}, {list: T, max: B}, {record: {id: T,
     ...}, defaults: {...}}, {string: B}, {bytes: B} (docs/adr/0086, section 4.3). */
  function kindOf(type) {
    if (Array.isArray(type)) {
      return 'enum';
    }
    if (type !== null && typeof type === 'object') {
      var keys = ['optional', 'list', 'record', 'string', 'bytes'];
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
      return value.map(function (item, index) { return readValue(item, type.list, below(path, index), pointerBelow(pointer, index)); });
    }
    if (kind === 'record') {
      if (typeof value !== 'object' || Array.isArray(value)) {
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
      } else {
        if (typeof value !== 'number') {
          refuse('wrongKind', pointer, where + 'is not a number');
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
      if (typeof value !== 'number') {
        refuse('wrongKind', pointer, where + 'is not a number');
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

  return {
    I8: integer(true, 8),
    I16: integer(true, 16),
    I32: integer(true, 32),
    I64: integer(true, 64),
    U8: integer(false, 8),
    U16: integer(false, 16),
    U32: integer(false, 32),
    U64: integer(false, 64),
    field: field,
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
