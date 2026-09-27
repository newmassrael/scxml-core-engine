<?php
// Expected values for sce:std/acl/acl_membership and acl_granted, computed
// by running sabre/dav's own Sabre\DAVACL\Plugin (BSD-3-Clause), unmodified,
// with its test suite's mock principal, node and auth backend.
//
// Principals are numbered: `principals/u<N>` is N. A privilege tree is
// flattened in the reference's own order (getFlatPrivilegeSet walks it
// pre-order) and numbered in that order, DAV:all 0. The documents answer
// with numbers; the reference's names are mapped back through the same
// tables, so every expected value that says what the reference did is the
// reference's answer.
//
// Where the reference is wrong the documents fix it; those cases say
// "deviation:", their expected value is the document's definition, and
// the note records what the reference did:
// - getPrincipalMembership lists the user among its own groups when a
//   membership cycle leads back to it;
// - a membership naming a group no principal defines aborts the lookup
//   (NotFound).
// Privileges are compared as sets: the document answers in tree order, the
// reference in the order its expansion stack left them.
//
// Usage (see README.md):
//   docker run --rm -v <sabre-dav-checkout>:/sabre:ro -v <vendor dir>:/vendor:ro \
//     -v $PWD/tools/reference-oracles:/oracle:ro php:8.4-cli-alpine php /oracle/sabre_acl.php

require '/vendor/vendor/autoload.php';
spl_autoload_register(function (string $class): void {
    if (strncmp($class, 'Sabre\\DAV', 9) === 0) {
        $file = '/sabre/lib/' . str_replace('\\', '/', substr($class, 6)) . '.php';
        if (is_file($file)) {
            require $file;
        }
    }
}, true, true);
require '/sabre/tests/Sabre/DAVACL/MockPrincipal.php';
require '/sabre/tests/Sabre/DAVACL/MockACLNode.php';
require '/sabre/tests/Sabre/DAV/Auth/Backend/Mock.php';

use Sabre\DAV;
use Sabre\DAVACL;
use Sabre\HTTP;

const ALL = 4294967295;
const AUTHENTICATED = 4294967294;
const UNAUTHENTICATED = 4294967293;
const OWNER = 4294967292;
const PSEUDO = [
    ALL => '{DAV:}all',
    AUTHENTICATED => '{DAV:}authenticated',
    UNAUTHENTICATED => '{DAV:}unauthenticated',
    OWNER => '{DAV:}owner',
];

function uri(int $n): string { return "principals/u$n"; }
function num(string $uri): int { return (int) substr($uri, strlen('principals/u')); }

/** A node with an owner and, optionally, its own supported-privilege tree. */
class Resource extends DAVACL\MockACLNode
{
    public $ownerUri;
    public $sps;
    public function getOwner() { return $this->ownerUri; }
    public function getSupportedPrivilegeSet() { return $this->sps; }
}

/**
 * A server holding the given principals with the given memberships, and
 * `node`. `$edges` is [[member, group], ...]; a group with no principal of
 * its own is left undefined, as a dangling membership is.
 */
function server(array $edges, ?int $user, ?Resource $node, array $defined): array
{
    $groups = [];
    foreach ($edges as [$m, $g]) {
        $groups[$m][] = uri($g);
    }
    $principals = [];
    foreach ($defined as $n) {
        $principals[] = new DAVACL\MockPrincipal("u$n", uri($n), $groups[$n] ?? []);
    }
    $children = [new DAV\SimpleCollection('principals', $principals)];
    if ($node) {
        $children[] = $node;
    }
    $server = new DAV\Server($children);
    $backend = new DAV\Auth\Backend\Mock();
    if ($user === null) {
        $backend->fail = true;
    } else {
        $backend->setPrincipal(uri($user));
    }
    $auth = new DAV\Auth\Plugin($backend);
    $auth->autoRequireLogin = false;
    $server->addPlugin($auth);
    $acl = new DAVACL\Plugin();
    $acl->allowUnauthenticatedAccess = true;
    $server->addPlugin($acl);
    $auth->beforeMethod(new HTTP\Request('GET', '/'), new HTTP\Response());
    return [$server, $acl];
}

/** The document's definition: breadth-first, the user never its own group. */
function membershipDefinition(array $edges, int $user): array
{
    $found = [];
    $queue = [$user];
    while ($queue) {
        $at = array_shift($queue);
        foreach ($edges as [$m, $g]) {
            if ($m === $at && $g !== $user && !in_array($g, $found, true)) {
                $found[] = $g;
                $queue[] = $g;
            }
        }
    }
    return $found;
}

function edgeRecords(array $edges): array
{
    return array_map(fn ($e) => ['member' => $e[0], 'group' => $e[1]], $edges);
}

$membership = [];
$membershipShapes = [
    ['no memberships', [], 1],
    ['one group', [[1, 10]], 1],
    ['a group of a group', [[1, 10], [10, 20]], 1],
    ['breadth before depth', [[1, 10], [1, 30], [10, 20], [30, 40]], 1],
    ['two paths to one group', [[1, 10], [1, 30], [10, 20], [30, 20]], 1],
    ['a cycle among groups', [[1, 10], [10, 20], [20, 10]], 1],
    ['a cycle back to the user', [[1, 10], [10, 20], [20, 1], [1, 30]], 1],
    ['a membership naming an undefined group', [[1, 10], [10, 99]], 1],
    ['another principal\'s groups are not the user\'s', [[1, 10], [2, 20]], 1],
];
foreach ($membershipShapes as [$note, $edges, $user]) {
    $defined = [];
    foreach ($edges as [$m, $g]) {
        $defined[$m] = $m;
        if ($g !== 99) {
            $defined[$g] = $g;
        }
    }
    $defined[$user] = $user;
    [, $acl] = server($edges, $user, null, array_values($defined));
    $definition = membershipDefinition($edges, $user);
    try {
        $got = array_map('num', $acl->getPrincipalMembership(uri($user)));
        if ($got === $definition) {
            $membership[] = ['args' => [edgeRecords($edges), $user], 'expected' => $got, 'note' => $note];
        } else {
            $membership[] = ['args' => [edgeRecords($edges), $user], 'expected' => $definition,
                'note' => "deviation: $note — the reference answered [" . implode(', ', $got) . '], the user among its own groups'];
        }
    } catch (DAV\Exception\NotFound $e) {
        $membership[] = ['args' => [edgeRecords($edges), $user], 'expected' => $definition,
            'note' => "deviation: $note — the reference threw NotFound and answered nothing"];
    }
}

