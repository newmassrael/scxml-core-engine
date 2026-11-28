---- MODULE ValueService ----
(***************************************************************************)
(* What a client of a server may rely on of the published values it        *)
(* subscribes to (docs/adr/0115, D1; the promise of docs/adr/0152, D10).   *)
(* No changelog, cursor, row, marker, floor, page, unit, queue, signature, *)
(* crash or message is in it.                                              *)
(*                                                                         *)
(* A session publishes the state of its values at versions, each above     *)
(* every version handed out before it, in a generation. When what holds    *)
(* the values is started again, they are published in a new generation,    *)
(* and a version of one generation says nothing of another. What the       *)
(* session is asked to publish (pending) it publishes by a step of its     *)
(* own. A version at which the state did not change is a version of the    *)
(* session, not a change of its values (Changed).                          *)
(*                                                                         *)
(* A client subscribes to the session's values under its incarnation, and  *)
(* its subscription (sub) ends with that incarnation. What it holds of     *)
(* each value (holds) is a generation, a version and the value, or nothing *)
(* (it holds none, or is reading the value again). Exact: what a client    *)
(* holds of a value at a version is that value as published at that        *)
(* version. Monotone: within one generation a client's version of a value  *)
(* never goes back. OneState: the values of the session a client holds are *)
(* of one version of it. Silent: a client's values move only to complete   *)
(* one it did not hold, or over versions at which the session's values     *)
(* changed; so a change that leaves them as they were gives the client     *)
(* nothing. Authorized: a client is given a value only under a             *)
(* subscription its current incarnation made, so nothing is delivered      *)
(* after the incarnation ended; a viewer the audience no longer admits is  *)
(* one whose incarnation ended (docs/adr/0152, D9). Spec has no step that  *)
(* breaks one of these, so a protocol that refines it keeps each.          *)
(*                                                                         *)
(* Silent is stated per interval and per session. A version a client moves *)
(* to may be one at which nothing changed (the version a response read),   *)
(* and a value moves with the session's other values to keep OneState;     *)
(* what it says is that something of the session changed since the version *)
(* the client held, or a value was completed.                              *)
(*                                                                         *)
(* Converges: the session publishes what it is asked, whatever any client  *)
(* does; and for every version at which the values changed while Fair is   *)
(* subscribed, Fair comes to hold every value at that version or a later   *)
(* one, or its subscription ends. A version that changed nothing is never  *)
(* given (Silent), so it is not one Fair is owed. It is progress: it holds *)
(* of a protocol under the fairness that protocol gives Fair and its own   *)
(* steps, and asks nothing of any other client. The promise names no step, *)
(* so its Spec states no fairness. D10 states it as long as each reset     *)
(* completes within tombstone_retention_ms and the removals of that time   *)
(* are at most tombstones; a run in which the session publishes finitely   *)
(* often meets both.                                                       *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Nothing: it is what a user may rely on, and names no mechanism.       *)
(***************************************************************************)
EXTENDS Integers, FiniteSets

CONSTANTS
    \* the clients, and the one Converges is stated of
    \* @type: Set(Int);
    Clients,
    \* @type: Int;
    Fair,
    \* the session's values are 1..NValues, each of rows by the keys 1..NKeys, a row present (1)
    \* or not (0)
    \* @type: Int;
    NValues,
    \* @type: Int;
    NKeys,
    \* the versions handed out are 0..MaxVersion, and the generations 0..MaxGen
    \* @type: Int;
    MaxVersion,
    \* @type: Int;
    MaxGen,
    \* the incarnations of a client are 1..MaxInc
    \* @type: Int;
    MaxInc

ASSUME Fair \in Clients

VARIABLES
    \* the generation the values are published in now
    \* @type: Int;
    gen,
    \* pub[<<g, v>>]: the state of the session's values published at version v of generation g,
    \* or Unset
    \* @type: <<Int, Int>> -> (Int -> (Int -> Int));
    pub,
    \* what the session is asked to publish next, or Unset
    \* @type: Int -> (Int -> Int);
    pending,
    \* each client's incarnation
    \* @type: Int -> Int;
    inc,
    \* the incarnation each client's subscription was made under, or 0 while it has none
    \* @type: Int -> Int;
    sub,
    \* what each client holds of each value, or Nothing
    \* @type: Int -> (Int -> {g: Int, v: Int, val: Int -> Int});
    holds

vars == <<gen, pub, pending, inc, sub, holds>>

Values == 1..NValues
Keys == 1..NKeys
Vers == 0..MaxVersion
Gens == 0..MaxGen

\* A value: whether each key has a row. A state: each value of the session.
Rows == [Keys -> 0..1]
States == [Values -> Rows]
EmptyRows == [k \in Keys |-> 0]
Empty == [a \in Values |-> EmptyRows]
NoRows == [k \in Keys |-> -1]
Unset == [a \in Values |-> NoRows]

\* What a client may hold of a value, and what it holds when it holds nothing.
Holdings == [g : Gens, v : Vers, val : Rows]
Nothing == [g |-> -1, v |-> -1, val |-> NoRows]

\* @type: (Set(Int)) => Int;
Max(S) == CHOOSE x \in S : \A y \in S : y <= x

\* @type: (Int, Int) => Bool;
Published(g, v) == pub[<<g, v>>] # Unset

\* The session's values changed at version v of generation g: v is the generation's first
\* version, or the state differs from the one published at the version before it.
\* @type: (Int, Int) => Bool;
Changed(g, v) ==
    /\ Published(g, v)
    /\ LET earlier == {u \in Vers : u < v /\ Published(g, u)} IN
       \/ earlier = {}
       \/ pub[<<g, v>>] # pub[<<g, Max(earlier)>>]

\* The newest version handed out, of any generation.
Newest == Max({v \in Vers : \E g \in Gens : Published(g, v)})

\* What a client holding o of each value may come to hold, n, as each property says.
\* @type: (Int -> {g: Int, v: Int, val: Int -> Int}) => Bool;
OneStateOf(n) ==
    \A a, b \in Values :
        (n[a] # Nothing /\ n[b] # Nothing) => (n[a].g = n[b].g /\ n[a].v = n[b].v)

\* @type: (Int -> {g: Int, v: Int, val: Int -> Int}, Int -> {g: Int, v: Int, val: Int -> Int}) => Bool;
SilentOf(o, n) ==
    (\E a \in Values : n[a] # Nothing /\ n[a] # o[a])
        => \/ \E a \in Values : n[a] # Nothing /\ (o[a] = Nothing \/ o[a].g # n[a].g)
           \/ \E a \in Values :
                 /\ o[a] # Nothing
                 /\ n[a] # Nothing
                 /\ o[a].g = n[a].g
                 /\ \E u \in Vers : o[a].v < u /\ u <= n[a].v /\ Changed(n[a].g, u)

\* @type: (Int, Int -> {g: Int, v: Int, val: Int -> Int}, Int -> {g: Int, v: Int, val: Int -> Int}) => Bool;
Allowed(c, o, n) ==
    /\ (\E a \in Values : n[a] # Nothing /\ n[a] # o[a]) => sub[c] = inc[c]
    /\ \A a \in Values : n[a] # Nothing => pub[<<n[a].g, n[a].v>>][a] = n[a].val
    /\ \A a \in Values :
          (o[a] # Nothing /\ n[a] # Nothing /\ o[a].g = n[a].g) => n[a].v >= o[a].v
    /\ OneStateOf(n)
    /\ SilentOf(o, n)

TypeOK ==
    /\ gen \in Gens
    /\ pub \in [Gens \X Vers -> States \cup {Unset}]
    /\ pending \in States \cup {Unset}
    /\ inc \in [Clients -> 1..MaxInc]
    /\ sub \in [Clients -> 0..MaxInc]
    /\ holds \in [Clients -> [Values -> Holdings \cup {Nothing}]]

Init ==
    /\ gen = 0
    /\ pub = [p \in Gens \X Vers |-> IF p = <<0, 0>> THEN Empty ELSE Unset]
    /\ pending = Unset
    /\ inc = [c \in Clients |-> 1]
    /\ sub = [c \in Clients |-> 0]
    /\ holds = [c \in Clients |-> [a \in Values |-> Nothing]]

-----------------------------------------------------------------------------
\* The session is asked to publish a state.
Ask(x) ==
    /\ pending = Unset
    /\ pending' = x
    /\ UNCHANGED <<gen, pub, inc, sub, holds>>

\* It publishes what it was asked at a version above every one handed out.
Publish(v) ==
    /\ pending # Unset
    /\ v > Newest
    /\ pub' = [pub EXCEPT ![<<gen, v>>] = pending]
    /\ pending' = Unset
    /\ UNCHANGED <<gen, inc, sub, holds>>

\* What holds the values is started again: they are published in a new generation, and what
\* the session was asked is dropped with what held them.
Begin(x, v) ==
    /\ gen < MaxGen
    /\ v > Newest
    /\ gen' = gen + 1
    /\ pub' = [pub EXCEPT ![<<gen + 1, v>>] = x]
    /\ pending' = Unset
    /\ UNCHANGED <<inc, sub, holds>>

\* A client subscribes under its incarnation.
Subscribe(c) ==
    /\ sub[c] = 0
    /\ sub' = [sub EXCEPT ![c] = inc[c]]
    /\ UNCHANGED <<gen, pub, pending, inc, holds>>

\* A client's incarnation ends, and its subscription with it; the next holds nothing.
End(c) ==
    /\ inc[c] < MaxInc
    /\ inc' = [inc EXCEPT ![c] = @ + 1]
    /\ sub' = [sub EXCEPT ![c] = 0]
    /\ holds' = [holds EXCEPT ![c] = [a \in Values |-> Nothing]]
    /\ UNCHANGED <<gen, pub, pending>>

\* What a client holds moves: values given to it, or dropped to nothing.
Hold(c) ==
    /\ holds'[c] \in [Values -> Holdings \cup {Nothing}]
    /\ \A d \in Clients \ {c} : holds'[d] = holds[d]
    /\ Allowed(c, holds[c], holds'[c])
    /\ UNCHANGED <<gen, pub, pending, inc, sub>>

Next ==
    \/ \E x \in States : Ask(x)
    \/ \E v \in Vers : Publish(v)
    \/ \E x \in States, v \in Vers : Begin(x, v)
    \/ \E c \in Clients : Subscribe(c) \/ End(c) \/ Hold(c)

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* What a client holds of a value at a version is that value as published at that version.
Exact ==
    \A c \in Clients, a \in Values :
        holds[c][a] # Nothing => pub[<<holds[c][a].g, holds[c][a].v>>][a] = holds[c][a].val

\* The values of the session a client holds are of one version of it.
OneState == \A c \in Clients : OneStateOf(holds[c])

\* Within one generation a client's version of a value never goes back.
Monotone ==
    [][\A c \in Clients, a \in Values :
          (holds[c][a] # Nothing /\ holds'[c][a] # Nothing /\ holds'[c][a].g = holds[c][a].g)
              => holds'[c][a].v >= holds[c][a].v]_vars

\* A client's values move only to complete one, or over versions at which the session changed.
Silent == [][\A c \in Clients : SilentOf(holds[c], holds'[c])]_vars

\* A client is given a value only under a subscription its current incarnation made.
Authorized ==
    [][\A c \in Clients :
          (\E a \in Values : holds'[c][a] # Nothing /\ holds'[c][a] # holds[c][a])
              => sub[c] = inc[c]]_vars

\* The session publishes what it is asked; and every version at which the session's values
\* changed while Fair is subscribed comes to be held by Fair, of every value, or Fair's
\* subscription ends.
Converges ==
    /\ pending # Unset ~> pending = Unset
    /\ \A v \in Vers :
          ((\E g \in Gens : Changed(g, v)) /\ sub[Fair] = inc[Fair])
              ~> (\/ sub[Fair] # inc[Fair]
                  \/ \A a \in Values : holds[Fair][a] # Nothing /\ holds[Fair][a].v >= v)
====
