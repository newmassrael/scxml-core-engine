(function () {
  /* SceStatic: the checked arithmetic and the typed event data of a
     datamodel="sce-static" document, for the script engine the Interpreter
     runs it on (docs/SCE_ACCEPTED_SUBSET.md 2.15).

     A Number holds an integer exactly only up to 2^53 - 1, and a generated
     backend's int64 holds more. So every operation is computed exactly, as a
     BigInt, and handed back as a Number only where the result fits the
     declared type AND is a Number's exact integer. Anything else throws: the
     script engine's failure channel is the one a generated backend records,
     so a document that raises error.execution there raises it here.

     An event's data reaches the Interpreter as untyped JSON, and a generated
     machine reads it through its event-schema. field() is that reading: the
     refusals are the generated backends' (no data, a bare value, a missing
     field, a value of another type or beyond its width), each thrown, so the
     expression that read the field fails the way an overflow does.

     The library is embedded in one attribute and its newlines collapse, so it
     holds no line comment and no statement that relies on a line break. */
  var SAFE = BigInt(Number.MAX_SAFE_INTEGER);
  var ZERO = BigInt(0);
  var ONE = BigInt(1);
  var RANGES = {};

  function fail(reason) {
    throw new Error('sce-static: ' + reason);
  }

  function exact(value) {
    if (!Number.isSafeInteger(value)) {
      fail('expected an integer a Number holds exactly, read ' + String(value));
    }
    return BigInt(value);
  }

  function integer(signed, bits) {
    var span = ONE << BigInt(bits);
    var min = signed ? -(span >> ONE) : ZERO;
    var max = signed ? (span >> ONE) - ONE : span - ONE;
    var name = (signed ? 'int' : 'uint') + bits;
    RANGES[name] = { min: min, max: max };

    function hold(value) {
      if (value < min || value > max) {
        fail('the value does not fit ' + name);
      }
      if (value > SAFE || value < -SAFE) {
        fail('the value of ' + name + ' is beyond the integers a Number holds exactly');
      }
      return Number(value);
    }

    function divisor(value) {
      var d = exact(value);
      if (d === ZERO) {
        fail('division by zero');
      }
      return d;
    }

    return {
      add: function (a, b) { return hold(exact(a) + exact(b)); },
      sub: function (a, b) { return hold(exact(a) - exact(b)); },
      mul: function (a, b) { return hold(exact(a) * exact(b)); },
      div: function (a, b) { return hold(exact(a) / divisor(b)); },
      rem: function (a, b) { return hold(exact(a) % divisor(b)); },
      neg: function (a) { return hold(-exact(a)); },
      narrow: function (a) { return hold(exact(a)); }
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
    fail('a field of type ' + String(type) + ' has no reader');
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
    append: function (list, capacity, value) {
      if (!Array.isArray(list)) {
        fail('expected a list, read ' + String(list));
      }
      if (list.length >= capacity) {
        fail('the list already holds its capacity of ' + String(capacity));
      }
      return list.concat([value]);
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
        fail('the precondition ' + what + ' does not hold');
      }
    },
    algorithms: {},
    at: function (collection, index) {
      if (!Number.isSafeInteger(index) || index < 0 || index >= collection.length) {
        fail('the index ' + String(index) + ' is outside the collection');
      }
      return collection[index];
    },
    round: function (value) {
      return (value >= 0 ? Math.floor(value + 0.5) : Math.ceil(value - 0.5)) + 0;
    }
  };
})()
