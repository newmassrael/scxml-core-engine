(function () {
  /* SceStatic: the checked arithmetic of a datamodel="sce-static" document,
     for the script engine the Interpreter runs it on
     (docs/SCE_ACCEPTED_SUBSET.md 2.15).

     A Number holds an integer exactly only up to 2^53 - 1, and a generated
     backend's int64 holds more. So every operation is computed exactly, as a
     BigInt, and handed back as a Number only where the result fits the
     declared type AND is a Number's exact integer. Anything else throws: the
     script engine's failure channel is the one a generated backend records,
     so a document that raises error.execution there raises it here.

     The library is embedded in one attribute and its newlines collapse, so it
     holds no line comment and no statement that relies on a line break. */
  var SAFE = BigInt(Number.MAX_SAFE_INTEGER);
  var ZERO = BigInt(0);
  var ONE = BigInt(1);

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

  return {
    I8: integer(true, 8),
    I16: integer(true, 16),
    I32: integer(true, 32),
    I64: integer(true, 64),
    U8: integer(false, 8),
    U16: integer(false, 16),
    U32: integer(false, 32),
    U64: integer(false, 64),
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
