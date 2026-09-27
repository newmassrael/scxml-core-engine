<?php
// Expected values for sce:std/http/precondition, computed by running
// sabre/dav's own Sabre\DAV\Server::checkPreconditions (BSD-3-Clause),
// unmodified, on real requests against a node with a known ETag and
// modification time.
//
// The document takes each header as a code the host computes (see its
// header). This script is that host: it turns each request's headers into
// the codes by RFC 9110 comparison — strong for If-Match, weak for
// If-None-Match (§8.8.3.2) — and asks the reference for its outcome.
// Where the reference agrees with RFC 9110 §13.2.2 the expected value is
// the reference's. Where it does not, the case is a "deviation", the
// expected value is §13.2.2's, and the note names which of the reference's
// known departures caused it; one it cannot name is marked UNEXPLAINED.
//
// Usage (see README.md):
//   docker run --rm -v <sabre-dav-checkout>:/sabre:ro -v <vendor dir>:/vendor:ro \
//     -v $PWD/tools/reference-oracles:/oracle:ro php:8.4-cli-alpine php /oracle/sabre_precondition.php

require '/vendor/vendor/autoload.php';
spl_autoload_register(function (string $class): void {
    if (strncmp($class, 'Sabre\\DAV', 9) === 0) {
        $file = '/sabre/lib/' . str_replace('\\', '/', substr($class, 6)) . '.php';
        if (is_file($file)) {
            require $file;
        }
    }
}, true, true);

const ETAG = '"abc"';
const MTIME = 1_700_000_000;

class KnownFile extends Sabre\DAV\File
{
    public function getName() { return 'item'; }
    public function getETag() { return ETAG; }
    public function getLastModified() { return MTIME; }
    public function get() { return 'body'; }
}

function outcome(bool $exists, string $method, array $headers): int
{
    $children = $exists ? [new KnownFile()] : [];
    $server = new Sabre\DAV\Server(new Sabre\DAV\SimpleCollection('root', $children));
    $request = new Sabre\HTTP\Request($method, '/item', $headers);
    $response = new Sabre\HTTP\Response();
    $server->httpRequest = $request;
    $server->httpResponse = $response;
    try {
        return $server->checkPreconditions($request, $response) ? 0 : $response->getStatus();
    } catch (Sabre\DAV\Exception\PreconditionFailed $e) {
        return 412;
    }
}

function opaque(string $tag): string { return preg_replace('/^W\//', '', $tag); }

// The host's side: headers to the document's codes (RFC 9110 §8.8.3.2, §13.1).
function codes(bool $exists, array $h): array
{
    $list = fn ($v) => array_map('trim', explode(',', $v));
    $ifMatch = 0;
    if (isset($h['If-Match'])) {
        $ifMatch = trim($h['If-Match']) === '*' ? 1
            : (in_array(ETAG, $list($h['If-Match']), true) && $exists ? 2 : 3);
    }
    $ifNoneMatch = 0;
    if (isset($h['If-None-Match'])) {
        $ifNoneMatch = trim($h['If-None-Match']) === '*' ? 1
            : ($exists && in_array(opaque(ETAG), array_map('opaque', $list($h['If-None-Match'])), true) ? 2 : 3);
    }
    $date = fn ($v) => strtotime($v);
    $ius = isset($h['If-Unmodified-Since']) && $exists ? (MTIME > $date($h['If-Unmodified-Since']) ? 2 : 1) : 0;
    $ims = isset($h['If-Modified-Since']) && $exists ? (MTIME > $date($h['If-Modified-Since']) ? 1 : 2) : 0;
    return [$ifMatch, $ius, $ifNoneMatch, $ims];
}

// RFC 9110 §13.2.2, the document's definition.
function rfc(bool $exists, bool $readOnly, array $c): int
{
    [$im, $ius, $inm, $ims] = $c;
    if ($im !== 0) {
        if (!$exists || $im === 3) return 412;
    } elseif ($ius === 2) {
        return 412;
    }
    if ($inm !== 0) {
        if (($inm === 1 && $exists) || $inm === 2) return $readOnly ? 304 : 412;
    } elseif ($readOnly && $ims === 2) {
        return 304;
    }
    return 0;
}

function why(string $method, array $h, array $c, int $ref, int $want): string
{
    $reasons = [];
    if (isset($h['If-Unmodified-Since']) && (isset($h['If-Match']) || isset($h['If-None-Match']) || isset($h['If-Modified-Since']))) {
        $reasons[] = 'If-Unmodified-Since evaluated last and beside If-Match';
    }
    if (isset($h['If-None-Match']) && str_contains($h['If-None-Match'], 'W/')) {
        $reasons[] = 'If-None-Match compared strongly';
    }
    if ($method === 'HEAD' && $c[2] !== 0) {
        $reasons[] = 'If-None-Match 304 for GET only';
    }
    if (!in_array($method, ['GET', 'HEAD'], true) && isset($h['If-Modified-Since'])) {
        $reasons[] = 'If-Modified-Since applied to every method';
    }
    return $reasons ? implode('; ', $reasons) : 'UNEXPLAINED';
}

$past = gmdate('D, d M Y H:i:s \G\M\T', MTIME - 3600);
$future = gmdate('D, d M Y H:i:s \G\M\T', MTIME + 3600);
$headerSets = [
    [],
    ['If-Match' => ETAG], ['If-Match' => '"x"'], ['If-Match' => '*'], ['If-Match' => '"x", ' . ETAG],
    ['If-None-Match' => ETAG], ['If-None-Match' => '"x"'], ['If-None-Match' => '*'], ['If-None-Match' => 'W/' . ETAG],
    ['If-Modified-Since' => $past], ['If-Modified-Since' => $future],
    ['If-Unmodified-Since' => $past], ['If-Unmodified-Since' => $future],
    ['If-Match' => ETAG, 'If-Unmodified-Since' => $past],
    ['If-None-Match' => ETAG, 'If-Unmodified-Since' => $past],
    ['If-None-Match' => '"x"', 'If-Modified-Since' => $future],
];

$cases = [];
foreach ([true, false] as $exists) {
    foreach (['GET', 'HEAD', 'PUT'] as $method) {
        foreach ($headerSets as $h) {
            if (!$exists && (isset($h['If-Modified-Since']) || isset($h['If-Unmodified-Since']))) {
                continue; // a date has nothing to compare with; the host passes 0
            }
            $c = codes($exists, $h);
            $readOnly = in_array($method, ['GET', 'HEAD'], true);
            $ref = outcome($exists, $method, $h);
            $want = rfc($exists, $readOnly, $c);
            $label = sprintf('%s %s %s', $method, $exists ? 'existing' : 'missing', $h ? json_encode($h) : 'no conditions');
            $cases[] = [
                'args' => [$exists, $readOnly, ...$c],
                'expected' => $want,
                'note' => $ref === $want ? "$label — reference $ref"
                    : "deviation: $label — reference $ref: " . why($method, $h, $c, $ref, $want),
            ];
        }
    }
}
echo json_encode(['reference' => 'sabre/dav DAV/Server.php checkPreconditions', 'precondition' => $cases], JSON_PRETTY_PRINT), "\n";
