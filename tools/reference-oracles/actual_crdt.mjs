// Expected values for sce:std/hash/murmur3_32 and sce:std/merge/merkle_*,
// computed by running Actual Budget's own crdt package (MIT) — the reference
// the documents are proven against. Nothing here re-implements the reference:
// every expected value inside the reference's domain comes from its code.
//
// Outside that domain the reference is wrong and the documents fix it
// (SCE_FORGE.md §4.12 stdlib notes). Those cases are marked "deviation", and
// for each the script also records what the reference does there, so the
// fix is evidenced rather than asserted.
//
// Usage (see README.md for the pinned reference and its dependencies):
//   node tools/reference-oracles/actual_crdt.mjs <actual-checkout> > cases.json

const root = process.argv[2];
if (!root) {
  console.error('usage: actual_crdt.mjs <path to actualbudget/actual checkout>');
  process.exit(2);
}
const crdt = `${root}/packages/crdt/src/crdt/`;
const { Timestamp } = await import(crdt + 'timestamp.ts');
const merkle = await import(crdt + 'merkle.ts');
const { createRequire } = await import('node:module');
const murmurhash = createRequire(crdt + 'timestamp.ts')('murmurhash');

// Deterministic input: a fixed-seed xorshift32, so the file regenerates
// byte-identically. Each group of cases starts from its own seed, so
// changing one group's cases leaves every other group's as it was.
let state;
function seed(s) { state = s >>> 0; }
function rand() {
  state ^= state << 13; state >>>= 0;
  state ^= state >>> 17;
  state ^= state << 5; state >>>= 0;
  return state;
}

// ---- murmur3_32 -----------------------------------------------------------

function murmurCases() {
  seed(0x9e3779b9);
  const cases = [];
  const text = s => [...new TextEncoder().encode(s)];
  const fixed = [
    ['', 0], ['', 1], ['', 0xffffffff], ['a', 0], ['ab', 0], ['abc', 0],
    ['abcd', 0], ['abcde', 0], ['hello', 0], ['hello', 0x9747b28c],
    ['The quick brown fox jumps over the lazy dog', 0],
    ['2026-09-27T10:00:00.000Z-0000-0000000000000001', 0],
  ];
  for (const [s, seed] of fixed) {
    const data = text(s);
    cases.push({
      args: [data, seed],
      expected: murmurhash.v3(new Uint8Array(data), seed),
      note: `${JSON.stringify(s)} seed ${seed}`,
    });
  }
  for (let len = 0; len < 12; len++) {
    const data = Array.from({ length: len }, () => rand() & 0xff);
    const seed = rand();
    cases.push({
      args: [data, seed],
      expected: murmurhash.v3(new Uint8Array(data), seed),
      note: `${len} pseudo-random bytes (tail of ${len & 3})`,
    });
  }
  return cases;
}

// ---- merkle ---------------------------------------------------------------

// The reference's nested trie as the documents' digest: one node per trie
// node, its base-3 path read as the prefix, ascending (depth, prefix). The
// reference's hash is a signed 32-bit int (JS XOR); `>>> 0` is the same bits.
function flatten(trie) {
  const out = [];
  (function walk(node, path) {
    out.push({
      depth: path.length,
      prefix: path.length === 0 ? 0 : parseInt(path, 3),
      hash: (node.hash || 0) >>> 0,
    });
    for (const k of merkle.getKeys(node)) walk(node[k], path + k);
  })(trie, '');
  out.sort((a, b) => a.depth - b.depth || a.prefix - b.prefix);
  return out;
}

const MINUTE = 60000;
// Inside the reference's domain: a 16-digit base-3 minute key.
const lo = 3 ** 15 * MINUTE, hi = 3 ** 16 * MINUTE;
const base = Date.UTC(2026, 8, 27, 10, 0, 0);

function stamp(ms, counter, node) {
  return new Timestamp(ms, counter, node.toString(16).padStart(16, '0'));
}
function randomStamp(spreadMinutes) {
  const ms = base - (rand() % spreadMinutes) * MINUTE + (rand() % MINUTE);
  return stamp(ms, rand() % 4, rand() % 8 + 1);
}
const minuteOf = t => Math.floor(t.millis() / MINUTE);
const hashOf = t => t.hash() >>> 0;

function insertCases() {
  seed(0x85ebca6b);
  const cases = [];
  for (const [count, spread] of [[1, 1], [2, 1], [3, 5], [3, 90], [4, 60 * 24 * 40]]) {
    let trie = merkle.emptyTrie();
    for (let i = 0; i < count - 1; i++) trie = merkle.insert(trie, randomStamp(spread));
    const t = randomStamp(spread);
    cases.push({
      args: [flatten(trie), minuteOf(t), hashOf(t)],
      expected: flatten(merkle.insert(trie, t)),
      note: `insert into a digest of ${count - 1} changes spread over ${spread} min`,
    });
  }
  return cases;
}

