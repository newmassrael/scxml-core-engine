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
     expression that read the field fails the way an overflow does.

     A failure names itself. The thrown Error carries sceFailure, one of the
     names a generated backend reports (SCE_FORGE.md 3.4.1): overflow,
     divide-by-zero, precondition, out-of-range, capacity-exceeded. A caller that
     compares this engine with a backend reads that name, not the message. The one
     name no backend has is unrepresentable: the integer is defined, and the
     Interpreter's data model cannot hold it, or an operand is a Number that is
     not an integer a Number holds exactly. A read of an event's data carries
     none, since a backend answers that with error.execution as well, and only
     ever for the shape of the event.

     A hybrid invoke names the document it starts by a value its srcexpr
     computes, and the document is one its sce:candidates declares, each lowered
     beside the invoking one under the name its stem gives it. candidate() reduces
     the value to its stem by the rule every engine reads from the one table
     (tests/document_stem/document_stem.json) and answers the file name of the
     lowered candidate, which the Interpreter loads beside the invoking document.
     A value that names no declared document throws, so the srcexpr cannot be
     evaluated: error.execution, and nothing starts.

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

  function field(data, name, type) {
    if (data === undefined || data === null) {
      fail('the event carries no data');
    }
    if (typeof data !== 'object' || Array.isArray(data)) {
      fail('the event data is a bare value, and its schema declares named fields');
    }
    if (!Object.prototype.hasOwnProperty.call(data, name)) {
      fail('the event data has no ' + name);
    }
    var value = data[name];
    if (Array.isArray(type)) {
      if (typeof value !== 'string') {
        fail(name + ' is not text');
      }
      if (type.indexOf(value) < 0) {
        fail(name + ' (' + value + ') is not a variant of its enum');
      }
      return value;
    }
    var range = RANGES[type];
    if (range !== undefined) {
      if (typeof value !== 'number') {
        fail(name + ' is not a number');
      }
      if (!Number.isSafeInteger(value)) {
        fail(name + ' is not an integer a Number holds exactly');
      }
      if (BigInt(value) < range.min || BigInt(value) > range.max) {
        fail(name + ' does not fit ' + type);
      }
      return value;
    }
    if (type === 'float32' || type === 'float64') {
      if (typeof value !== 'number') {
        fail(name + ' is not a number');
      }
      if (type === 'float32') {
        /* A Number is a binary64: the field a schema declares float32 holds the
           binary32 nearest it, and a number past the range of a single does not
           fit it, as an integer past its width does not. */
        if (Math.abs(value) > FLOAT32_MAX) {
          fail(name + ' does not fit float32');
        }
        return Math.fround(value);
      }
      return value;
    }
    if (type === 'bool') {
      if (typeof value !== 'boolean') {
        fail(name + ' is not a truth value');
      }
      return value;
    }
    if (type === 'string') {
      if (typeof value !== 'string') {
        fail(name + ' is not text');
      }
      return value;
    }
    if (type === 'bytes') {
      /* The wire spells a byte string as its byte-exact Latin-1 text, one
         character to a byte, so the field is the text and no longer than its
         schema's bound only where it is written (boundedBytes). */
      return byteText(value, name + ' is not text');
    }
    fail('a field of type ' + String(type) + ' has no reader');
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
    bounded: function (value, capacity) {
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
    fromReal: fromReal,
    round: function (value) {
      return fromReal((value >= 0 ? Math.floor(value + 0.5) : Math.ceil(value - 0.5)) + 0);
    }
  };
})()
