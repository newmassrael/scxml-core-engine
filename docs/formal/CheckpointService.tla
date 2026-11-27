-------------------------- MODULE CheckpointService --------------------------
(***************************************************************************)
(* What a client of a server and a reader of its journal may rely on       *)
(* together (docs/adr/0115, D1 and D12): the promise of docs/adr/0113 and  *)
(* the promise of docs/adr/0114 over one server, and what ties them. No    *)
(* write, lease, clock, fence or position is in it.                        *)
(*                                                                         *)
(* A request takes effect at most once and is answered only after it did   *)
(* (AtMostOnceService); the journal only grows, and a reader takes it in   *)
(* order, each entry once (JournalService). And an effect and its entry    *)
(* are one: a request takes effect in the step that makes its entry, an    *)
(* entry of that request alone; the journal holds no entry of anything     *)
(* that did not take effect; and an effect's entry is never passed over,   *)
(* so once the journal holds a later entry of its stream it holds that one *)
(* too. A reader that took the journal in has then taken in every effect   *)
(* up to where it is, once each, and nothing else.                         *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   Nothing: it is what a user may rely on, and names no mechanism.       *)
(***************************************************************************)
EXTENDS Naturals, Sequences

CONSTANTS
    \* the requests, each as <<instance, request>>
    \* @type: Set(<<Int, Int>>);
    Requests,
    \* the streams of the journal, and the most entries of one
    \* @type: Set(Int);
    Streams,
    \* @type: Int;
    MaxEntry

VARIABLES
    \* what AtMostOnceService holds of each request
    \* @type: <<Int, Int>> -> Str;
    status,
    \* @type: <<Int, Int>> -> Int;
    effect,
    \* @type: Bool;
    faulted,
    \* what JournalService holds: the entries, as <<stream, number>>, and its reader
    \* @type: Seq(<<Int, Int>>);
    journal,
    \* @type: Int;
    taken,
    \* @type: <<Int, Int>> -> Int;
    did,
    \* entry[r]: the entry the effect of r made, or None while r has taken no effect
    \* @type: <<Int, Int>> -> <<Int, Int>>;
    entry

vars == <<status, effect, faulted, journal, taken, did, entry>>

Once == INSTANCE AtMostOnceService
Log == INSTANCE JournalService

\* What no entry is.
None == <<0, 0>>

Init == Once!Init /\ Log!Init /\ entry = [r \in Requests |-> None]

\* A request takes effect in the step its entry is made; an entry once made stays, and is of one
\* request.
Made ==
    \A r \in Requests :
        /\ entry[r] # None => entry'[r] = entry[r]
        /\ (effect'[r] # effect[r]) <=> (entry[r] = None /\ entry'[r] # None)
        /\ entry'[r] # None => \A q \in Requests \ {r} : entry'[q] # entry'[r]

\* The journal holds the entries of effects, and nothing else.
Journaled == \A i \in 1..Len(journal) : \E r \in Requests : entry[r] = journal[i]

\* An effect's entry is never passed over: a later entry of its stream is in the journal only
\* with it.
NoneSkipped ==
    \A r \in Requests, i \in 1..Len(journal) :
        (entry[r] # None /\ journal[i][1] = entry[r][1] /\ journal[i][2] > entry[r][2])
            => \E h \in 1..Len(journal) : journal[h] = entry[r]

Next ==
    /\ Once!Next
    /\ Log!Extend \/ Log!Take \/ UNCHANGED Log!vars
    /\ Made
    /\ Journaled'
    /\ NoneSkipped'

Spec == Init /\ [][Next]_vars

=============================================================================