function pruneCases() {
  seed(0xc2b2ae35);
  const cases = [];
  // A node has at most three children in the window, so `keep` 3 drops
  // nothing there, and random minutes rarely give a node three. The minutes
  // are chosen instead: offsets from a minute whose last three base-3 digits
  // are 0, so that consecutive minutes are siblings.
  const day = 60 * 24, year = day * 365;
  const m0 = Math.floor((Math.floor(base / MINUTE) - 4 * year) / 27) * 27;
  const shapes = [
    ['three sibling minutes', [0, 1, 2], 2],
    ['siblings at the two lowest levels', [0, 1, 2, 3, 4, 5, 9, 18], 2],
    ['a spread over days, one branch kept', [0, 1, 2, 100, 5000, 50000], 1],
    ['a spread over years', [0, 1, 2, year, 2 * year, 3 * year + 7], 2],
  ];
  for (const [name, offsets, keep] of shapes) {
    let trie = merkle.emptyTrie();
    for (const off of offsets) {
      trie = merkle.insert(trie, stamp((m0 + off) * MINUTE + (rand() % MINUTE), rand() % 4, rand() % 8 + 1));
    }
    const before = flatten(trie), after = flatten(merkle.prune(trie, keep));
    // A case where nothing is dropped would not exercise the pruning at all.
    if (after.length >= before.length) throw new Error(`prune case "${name}" drops nothing`);
    cases.push({
      args: [before, keep],
      expected: after,
      note: `${name}, keep ${keep}: ${before.length} nodes to ${after.length}`,
    });
  }
  return cases;
}

function diffCases() {
  seed(0xb28f2b18);
  const cases = [];
  const shapes = [
    ['equal digests', 0, 0, false],
    ['one extra change on b', 0, 1, false],
    ['one extra change on each side', 1, 1, false],
    ['extra changes, both pruned', 2, 3, true],
    ['an old change, both pruned', 0, 1, true],
  ];
  for (const [name, extraA, extraB, prune] of shapes) {
    let common = merkle.emptyTrie();
    const spread = name.startsWith('an old') ? 60 * 24 * 60 : 60 * 24 * 3;
    for (let i = 0; i < 3; i++) common = merkle.insert(common, randomStamp(spread));
    let a = common, b = common;
    for (let i = 0; i < extraA; i++) a = merkle.insert(a, randomStamp(spread));
    for (let i = 0; i < extraB; i++) b = merkle.insert(b, randomStamp(spread));
    if (prune) { a = merkle.prune(a); b = merkle.prune(b); }
    const d = merkle.diff(a, b);
    cases.push({
      args: [flatten(a), flatten(b)],
      expected: { differs: d !== null, millis: d === null ? 0 : d },
      note: name,
    });
  }
  return cases;
}

// Outside the reference's domain. The expected value is the documents'
// definition (a node at depth d holds minute / 3^(16-d)); `reference`
// records what Actual does with the same input.
function deviationCases() {
  const cases = [];
  for (const [label, ms] of [['1990-01-01 (15-digit key)', Date.UTC(1990, 0, 1)], ['2060-01-01 (17-digit key)', Date.UTC(2060, 0, 1)]]) {
    const t = stamp(ms, 0, 1);
    const minute = minuteOf(t);
    const expected = [{ depth: 0, prefix: 0, hash: hashOf(t) }];
    for (let d = 1; d <= 16; d++) expected.push({ depth: d, prefix: Math.floor(minute / 3 ** (16 - d)), hash: hashOf(t) });
    // A second change in the same minute makes the reference's diff descend
    // to the leaf and read the minute back from the path it walked.
    let reference;
    try {
      const a = merkle.insert(merkle.emptyTrie(), t);
      const b = merkle.insert(a, stamp(ms + 1, 0, 2));
      const back = merkle.diff(a, b);
      reference = `a ${minute.toString(3).length}-digit key; diff of two digests parting at this minute answers ${back}${back === minute * MINUTE ? '' : `, not ${minute * MINUTE}`}`;
    } catch (e) {
      reference = `a ${minute.toString(3).length}-digit key; diff throws ${e.constructor.name}: ${e.message}`;
    }
    cases.push({ args: [[], minute, hashOf(t)], expected, note: `deviation: ${label} — reference: ${reference}` });
  }
  return cases;
}

// ---- hlc_within_drift -----------------------------------------------------

