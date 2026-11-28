---- MODULE Subscriptions ----
(***************************************************************************)
(* The subscription of docs/adr/0152 (D2 to D5, D7 and D9), the mechanism, *)
(* refining ValueService.                                                  *)
(*                                                                         *)
(* One session holds NValues values, each rows by key (present or not). A  *)
(* write of the session takes the next version of the store's counter and  *)
(* writes, with the session's state, each value's changelog: each row the  *)
(* write changed with that version, and each key it removed as a marker    *)
(* with the version of its removal. A marker is dropped when its time has  *)
(* passed (Lapse, tombstone_retention_ms: at any moment, as the model has  *)
(* no clock) or, the oldest first, past Tombstones markers of the value;   *)
(* the value's floor rises to the newest version dropped. Nothing is       *)
(* written for bytes that did not change. A changelog started again (the   *)
(* session removed and started again) draws a new generation, holds its    *)
(* first state at its first version, and has dropped no marker.            *)
(*                                                                         *)
(* A client holds of each value a cursor: none; whole at a version (g, c); *)
(* or reading a reset (g, f, k), its first page read at f and its next     *)
(* page after key k. It keeps up to Exchanges exchanges in flight; each    *)
(* request names every cursor and subscribes, and the server's record of   *)
(* the subscription names the incarnation that made it and ends with it.   *)
(* The server answers when something is owed, reading the session at one   *)
(* version (Now): to a whole cursor of its generation at or above the      *)
(* floor, the rows and markers above it; to a reset cursor of its          *)
(* generation whose f is at or above the floor, the next page; to anything *)
(* else, a reset's first page. A page is one key. A page that is not the   *)
(* last is carried on its own; anything else owed is one unit of the       *)
(* session: the change of each value held whole from its version, and the  *)
(* last page of each value completing a reset with the change since its f, *)
(* all brought to Now. The client takes a unit only when each value it     *)
(* moves is held whole at the unit's from and below or at its to, a page   *)
(* only when it continues the cursor it holds (a first page when that      *)
(* cursor is the one its request named), and keeps what it holds and its   *)
(* cursors in one stored write. What it holds whole is what it presents.   *)
(*                                                                         *)
(* The server crashes. A client subscribes, goes absent, reads slowly      *)
(* (takes an answer late), stops, loses its store and ends its             *)
(* incarnation. The network loses, repeats (MaxRepeat copies), delays and  *)
(* reorders exchanges, and an intermediary changes what is not signed.     *)
(*                                                                         *)
(* The parts, each a constant that is TRUE in the protocol, and each taken *)
(* out by a run that must find what docs/adr/0152, D10 says it breaks:     *)
(*   Markers             a removed key is kept as a marker                 *)
(*   FloorCheck          a cursor whose version is below the floor is owed *)
(*                       a reset                                           *)
(*   WithState           the changelog is written by the write that keeps  *)
(*                       the session's state; without it, it is the        *)
(*                       server's memory, ahead of that write              *)
(*   Generation          a changelog started again draws a new generation  *)
(*   Digest              the request's digest over the cursors and the     *)
(*                       response's content-digest;req; without them, an   *)
(*                       intermediary moves a whole cursor forward         *)
(*   ResetCursor         a client reading a reset names (g, f, k); without *)
(*                       it, it names none                                 *)
(*   CatchUp             the unit that completes a reset carries the       *)
(*                       change since its first page                       *)
(*   UnitRule            the client's rule for a unit                      *)
(*   PageRule            the client's rule for a page                      *)
(*   OneUnit             a session's changes are one unit; without it, a   *)
(*                       value with no row changed is not moved            *)
(*   OneWrite            the client keeps what it holds and its cursors in *)
(*                       one stored write; without it, the cursors first   *)
(*   FromChangelog       what is owed is read from the changelog the       *)
(*                       session writes once; without it, each write that  *)
(*                       changes a value puts its version in a queue of    *)
(*                       each subscription, bounded by QueueBound and let  *)
(*                       go as the client's cursors pass it, and a full    *)
(*                       queue refuses the write, as a full outbox refuses *)
(*                       a send (docs/adr/0109, D10)                       *)
(*   SkipUnchanged       nothing is written for bytes that did not change  *)
(*   EndWithIncarnation  the subscriptions end with their incarnation      *)
(*                                                                         *)
(* pub is a history variable: the state published at each version of each  *)
(* generation, which no server keeps and the promise is stated by.         *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   A write of the session store is atomic and durable, and its versions  *)
(*   come from one counter that only rises (SessionStore).                 *)
(*   A generation is drawn once and never twice.                           *)
(*   The membership answers no request of an incarnation it does not hold: *)
(*   an incarnation's exchanges end with it (docs/adr/0113).               *)
(*   A client knows which of its requests an answer is to, and what that   *)
(*   request named. What a party signed cannot be changed; what is not     *)
(*   signed can (Digest).                                                  *)
(*   What the session is asked to publish (pending) is its environment's,  *)
(*   which asks again what a crash lost.                                   *)
(*   A crash loses nothing the server answers from but what WithState      *)
(*   taken out puts in its memory: the due set is read again from the      *)
(*   store, and the exchanges it held are losses of the network.           *)
(*   The session publishes finitely often in every run (MaxVersion), which *)
(*   meets the condition D10 states Converges under: each reset meets      *)
(*   finitely many floors.                                                 *)
(*   LiveSpec: the session writes what it is asked and keeps it; the       *)
(*   server answers what is owed to Fair, and ends a held exchange of      *)
(*   Fair's that is owed nothing; Fair keeps exchanging, takes in what     *)
(*   reaches it and stores it; an exchange sent again and again is         *)
(*   answered. Nothing is asked of any other client.                       *)
(***************************************************************************)
EXTENDS Integers, FiniteSets

CONSTANTS
    \* the clients, and the one LiveSpec is fair to
    \* @type: Set(Int);
    Clients,
    \* @type: Int;
    Fair,
    \* the session's values 1..NValues, each of rows by the keys 1..NKeys; the versions
    \* 0..MaxVersion; the changelogs started again; each client's incarnations 1..MaxInc
    \* @type: Int;
    NValues,
    \* @type: Int;
    NKeys,
    \* @type: Int;
    MaxVersion,
    \* @type: Int;
    MaxGen,
    \* @type: Int;
    MaxInc,
    \* the removal markers one value's changelog keeps (tombstones, docs/adr/0152, D8)
    \* @type: Int;
    Tombstones,
    \* the versions a queue of one subscription holds, where FromChangelog is taken out
    \* @type: Int;
    QueueBound,
    \* the exchanges a client keeps in flight (two, SCE_MESH.md section 22.3)
    \* @type: Int;
    Exchanges,
    \* the answers the network delivers again, in all
    \* @type: Int;
    MaxRepeat,
    \* how many times each client stops, and loses its store
    \* @type: Int;
    MaxStop,
    \* @type: Int;
    MaxLose,
    \* the parts
    \* @type: Bool;
    Markers,
    \* @type: Bool;
    FloorCheck,
    \* @type: Bool;
    WithState,
    \* @type: Bool;
    Generation,
    \* @type: Bool;
    Digest,
    \* @type: Bool;
    ResetCursor,
    \* @type: Bool;
    CatchUp,
    \* @type: Bool;
    UnitRule,
    \* @type: Bool;
    PageRule,
    \* @type: Bool;
    OneUnit,
    \* @type: Bool;
    OneWrite,
    \* @type: Bool;
    FromChangelog,
    \* @type: Bool;
    SkipUnchanged,
    \* @type: Bool;
    EndWithIncarnation

ASSUME
    /\ Fair \in Clients
    /\ NValues >= 1 /\ NKeys >= 1 /\ MaxVersion >= 0 /\ MaxGen >= 0 /\ MaxInc >= 1
    /\ Tombstones >= 0 /\ QueueBound >= 1 /\ Exchanges >= 1
    /\ MaxRepeat >= 0 /\ MaxStop >= 0 /\ MaxLose >= 0

VARIABLES
    \* the session: what it is asked to publish (or Unset), the store's counter, its durable
    \* state and version, whether a run's write is owed (only where WithState is taken out), how
    \* many times its changelogs were started again, and their generation
    \* @type: Int -> (Int -> Int);
    pending,
    \* @type: Int;
    counter,
    \* @type: {st: Int -> (Int -> Int), ver: Int};
    kept,
    \* @type: Bool;
    ran,
    \* @type: Int;
    starts,
    \* @type: Int;
    gen,
    \* each value's changelog the server answers from: each key's row or marker with the version
    \* it last changed at (val 0 and ver 0: nothing kept), and the floor
    \* @type: Int -> {rows: Int -> {val: Int, ver: Int}, floor: Int};
    log,
    \* the server's record of each client's subscription: the incarnation that made it, or 0
    \* @type: Int -> Int;
    rec,
    \* the versions owed in each subscription's queue (only where FromChangelog is taken out)
    \* @type: Int -> Set(Int);
    owed,
    \* each client: its incarnation, what its core holds of each value with its cursor, the
    \* values its store holds, and how many times it stopped and lost its store
    \* @type: Int -> Int;
    inc,
    \* @type: Int -> (Int -> {mode: Str, g: Int, v: Int, k: Int, val: Int -> Int});
    mem,
    \* @type: Int -> (Int -> (Int -> Int));
    disk,
    \* @type: Int -> Int;
    stops,
    \* @type: Int -> Int;
    losses,
    \* each client's exchanges in flight: the cursors it named, those the server read, and the
    \* answer (kind "wait" until there is one); and the answers delivered again
    \* @type: Int -> Set({sent: Int -> {mode: Str, g: Int, v: Int, k: Int}, cur: Int -> {mode: Str, g: Int, v: Int, k: Int}, resp: {kind: Str, a: Int, g: Int, to: Int, f: Int, k: Int, row: Int, parts: Int -> {kind: Str, from: Int, rows: Int -> Int}}});
    ex,
    \* @type: Int;
    repeats,
    \* the history variable: the state published at each version of each generation
    \* @type: <<Int, Int>> -> (Int -> (Int -> Int));
    pub

sessionVars == <<pending, counter, kept, ran, starts, gen, log, pub>>
serverVars == <<rec, owed>>
clientVars == <<inc, mem, disk, stops, losses>>
netVars == <<ex, repeats>>
vars == <<sessionVars, serverVars, clientVars, netVars>>

\* What a client presents of each value: what it holds whole, and nothing of a value it is
\* reading again (docs/adr/0152, D7).
Presented(c) ==
    [a \in 1..NValues |->
        IF mem[c][a].mode = "whole"
          THEN [g |-> mem[c][a].g, v |-> mem[c][a].v, val |-> mem[c][a].val]
          ELSE [g |-> -1, v |-> -1, val |-> [k \in 1..NKeys |-> -1]]]

\* What the protocol implements (docs/adr/0115, D2): a client's subscription is the server's
\* record of it, and what a client holds is what it presents.
Promise == INSTANCE ValueService
    WITH sub <- rec, holds <- [c \in Clients |-> Presented(c)]

Values == Promise!Values
Keys == Promise!Keys
Vers == Promise!Vers
Gens == Promise!Gens
Rows == Promise!Rows
States == Promise!States
EmptyRows == Promise!EmptyRows
Empty == Promise!Empty
NoRows == Promise!NoRows
Unset == Promise!Unset

\* @type: (Int, Int) => Int;
Higher(a, b) == IF a < b THEN b ELSE a

NoRow == [val |-> 0, ver |-> 0]
Modes == {"none", "whole", "reset"}
NoMem == [mode |-> "none", g |-> 0, v |-> 0, k |-> 0, val |-> EmptyRows]
NoCursor == [mode |-> "none", g |-> 0, v |-> 0, k |-> 0]
NoPart == [kind |-> "none", from |-> 0, rows |-> NoRows]
Wait ==
    [kind |-> "wait", a |-> 0, g |-> 0, to |-> 0, f |-> 0, k |-> 0, row |-> 0,
     parts |-> [a \in Values |-> NoPart]]

Cursors == [mode : Modes, g : Gens, v : Vers, k : 0..NKeys]
Parts == [kind : {"none", "change", "last"}, from : Vers, rows : [Keys -> -1..1]]
Responses ==
    [kind : {"wait", "page", "unit"}, a : 0..NValues, g : Gens, to : Vers, f : Vers,
     k : 0..NKeys, row : 0..1, parts : [Values -> Parts]]
Exchange == [sent : [Values -> Cursors], cur : [Values -> Cursors], resp : Responses]
Mems == [mode : Modes, g : Gens, v : Vers, k : 0..NKeys, val : Rows]
Logs == [rows : [Keys -> [val : 0..1, ver : Vers]], floor : Vers]

TypeOK ==
    /\ pending \in States \cup {Unset}
    /\ counter \in Vers
    /\ kept \in [st : States, ver : Vers]
    /\ ran \in BOOLEAN
    /\ starts \in 0..MaxGen
    /\ gen \in Gens
    /\ log \in [Values -> Logs]
    /\ rec \in [Clients -> 0..MaxInc]
    /\ owed \in [Clients -> SUBSET Vers]
    /\ inc \in [Clients -> 1..MaxInc]
    /\ mem \in [Clients -> [Values -> Mems]]
    /\ disk \in [Clients -> States]
    /\ stops \in [Clients -> 0..MaxStop]
    /\ losses \in [Clients -> 0..MaxLose]
    /\ \A c \in Clients : ex[c] \subseteq Exchange /\ Cardinality(ex[c]) <= Exchanges
    /\ repeats \in 0..MaxRepeat
    /\ pub \in [Gens \X Vers -> States \cup {Unset}]

-----------------------------------------------------------------------------
\* The value a changelog holds.
\* @type: ({rows: Int -> {val: Int, ver: Int}, floor: Int}) => (Int -> Int);
Cur(L) == [k \in Keys |-> L.rows[k].val]

\* A value's changelog once the session's write of x at version v is in it: each row x changed
\* with v, each key it removed as a marker with v, and the oldest markers past Tombstones
\* dropped, the floor raised to the newest version dropped.
Wrote(L, x, v) ==
    LET changed(k) == x[k] # L.rows[k].val
        stamped(k) == IF SkipUnchanged THEN changed(k) ELSE changed(k) \/ x[k] = 1
        written ==
            [k \in Keys |->
                IF ~stamped(k) THEN L.rows[k]
                ELSE IF x[k] = 1 THEN [val |-> 1, ver |-> v]
                ELSE IF Markers THEN [val |-> 0, ver |-> v]
                ELSE NoRow]
        marks == {k \in Keys : written[k].val = 0 /\ written[k].ver > 0}
        older(j, k) ==
            \/ written[j].ver < written[k].ver
            \/ written[j].ver = written[k].ver /\ j < k
        dropped ==
            {k \in marks :
                Cardinality({j \in marks : older(j, k)}) < Cardinality(marks) - Tombstones}
    IN [rows |-> [k \in Keys |-> IF k \in dropped THEN NoRow ELSE written[k]],
        floor |-> IF dropped = {} THEN L.floor
                  ELSE Higher(L.floor, Promise!Max({written[k].ver : k \in dropped}))]

\* A value's changelog started at version v with the value x.
Started(x, v) ==
    [rows |-> [k \in Keys |-> IF x[k] = 1 THEN [val |-> 1, ver |-> v] ELSE NoRow], floor |-> 0]

\* What a server that keeps its changelogs in memory reads again after a crash: the durable
\* state's rows at its version, below which it cannot say what was removed.
Rebuilt(a) ==
    [rows |-> [k \in Keys |->
                  IF kept.st[a][k] = 1 THEN [val |-> 1, ver |-> kept.ver] ELSE NoRow],
     floor |-> kept.ver]

\* The version the server reads the session at: its memory's, ahead of the store's where a
\* run's write is owed.
Now == IF ran THEN counter + 1 ELSE counter

\* The cursor a client names of a value it holds as m.
Cursor(m) ==
    IF m.mode = "whole" \/ (m.mode = "reset" /\ ResetCursor)
      THEN [mode |-> m.mode, g |-> m.g, v |-> m.v, k |-> m.k]
      ELSE NoCursor

CursorsOf(c) == [a \in Values |-> Cursor(mem[c][a])]

\* A value's rows changed above u: each row or marker stamped above it, -1 for the others.
RowsAbove(a, u) == [k \in Keys |-> IF log[a].rows[k].ver > u THEN log[a].rows[k].val ELSE -1]

\* A value with the rows of a change on it.
Applied(val, rows) == [k \in Keys |-> IF rows[k] >= 0 THEN rows[k] ELSE val[k]]

-----------------------------------------------------------------------------
\* What is owed for a cursor q of the value a: the change above a whole cursor, the next page
\* of a reset, or a reset's first page, read now.
Need(q, a) ==
    IF q.mode # "none" /\ q.g = gen /\ (FloorCheck => q.v >= log[a].floor)
      THEN IF q.mode = "whole"
             THEN [what |-> "change", from |-> q.v, k |-> 0]
             ELSE [what |-> "page", from |-> q.v, k |-> q.k + 1]
      ELSE [what |-> "page", from |-> Now, k |-> 1]

\* The values owed a page that is not the last, each carried on its own.
Paging(q) == {a \in Values : Need(q[a], a).what = "page" /\ Need(q[a], a).k < NKeys}

\* What a unit carries of the value a: the last page with the change since the reset's first
\* page, or the change above the version held.
Part(q, a) ==
    LET n == Need(q[a], a) IN
    IF n.what = "page"
      THEN [kind |-> "last", from |-> n.from,
            rows |-> [k \in Keys |->
                         IF k = NKeys THEN log[a].rows[k].val
                         ELSE IF CatchUp /\ log[a].rows[k].ver > n.from THEN log[a].rows[k].val
                         ELSE -1]]
      ELSE IF ~OneUnit /\ RowsAbove(a, n.from) = NoRows THEN NoPart
      ELSE [kind |-> "change", from |-> n.from, rows |-> RowsAbove(a, n.from)]

\* Whether anything is owed to the cursors q.
Owes(q) ==
    \/ Paging(q) # {}
    \/ \E a \in Values :
          \/ Part(q, a).kind = "last"
          \/ Part(q, a).kind = "change" /\ Part(q, a).rows # NoRows

\* The answer to the cursors q, read at one version of the session.
Reply(q) ==
    IF Paging(q) # {}
      THEN LET a == CHOOSE b \in Paging(q) : \A d \in Paging(q) : b <= d
               n == Need(q[a], a)
           IN [kind |-> "page", a |-> a, g |-> gen, to |-> Now, f |-> n.from, k |-> n.k,
               row |-> log[a].rows[n.k].val, parts |-> [b \in Values |-> NoPart]]
      ELSE [kind |-> "unit", a |-> 0, g |-> gen, to |-> Now, f |-> 0, k |-> 0, row |-> 0,
            parts |-> [b \in Values |-> Part(q, b)]]

\* What reaches the server of the cursors q a client named: q, or, where the cursors are not
\* signed, q with one whole cursor moved forward by an intermediary.
Carried(q) ==
    IF Digest THEN {q}
    ELSE {q} \cup
         UNION {{[q EXCEPT ![a] = [mode |-> "whole", g |-> q[a].g, v |-> u, k |-> 0]] :
                    u \in {w \in Vers : w > q[a].v}} :
                a \in {b \in Values : q[b].mode = "whole"}}

-----------------------------------------------------------------------------
\* The client's rule for a page (docs/adr/0152, D7): a first page when the cursor it holds now
\* is the one its request named, a later page when it continues the cursor (g, f, k).
PageTakes(c, x) ==
    LET r == x.resp
        m == mem[c][r.a]
    IN \/ ~PageRule
       \/ IF r.k = 1 THEN Cursor(m) = x.sent[r.a]
          ELSE m.mode = "reset" /\ m.g = r.g /\ m.v = r.f /\ m.k = r.k - 1

\* The same for the last page a unit carries of the value a.
LastTakes(c, x, a) ==
    LET r == x.resp
        m == mem[c][a]
    IN \/ ~PageRule
       \/ IF NKeys = 1 THEN Cursor(m) = x.sent[a]
          ELSE m.mode = "reset" /\ m.g = r.g /\ m.v = r.parts[a].from /\ m.k = NKeys - 1

\* The client's rule for a unit: each value it changes is held whole, at the unit's from, at
\* or below its to.
UnitTakes(c, x) ==
    \A a \in Values :
        LET r == x.resp
            p == r.parts[a]
            m == mem[c][a]
        IN /\ p.kind = "change" =>
                 (UnitRule => (m.mode = "whole" /\ m.g = r.g /\ m.v = p.from /\ m.v <= r.to))
           /\ p.kind = "last" => LastTakes(c, x, a)

Takes(c, x) ==
    IF x.resp.kind = "page" THEN PageTakes(c, x) ELSE UnitTakes(c, x)

\* What client c holds once it takes the page r. A later page goes into the reset the client
\* holds, under its cursor's generation and first version; a first page starts one.
PageTaken(c, r) ==
    LET m == mem[c][r.a]
        into == r.k > 1 /\ m.mode = "reset"
    IN [mem[c] EXCEPT ![r.a] =
           [mode |-> "reset",
            g |-> IF into THEN m.g ELSE r.g,
            v |-> IF into THEN m.v ELSE r.f,
            k |-> r.k,
            val |-> [(IF r.k = 1 THEN EmptyRows ELSE m.val) EXCEPT ![r.k] = r.row]]]

\* What client c holds once it takes the answer of x.
Taken(c, x) ==
    LET r == x.resp IN
    IF r.kind = "page" THEN PageTaken(c, r)
    ELSE [a \in Values |->
             IF r.parts[a].kind = "none" THEN mem[c][a]
             ELSE [mode |-> "whole", g |-> r.g, v |-> r.to, k |-> 0,
                   val |-> Applied(mem[c][a].val, r.parts[a].rows)]]

\* An exchange of client c in flight that the server holds, owed nothing.
Idle(c, x) == x \in ex[c] /\ x.resp.kind = "wait" /\ ~Owes(x.cur)

-----------------------------------------------------------------------------
Init ==
    /\ pending = Unset
    /\ counter = 0
    /\ kept = [st |-> Empty, ver |-> 0]
    /\ ran = FALSE
    /\ starts = 0
    /\ gen = 0
    /\ log = [a \in Values |-> [rows |-> [k \in Keys |-> NoRow], floor |-> 0]]
    /\ rec = [c \in Clients |-> 0]
    /\ owed = [c \in Clients |-> {}]
    /\ inc = [c \in Clients |-> 1]
    /\ mem = [c \in Clients |-> [a \in Values |-> NoMem]]
    /\ disk = [c \in Clients |-> Empty]
    /\ stops = [c \in Clients |-> 0]
    /\ losses = [c \in Clients |-> 0]
    /\ ex = [c \in Clients |-> {}]
    /\ repeats = 0
    /\ pub = [p \in Gens \X Vers |-> IF p = <<0, 0>> THEN Empty ELSE Unset]

\* The session is asked to publish x, while the store has a version left for it.
Ask(x) ==
    /\ pending = Unset
    /\ Now < MaxVersion
    /\ pending' = x
    /\ UNCHANGED <<counter, kept, ran, starts, gen, log, pub, serverVars, clientVars, netVars>>

\* The session runs what it was asked and writes it at the next version (Write::values with
\* the checkpoint). Without WithState the changelogs take it in memory, and the write that keeps
\* the state and moves the counter is owed (Checkpoint). Without FromChangelog a write that
\* changes a value puts its version in every subscription's queue, and a full one refuses it.
Write ==
    /\ pending # Unset
    /\ ~ran
    /\ LET v == counter + 1
           moved == \E a \in Values : pending[a] # Cur(log[a])
       IN /\ (~FromChangelog /\ moved) =>
                 \A c \in Clients : (rec[c] # 0 => Cardinality(owed[c]) < QueueBound)
          /\ owed' = IF ~FromChangelog /\ moved
                       THEN [c \in Clients |-> IF rec[c] # 0 THEN owed[c] \cup {v} ELSE owed[c]]
                       ELSE owed
          /\ log' = [a \in Values |-> Wrote(log[a], pending[a], v)]
          /\ pub' = [pub EXCEPT ![<<gen, v>>] = pending]
          /\ IF WithState
               THEN /\ counter' = v
                    /\ kept' = [st |-> pending, ver |-> v]
                    /\ ran' = FALSE
               ELSE /\ ran' = TRUE
                    /\ UNCHANGED <<counter, kept>>
    /\ pending' = Unset
    /\ UNCHANGED <<starts, gen, rec, clientVars, netVars>>

\* Without WithState: the write that keeps the run's state, apart from the changelogs. With it
\* no run's write is owed, and this and Crash are never enabled.
Checkpoint ==
    /\ ran
    /\ counter' = counter + 1
    /\ kept' = [st |-> [a \in Values |-> Cur(log[a])], ver |-> counter + 1]
    /\ ran' = FALSE
    /\ UNCHANGED <<pending, starts, gen, log, pub, serverVars, clientVars, netVars>>

\* The server stops before an owed write, and reads its changelogs again from the store.
Crash ==
    /\ ran
    /\ log' = [a \in Values |-> Rebuilt(a)]
    /\ ran' = FALSE
    /\ UNCHANGED <<pending, counter, kept, starts, gen, pub, serverVars, clientVars, netVars>>

\* A marker's time has passed (tombstone_retention_ms): it is dropped, and the floor follows.
Lapse(a, k) ==
    /\ log[a].rows[k].val = 0
    /\ log[a].rows[k].ver > 0
    /\ log' = [log EXCEPT ![a].rows[k] = NoRow,
                          ![a].floor = Higher(@, log[a].rows[k].ver)]
    /\ UNCHANGED <<pending, counter, kept, ran, starts, gen, pub, serverVars, clientVars,
                   netVars>>

\* The session is removed and started again with the state x, its changelogs with it.
Restart(x) ==
    /\ starts < MaxGen
    /\ ~ran
    /\ counter < MaxVersion
    /\ LET v == counter + 1
           g == IF Generation THEN gen + 1 ELSE gen
       IN /\ gen' = g
          /\ log' = [a \in Values |-> Started(x[a], v)]
          /\ counter' = v
          /\ kept' = [st |-> x, ver |-> v]
          /\ pub' = [pub EXCEPT ![<<g, v>>] = x]
    /\ pending' = Unset
    /\ starts' = starts + 1
    /\ UNCHANGED <<ran, serverVars, clientVars, netVars>>

-----------------------------------------------------------------------------
\* Client c sends an exchange naming every cursor it holds, which subscribes too.
Send(c) ==
    /\ Cardinality(ex[c]) < Exchanges
    /\ \E q \in Carried(CursorsOf(c)) :
          ex' = [ex EXCEPT ![c] = @ \cup {[sent |-> CursorsOf(c), cur |-> q, resp |-> Wait]}]
    /\ UNCHANGED <<sessionVars, serverVars, clientVars, repeats>>

\* The server answers an exchange of client c that is owed something: it records the
\* subscription when it has no record of the client's (one number, one subscription), lets go of
\* what the queue holds at or below the cursors, and answers what is owed.
Answer(c, x) ==
    /\ x \in ex[c]
    /\ x.resp.kind = "wait"
    /\ Owes(x.cur)
    /\ ex' = [ex EXCEPT ![c] = (@ \ {x}) \cup {[x EXCEPT !.resp = Reply(x.cur)]}]
    /\ rec' = IF rec[c] = 0 THEN [rec EXCEPT ![c] = inc[c]] ELSE rec
    /\ owed' = IF ~FromChangelog /\ \A a \in Values : x.cur[a].mode = "whole"
                 THEN [owed EXCEPT ![c] =
                          {u \in @ : \E a \in Values : u > x.cur[a].v}]
                 ELSE owed
    /\ UNCHANGED <<sessionVars, clientVars, repeats>>

\* Client c takes in the answer of an exchange, at any time after it came: a page or a unit and
\* the cursors it moves to in one input (docs/adr/0152, D7), stored in one write, or without
\* OneWrite the cursors first. A copy may be delivered again.
TakeIn(c, x) ==
    /\ x \in ex[c]
    /\ x.resp.kind # "wait"
    /\ \/ ex' = [ex EXCEPT ![c] = @ \ {x}] /\ repeats' = repeats
       \/ ex' = ex /\ repeats < MaxRepeat /\ repeats' = repeats + 1
    /\ IF Takes(c, x)
         THEN LET n == Taken(c, x) IN
              /\ mem' = [mem EXCEPT ![c] = n]
              /\ disk' = IF OneWrite THEN [disk EXCEPT ![c] = [a \in Values |-> n[a].val]]
                         ELSE disk
         ELSE UNCHANGED <<mem, disk>>
    /\ UNCHANGED <<sessionVars, serverVars, inc, stops, losses>>

\* An exchange ends with nothing: its request or its answer was lost, or the server ended its
\* hold.
Drop(c, x) ==
    /\ x \in ex[c]
    /\ ex' = [ex EXCEPT ![c] = @ \ {x}]
    /\ UNCHANGED <<sessionVars, serverVars, clientVars, repeats>>

\* Without OneWrite: the second write, of the values the cursors were stored for. With it the
\* store holds what the memory does, and this is never enabled.
Persist(c) ==
    /\ disk[c] # [a \in Values |-> mem[c][a].val]
    /\ disk' = [disk EXCEPT ![c] = [a \in Values |-> mem[c][a].val]]
    /\ UNCHANGED <<sessionVars, serverVars, inc, mem, stops, losses, netVars>>

\* Client c stops and starts again from its store; the exchanges it had in flight end with it.
Stop(c) ==
    /\ stops[c] < MaxStop
    /\ stops' = [stops EXCEPT ![c] = @ + 1]
    /\ mem' = [mem EXCEPT ![c] = [a \in Values |-> [mem[c][a] EXCEPT !.val = disk[c][a]]]]
    /\ ex' = [ex EXCEPT ![c] = {}]
    /\ UNCHANGED <<sessionVars, serverVars, inc, disk, losses, repeats>>

\* Client c's store is cleared: it holds nothing, and its next exchange is given resets.
LoseStore(c) ==
    /\ losses[c] < MaxLose
    /\ losses' = [losses EXCEPT ![c] = @ + 1]
    /\ mem' = [mem EXCEPT ![c] = [a \in Values |-> NoMem]]
    /\ disk' = [disk EXCEPT ![c] = Empty]
    /\ ex' = [ex EXCEPT ![c] = {}]
    /\ UNCHANGED <<sessionVars, serverVars, inc, stops, repeats>>

\* Client c's incarnation ends (its lease, a change of person, a delegation that ended), with
\* its exchanges, and the next begins with a store of its own; the subscription record it made
\* ends with it.
End(c) ==
    /\ inc[c] < MaxInc
    /\ inc' = [inc EXCEPT ![c] = @ + 1]
    /\ rec' = IF EndWithIncarnation THEN [rec EXCEPT ![c] = 0] ELSE rec
    /\ owed' = IF EndWithIncarnation THEN [owed EXCEPT ![c] = {}] ELSE owed
    /\ mem' = [mem EXCEPT ![c] = [a \in Values |-> NoMem]]
    /\ disk' = [disk EXCEPT ![c] = Empty]
    /\ ex' = [ex EXCEPT ![c] = {}]
    /\ UNCHANGED <<sessionVars, stops, losses, repeats>>

Next ==
    \/ \E x \in States : Ask(x) \/ Restart(x)
    \/ Write \/ Checkpoint \/ Crash
    \/ \E a \in Values, k \in Keys : Lapse(a, k)
    \/ \E c \in Clients :
          \/ Send(c) \/ Persist(c) \/ Stop(c) \/ LoseStore(c) \/ End(c)
          \/ \E x \in ex[c] : Answer(c, x) \/ TakeIn(c, x) \/ Drop(c, x)

Spec == Init /\ [][Next]_vars

\* The session writes what it is asked and keeps it; the server answers what is owed to Fair
\* and ends a held exchange of Fair's that is owed nothing; Fair keeps exchanging, takes in what
\* reaches it and stores it; an exchange sent again and again is answered. Nothing is asked of
\* any other client.
Fairness ==
    /\ WF_vars(Write)
    /\ WF_vars(Checkpoint)
    /\ WF_vars(Send(Fair))
    /\ WF_vars(Persist(Fair))
    /\ SF_vars(\E x \in ex[Fair] : Answer(Fair, x))
    /\ SF_vars(\E x \in ex[Fair] : TakeIn(Fair, x))
    /\ WF_vars(\E x \in ex[Fair] : Idle(Fair, x) /\ Drop(Fair, x))

LiveSpec == Spec /\ Fairness

-----------------------------------------------------------------------------
\* What the protocol keeps of the promise (docs/adr/0152, D10).
Refines == Promise!Spec

Exact == Promise!Exact

OneState == Promise!OneState

Monotone == Promise!Monotone

Silent == Promise!Silent

Authorized == Promise!Authorized

Converges == Promise!Converges
====