// Privilege trees, as the reference nests them.
$default = null; // the node's own: the reference's default set
$deep = [
    '{DAV:}read' => ['aggregates' => [
        '{DAV:}read-acl' => [],
        '{urn:x}read-meta' => ['aggregates' => ['{urn:x}read-meta-a' => [], '{urn:x}read-meta-b' => []]],
    ]],
    '{DAV:}write' => ['abstract' => true, 'aggregates' => [
        '{DAV:}write-content' => [],
        '{DAV:}bind' => [],
    ]],
    '{urn:x}audit' => [],
];

/** The tree as the documents number it: the reference's flat set, in its order. */
function numbered(DAVACL\Plugin $acl, string $path): array
{
    $flat = $acl->getFlatPrivilegeSet($path);
    $ids = [];
    foreach ($flat as $name => $p) {
        $ids[$name] = count($ids);
    }
    $tree = [];
    foreach ($flat as $name => $p) {
        $parent = 0;
        foreach ($flat as $other => $q) {
            if (in_array($name, $q['aggregates'], true)) {
                $parent = $ids[$other];
            }
        }
        $tree[] = ['id' => $ids[$name], 'parent' => $parent];
    }
    return [$ids, $tree];
}

$granted = [];
$grantShapes = [
    // [note, tree, acl [[principal, privilege name]], user, edges, owner]
    ['an authenticated user and the default grant of read', $default, [[AUTHENTICATED, '{DAV:}read']], 7, [], null],
    ['the same grant to no user', $default, [[AUTHENTICATED, '{DAV:}read']], null, [], null],
    ['a grant to everyone of all', $default, [[ALL, '{DAV:}all']], null, [], null],
    ['a grant to the unauthenticated', $default, [[UNAUTHENTICATED, '{DAV:}read-acl']], null, [], null],
    ['the unauthenticated grant to a user', $default, [[UNAUTHENTICATED, '{DAV:}read-acl']], 7, [], null],
    ['a grant to the user by number', $default, [[7, '{DAV:}write'], [8, '{DAV:}read']], 7, [], null],
    ['a grant to a group of the user\'s group', $default, [[20, '{DAV:}write-content']], 7, [[7, 10], [10, 20]], null],
    ['a grant to the owner, who is the user', $default, [[OWNER, '{DAV:}read']], 7, [], 7],
    ['a grant to the owner, who is the user\'s group', $default, [[OWNER, '{DAV:}write']], 7, [[7, 10]], 10],
    ['a grant to the owner, who is someone else', $default, [[OWNER, '{DAV:}read']], 7, [], 8],
    ['a grant of a privilege the tree does not hold', $default, [[7, '{urn:x}nothing'], [7, '{DAV:}unlock']], 7, [], null],
    ['a deeper tree, a nested aggregate granted', $deep, [[7, '{urn:x}read-meta']], 7, [], null],
    ['a deeper tree, an abstract aggregate granted', $deep, [[7, '{DAV:}write'], [7, '{urn:x}audit']], 7, [], null],
    ['a deeper tree, overlapping grants', $deep, [[7, '{DAV:}read'], [ALL, '{urn:x}read-meta-b'], [7, '{DAV:}all']], 7, [], null],
];
foreach ($grantShapes as [$note, $sps, $entries, $user, $edges, $owner]) {
    $node = new Resource('item', array_map(fn ($e) => [
        'principal' => PSEUDO[$e[0]] ?? uri($e[0]),
        'privilege' => $e[1],
        'protected' => false,
    ], $entries));
    $node->sps = $sps;
    $node->ownerUri = $owner === null ? null : uri($owner);
    $defined = [7, 8];
    foreach ($edges as [$m, $g]) {
        $defined[] = $m;
        $defined[] = $g;
    }
    if ($owner !== null) {
        $defined[] = $owner;
    }
    [, $acl] = server($edges, $user, $node, array_values(array_unique($defined)));
    [$ids, $tree] = numbered($acl, 'item');
    $groups = $user === null ? [] : array_map('num', $acl->getPrincipalMembership(uri($user)));
    $set = $acl->getCurrentUserPrivilegeSet('item');
    $expected = [];
    foreach ($ids as $name => $id) {
        if (in_array($name, $set, true)) {
            $expected[] = $id;
        }
    }
    // A privilege the tree does not hold is given a number no tree holds.
    $aces = array_map(fn ($e) => [
        'principal' => $e[0],
        'privilege' => $ids[$e[1]] ?? 4294967295,
    ], $entries);
    $granted[] = [
        'args' => [$tree, $aces, $user ?? 0, $groups, $owner ?? 0],
        'expected' => $expected,
        'note' => $note,
    ];
}

echo json_encode([
    'reference' => 'sabre/dav DAVACL/Plugin.php getPrincipalMembership, getFlatPrivilegeSet, getCurrentUserPrivilegeSet',
    'acl_membership' => $membership,
    'acl_granted' => $granted,
], JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES), "\n";