// The reference's recv refuses a message stamped more than maxDrift ahead of
// its wall clock (ClockDriftError). Its clock is `Date.now`, held here at a
// fixed reading so the answer is the reference's and not the machine's.
function driftCases() {
  const cases = [];
  const realNow = Date.now;
  const now = Date.UTC(2026, 8, 27, 10, 0, 0);
  const maxDrift = 5 * 60 * 1000;
  try {
    Date.now = () => now;
    for (const ahead of [-60000, 0, 1, maxDrift - 1, maxDrift, maxDrift + 1, 10 * maxDrift]) {
      Timestamp.init({ node: '1', maxDrift });
      let within = true;
      try {
        Timestamp.recv(stamp(now + ahead, 0, 2));
      } catch (e) {
        if (e.name !== 'ClockDriftError') throw e;
        within = false;
      }
      cases.push({ args: [now + ahead, now, maxDrift], expected: within, note: `a message ${ahead} ms ahead, bound ${maxDrift}` });
    }
  } finally {
    Date.now = realNow;
  }
  cases.push({ args: [0, 0, -1], fails: 'precondition', note: 'a negative bound' });
  return cases;
}

// ---- hlc_text -------------------------------------------------------------

// The reference's Timestamp.toString(), as bytes. The record's node is a
// uint64; the reference's is the text, and the node ids it issues are
// lower-case hex (makeClientId), which is the form written here.
function textCases() {
  seed(0x165667b1);
  const cases = [];
  const hlc = (wallTime, counter, nodeId) => ({ wallTime, counter, nodeId });
  const fixed = [
    [Date.UTC(2026, 8, 27, 10, 0, 0), 0, 1n],
    [0, 0, 0n],
    [Date.UTC(9999, 11, 31, 23, 59, 59, 999), 0xffff, 0xffffffffffffffffn],
    [Date.UTC(2000, 1, 29, 12, 34, 56, 789), 0x0a1b, 0x0123456789abcdefn],
  ];
  for (let i = 0; i < 4; i++) {
    fixed.push([Date.UTC(2026, 0, 1) + rand() * 1000 + (rand() % 1000), rand() & 0xffff, (BigInt(rand()) << 32n) | BigInt(rand())]);
  }
  for (const [ms, counter, node] of fixed) {
    const t = new Timestamp(ms, counter, node.toString(16).padStart(16, '0'));
    const text = t.toString();
    cases.push({
      args: [hlc(ms, counter, Number(node) === Number(node) && node <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(node) : node.toString())],
      expected: [...new TextEncoder().encode(text)],
      note: text,
    });
  }
  cases.push({ args: [hlc(Date.UTC(2026, 0, 1), 0x10000, 1)], fails: 'precondition', note: 'a counter past four hex digits — the reference refuses it at send (OverflowError)' });
  cases.push({ args: [hlc(253402300800000, 0, 1)], fails: 'precondition', note: 'the year 10000 — the reference writes "+010000-…", not 46 characters' });
  return cases;
}

// ---- lww_classify ---------------------------------------------------------

// The reference's compareMessages needs its database; its rule (index.ts,
// the loop after the query) is: the logged stamps of the cell include the
// message's → duplicate; some logged stamp is later → old; else apply. The
// order is the reference's own — its Timestamp text, compared as strings.
function lwwCases() {
  seed(0xd3a2646c);
  const cases = [];
  const rec = t => ({ wallTime: t.millis(), counter: t.counter(), nodeId: parseInt(t.node(), 16) });
  const classify = (logged, msg) => {
    const m = msg.toString(), texts = logged.map(t => t.toString());
    return texts.includes(m) ? 2 : texts.some(x => x > m) ? 1 : 0;
  };
  const t = (ms, c, n) => stamp(ms, c, n);
  const shapes = [
    ['nothing logged', [], t(base, 0, 1)],
    ['the same write again', [t(base, 0, 1), t(base + 5, 0, 1)], t(base + 5, 0, 1)],
    ['a later wall time logged', [t(base + 10, 0, 1)], t(base, 0, 2)],
    ['a later counter logged', [t(base, 3, 1)], t(base, 2, 9)],
    ['a later node logged', [t(base, 0, 9)], t(base, 0, 1)],
    ['everything logged is earlier', [t(base - 10, 0, 9), t(base, 0, 1)], t(base, 1, 1)],
  ];
  for (let i = 0; i < 4; i++) {
    const logged = Array.from({ length: 1 + (rand() % 4) }, () => t(base + (rand() % 4), rand() % 3, rand() % 3 + 1));
    shapes.push([`${logged.length} pseudo-random stamps`, logged, t(base + (rand() % 4), rand() % 3, rand() % 3 + 1)]);
  }
  for (const [name, logged, msg] of shapes) {
    cases.push({ args: [logged.map(rec), rec(msg)], expected: classify(logged, msg), note: name });
  }
  return cases;
}

console.log(JSON.stringify({
  reference: 'actualbudget/actual packages/crdt + murmurhash@2.0.1',
  window: [new Date(lo).toISOString(), new Date(hi).toISOString()],
  murmur3_32: murmurCases(),
  merkle_insert: [...insertCases(), ...deviationCases()],
  merkle_prune: pruneCases(),
  merkle_diff: diffCases(),
  hlc_within_drift: driftCases(),
  hlc_text: textCases(),
  lww_classify: lwwCases(),
}, null, 1));
