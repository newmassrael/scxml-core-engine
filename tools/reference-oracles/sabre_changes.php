<?php
// Expected values for sce:std/sync/changes_since, computed by running
// sabre/dav's own CalDAV PDO backend (BSD-3-Clause) —
// Sabre\CalDAV\Backend\PDO::getChangesForCalendar, unmodified — over an
// in-memory SQLite change log built from the reference's own schema
// (examples/sql/sqlite.calendars.sql).
//
// Where no item repeats among the rows the reference fetches, its answer and
// the document's are the same page, and the expected value is the
// reference's, re-ordered by token (the reference groups by operation). Where
// an item repeats, the reference merges after cutting at `limit` and drops
// the truncation mark; the document merges first. Those cases are marked
// "deviation", their expected value is the document's definition, and the
// note records what the reference answered.
//
// Usage (see README.md):
//   docker run --rm -v <sabre-dav-checkout>:/sabre:ro -v $PWD/tools/reference-oracles:/oracle:ro \
//     php:8.4-cli-alpine php /oracle/sabre_changes.php > cases.json

spl_autoload_register(function (string $class): void {
    $prefix = 'Sabre\\';
    if (strncmp($class, $prefix, strlen($prefix)) === 0) {
        $file = '/sabre/lib/' . str_replace('\\', '/', substr($class, strlen($prefix))) . '.php';
        if (is_file($file)) {
            require $file;
        }
    }
});

function backend(array $log): array
{
    $pdo = new PDO('sqlite::memory:');
    $pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
    $pdo->exec(file_get_contents('/sabre/examples/sql/sqlite.calendars.sql'));
    $current = $log ? max(array_column($log, 2)) + 1 : 1;
    $pdo->exec("INSERT INTO calendars (id, synctoken, components) VALUES (1, $current, 'VEVENT')");
    $insert = $pdo->prepare('INSERT INTO calendarchanges (uri, synctoken, calendarid, operation) VALUES (?, ?, 1, ?)');
    foreach ($log as [$item, $op, $token]) {
        $insert->execute(["item-$item.ics", $token, $op]);
    }
    return [new Sabre\CalDAV\Backend\PDO($pdo), $current];
}

// The document's answer: of the rows at or after `since`, each item's last,
// in token order, at most `limit`.
function documented(array $log, int $since, int $limit): array
{
    $out = [];
    foreach ($log as $i => [$item, $op, $token]) {
        if ($token < $since || count($out) >= $limit) {
            continue;
        }
        $superseded = false;
        foreach ($log as $j => [$other]) {
            if ($j > $i && $other === $item) {
                $superseded = true;
            }
        }
        if (!$superseded) {
            $out[] = ['item' => $item, 'op' => $op, 'token' => $token];
        }
    }
    return $out;
}

function entries(array $log): array
{
    return array_map(fn ($r) => ['item' => $r[0], 'op' => $r[1], 'token' => $r[2]], $log);
}

$scenarios = [
    // [name, log [item, op, token] ascending, since, limit]
    ['initial page of a fresh log', [[1, 1, 1], [2, 1, 2], [3, 1, 3]], 1, 10],
    ['since the middle', [[1, 1, 1], [2, 1, 2], [3, 1, 3], [2, 2, 4]], 3, 10],
    ['modified then deleted', [[1, 1, 1], [1, 2, 2], [1, 3, 3], [2, 1, 4]], 1, 10],
    ['deleted then added again', [[1, 1, 1], [1, 3, 2], [1, 1, 3]], 1, 10],
    ['a page cut at the limit', [[1, 1, 1], [2, 1, 2], [3, 1, 3], [4, 1, 4]], 1, 2],
    ['nothing new', [[1, 1, 1], [2, 1, 2]], 3, 10],
    ['a repeat inside the fetched rows', [[1, 1, 1], [2, 1, 2], [1, 2, 3], [3, 1, 4]], 1, 2],
    ['a repeat at the cut', [[1, 1, 1], [1, 2, 2], [2, 1, 3], [3, 1, 4]], 1, 1],
];

$cases = [];
foreach ($scenarios as [$name, $log, $since, $limit]) {
    [$backend, $current] = backend($log);
    $ref = $backend->getChangesForCalendar([1, 1], $since, 1, $limit);
    $tokenOf = [];
    foreach ($log as [$item, $op, $token]) {
        if ($token >= $since) {
            $tokenOf["item-$item.ics"] = [$item, $op, $token];
        }
    }
    $refEntries = [];
    foreach (['added', 'modified', 'deleted'] as $bucket) {
        foreach ($ref[$bucket] as $uri) {
            [$item, $op, $token] = $tokenOf[$uri];
            $refEntries[] = ['item' => $item, 'op' => $op, 'token' => $token];
        }
    }
    usort($refEntries, fn ($a, $b) => $a['token'] <=> $b['token']);
    $expected = documented($log, $since, $limit);
    $truncated = !empty($ref['result_truncated']);
    $note = sprintf('%s — reference: %d entries, token %d%s', $name, count($refEntries), $ref['syncToken'],
        $truncated ? ', truncated' : '');
    if ($refEntries !== $expected) {
        $note = "deviation: $note, where the document answers " . json_encode($expected);
    } elseif ($expected && $ref['syncToken'] !== end($expected)['token'] + 1) {
        $note = "deviation: $note — next token differs";
    }
    $cases[] = ['args' => [entries($log), $since, $current, 0, $limit], 'expected' => $expected, 'note' => $note];
}

// The low-water mark: the reference has none, so a token below a compacted
// log is answered from what is left. The document refuses it.
[$backend] = backend([[2, 1, 5], [3, 1, 6]]);
$ref = $backend->getChangesForCalendar([1, 1], 1, 1, 10);
$cases[] = [
    'args' => [entries([[2, 1, 5], [3, 1, 6]]), 1, 7, 5, 10],
    'fails' => 'precondition',
    'note' => 'deviation: a token below a log compacted to 5 — reference answers ' .
        count($ref['added']) . ' entries and token ' . $ref['syncToken'] . ' as if nothing before 5 had happened',
];

echo json_encode(['reference' => 'sabre/dav CalDAV/Backend/PDO.php getChangesForCalendar', 'changes_since' => $cases],
    JSON_PRETTY_PRINT), "\n";
