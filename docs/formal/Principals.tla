----------------------------- MODULE Principals -----------------------------
(***************************************************************************)
(* The protocol of ADR 0116: a person is bound to an instance where the    *)
(* instance joins, by a proof made for the incarnation it was offered.     *)
(*                                                                         *)
(* A join is OFFERED an incarnation. The CHALLENGE of a proof is computed  *)
(* from the instance and that incarnation, and the person proves who they  *)
(* are to an identity provider, which signs the challenge with the person. *)
(* The exchange that names the offer and carries the proof CONFIRMS it, in *)
(* one write: the member, its person, and the end of the incarnation       *)
(* before. A run begins under the incarnation its event came under, and    *)
(* the write that keeps it is refused when the store no longer holds it.   *)
(* An incarnation ends by a revocation, its time, or another confirmation; *)
(* what was made for the instance under it ends with it, and an exchange under it is told so (the fence).  *)
(*                                                                         *)
(* People change at a device, at any moment. The thief is another instance *)
(* with a key of its own: it and whoever is at a device can present any    *)
(* proof anyone was ever given. A peer machine quotes whom it likes.       *)
(* Messages repeat and are late. Each constant that is a part of the       *)
(* protocol can be switched off, and TLC must then find a violation.       *)
(*                                                                         *)
(* Taken as given:                                                         *)
(*   The instance's key signs its requests, and nobody else has it.        *)
(*   An identity provider signs a proof only for the person who is there,  *)
(*     over the challenge it was given, and the host's port judges it.     *)
(*   What the protocol signs cannot be changed (OpenInstances, the Sign    *)
(*     parts), and an incarnation is drawn once.                           *)
(*   A device shows what it was sent to whoever is at it, under the        *)
(*     incarnation it was sent in; one that changes its person gives up    *)
(*     the incarnation it had.                                             *)
(*   LiveSpec: a message arrives, and what can be done is done.            *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS NInst,                 \* the devices; the last is the thief's
          Persons, Nobody,
          MaxInc,                \* incarnations offered to an instance
          MaxSwitch,             \* how often a person changes at a device
          MaxEnd, MaxCrash,      \* ends that are a revocation's or the time's, and crashes of the server
          MaxRun,                \* requests made
          BindInst,              \* the challenge names the instance and its key
          BindInc,               \* the challenge names the incarnation
          OnePerInc,             \* an incarnation is confirmed once
          Condition,             \* the write of a run is refused when the incarnation ended
          Durable,               \* an end is written before it counts
          Quoted,                \* a quoted person is told as quoted
          Told                   \* an exchange that names an incarnation the server does not hold is told so

Inst == 1..NInst
Incs == 1..MaxInc
People == Persons \cup {Nobody}

VARIABLES member, offer, who, memberD,       \* the server: the member and the offer of an instance, the
                                             \* person of an incarnation, and what the store holds
          drawn, inbox, running, mail,       \* incarnations drawn, what waits, what runs, what is to be sent
          at, cinc, cask,                    \* the devices: who is there, the incarnation, the offer taken
          proofs, net,
          ended, kept, read, proved,         \* what happened
          nSwitch, nEnd, nCrash, nRun

srv  == <<member, offer, who, memberD, drawn, inbox, running, mail>>
dev  == <<at, cinc, cask>>
hist == <<ended, kept, read, proved>>
cnt  == <<nSwitch, nEnd, nCrash, nRun>>
vars == <<srv, dev, proofs, net, hist, cnt>>

Challenge(i, c) == <<IF BindInst THEN i ELSE 0, IF BindInc THEN c ELSE 0>>

Init ==
    /\ member = [i \in Inst |-> 0] /\ memberD = [i \in Inst |-> 0] /\ offer = [i \in Inst |-> 0]
    /\ who = [i \in Inst |-> [c \in Incs |-> Nobody]] /\ drawn = [i \in Inst |-> 0]
    /\ inbox = {} /\ running = {} /\ mail = {}
    /\ at = [i \in Inst |-> Nobody] /\ cinc = [i \in Inst |-> 0] /\ cask = [i \in Inst |-> 0]
    /\ proofs = {} /\ net = {}
    /\ ended = {} /\ kept = {} /\ read = {} /\ proved = {}
    /\ nSwitch = 0 /\ nEnd = 0 /\ nCrash = 0 /\ nRun = 0

-----------------------------------------------------------------------------
\* The devices.

\* Someone sits down at a device, and it gives up the incarnation of who was there. Without OnePerInc it
\* keeps it, and the next person proves who they are under it.
DSwitch(i, p) ==
    /\ nSwitch < MaxSwitch /\ nSwitch' = nSwitch + 1
    /\ at[i] # p /\ at' = [at EXCEPT ![i] = p]
    /\ cinc' = IF OnePerInc THEN [cinc EXCEPT ![i] = 0] ELSE cinc
    /\ cask' = IF OnePerInc THEN [cask EXCEPT ![i] = 0] ELSE [cask EXCEPT ![i] = cinc[i]]
    /\ UNCHANGED <<srv, proofs, net, hist, nEnd, nCrash, nRun>>

\* A join is answered at once with an offer: the handshake is OpenInstances'.
DJoin(i) ==
    /\ at[i] # Nobody /\ cinc[i] = 0 /\ cask[i] = 0 /\ drawn[i] < MaxInc
    /\ drawn' = [drawn EXCEPT ![i] = @ + 1]
    /\ offer' = [offer EXCEPT ![i] = drawn[i] + 1]
    /\ cask' = [cask EXCEPT ![i] = drawn[i] + 1]
    /\ UNCHANGED <<member, memberD, who, inbox, running, mail, at, cinc, proofs, net, hist, cnt>>

\* The person who is there proves who they are for the challenge of the offer.
DProve(i) ==
    /\ at[i] # Nobody /\ cask[i] # 0
    /\ proofs' = proofs \cup {<<at[i], Challenge(i, cask[i])>>}
    /\ proved' = proved \cup {<<at[i], <<i, cask[i]>>>>}
    /\ UNCHANGED <<srv, dev, net, ended, kept, read, cnt>>

\* The exchange that confirms, signed by the instance's key, with any proof there ever was.
DConfirm(i) ==
    /\ cask[i] # 0
    /\ \E f \in proofs : net' = net \cup {[kind |-> "confirm", i |-> i, c |-> cask[i], f |-> f]}
    /\ UNCHANGED <<srv, dev, proofs, hist, cnt>>

DConfirmed(i) ==
    /\ cask[i] # 0 /\ member[i] = cask[i]
    /\ cinc' = [cinc EXCEPT ![i] = cask[i]] /\ cask' = [cask EXCEPT ![i] = 0]
    /\ UNCHANGED <<srv, at, proofs, net, hist, cnt>>

\* The fence: an exchange names an incarnation the server does not hold, and its answer says so.
DFenced(i) ==
    /\ Told
    /\ \/ /\ cask[i] # 0 /\ offer[i] # cask[i] /\ member[i] # cask[i]
          /\ cask' = [cask EXCEPT ![i] = 0] /\ UNCHANGED cinc
       \/ /\ cinc[i] # 0 /\ member[i] # cinc[i]
          /\ cinc' = [cinc EXCEPT ![i] = 0] /\ UNCHANGED cask
    /\ UNCHANGED <<srv, at, proofs, net, hist, cnt>>

DSend(i) ==
    /\ cinc[i] # 0 /\ nRun < MaxRun /\ nRun' = nRun + 1
    /\ net' = net \cup {[kind |-> "req", i |-> i, c |-> cinc[i], q |-> Nobody]}
    /\ UNCHANGED <<srv, dev, proofs, hist, nSwitch, nEnd, nCrash>>

\* A peer machine, which acts for nobody here, quotes whom it likes.
DQuote(i, p) ==
    /\ i = NInst /\ cinc[i] # 0 /\ nRun < MaxRun /\ nRun' = nRun + 1
    /\ net' = net \cup {[kind |-> "req", i |-> i, c |-> cinc[i], q |-> p]}
    /\ UNCHANGED <<srv, dev, proofs, hist, nSwitch, nEnd, nCrash>>

DRead(i, l) ==
    /\ l \in mail /\ l.i = i /\ l.c = cinc[i] /\ member[i] = l.c
    /\ read' = read \cup {<<at[i], l.w>>}
    /\ UNCHANGED <<srv, dev, proofs, net, ended, kept, proved, cnt>>

-----------------------------------------------------------------------------
\* The server. A message taken stays in the network, which repeats it.

\* The challenge is computed from what the server holds, the instance the key proves and its offer.
Good(m) == m.f[2] = Challenge(m.i, m.c)

SConfirm(m) ==
    /\ m.kind = "confirm" /\ Good(m)
    /\ offer[m.i] = m.c \/ (~OnePerInc /\ member[m.i] = m.c)
    /\ who' = [who EXCEPT ![m.i][m.c] = m.f[1]]
    /\ member' = [member EXCEPT ![m.i] = m.c] /\ memberD' = [memberD EXCEPT ![m.i] = m.c]
    /\ offer' = [offer EXCEPT ![m.i] = 0]
    /\ ended' = IF member[m.i] \in {0, m.c} THEN ended ELSE ended \cup {<<m.i, member[m.i]>>}
    /\ mail' = IF OnePerInc THEN {l \in mail : l.i # m.i} ELSE mail
    /\ UNCHANGED <<drawn, inbox, running, dev, proofs, net, kept, read, proved, cnt>>

SEdge(m) ==
    /\ m.kind = "req" /\ member[m.i] = m.c
    /\ inbox' = inbox \cup {m}
    /\ UNCHANGED <<member, offer, who, memberD, drawn, running, mail, dev, proofs, net, hist, cnt>>

\* A run begins, and reads the person of the incarnation its event came under.
SBegin(m) ==
    /\ m \in inbox /\ inbox' = inbox \ {m}
    /\ running' = IF member[m.i] = m.c
                   THEN running \cup {[i |-> m.i, c |-> m.c,
                                       w |-> IF m.q = Nobody THEN who[m.i][m.c] ELSE IF Quoted THEN Nobody ELSE m.q]}
                   ELSE running
    /\ UNCHANGED <<member, offer, who, memberD, drawn, mail, dev, proofs, net, hist, cnt>>

SKeep(r) ==
    /\ r \in running /\ running' = running \ {r}
    /\ IF Condition => memberD[r.i] = r.c
       THEN /\ kept' = kept \cup {<<<<r.i, r.c>>, r.w>>}
            /\ mail' = mail \cup {r}
       ELSE UNCHANGED <<kept, mail>>
    /\ UNCHANGED <<member, offer, who, memberD, drawn, inbox, dev, proofs, net, ended, read, proved, cnt>>

\* A revocation, or the time: the incarnation ends. Without Durable the end is in memory and the store is
\* written by nobody, so a crash takes it back.
SEnd(i) ==
    /\ nEnd < MaxEnd /\ nEnd' = nEnd + 1 /\ member[i] # 0
    /\ ended' = ended \cup {<<i, member[i]>>}
    /\ member' = [member EXCEPT ![i] = 0]
    /\ memberD' = IF Durable THEN [memberD EXCEPT ![i] = 0] ELSE memberD
    /\ mail' = {l \in mail : l.i # i}
    /\ UNCHANGED <<offer, who, drawn, inbox, running, dev, proofs, net, kept, read, proved, nSwitch, nCrash, nRun>>

\* An offer waits for its proof only so long.
SLapse(i) ==
    /\ nEnd < MaxEnd /\ nEnd' = nEnd + 1 /\ offer[i] # 0
    /\ offer' = [offer EXCEPT ![i] = 0]
    /\ UNCHANGED <<member, memberD, who, drawn, inbox, running, mail, dev, proofs, net, hist, nSwitch, nCrash, nRun>>

SCrash ==
    /\ nCrash < MaxCrash /\ nCrash' = nCrash + 1
    /\ member' = memberD /\ inbox' = {} /\ running' = {}
    /\ UNCHANGED <<offer, who, memberD, drawn, mail, dev, proofs, net, hist, nSwitch, nEnd, nRun>>

-----------------------------------------------------------------------------
Next ==
    \/ \E i \in Inst : DJoin(i) \/ DProve(i) \/ DConfirm(i) \/ DConfirmed(i) \/ DFenced(i)
                      \/ DSend(i) \/ SEnd(i) \/ SLapse(i)
    \/ \E i \in Inst, p \in Persons : DSwitch(i, p) \/ DQuote(i, p)
    \/ \E i \in Inst : \E l \in mail : DRead(i, l)
    \/ \E m \in net : SConfirm(m) \/ SEdge(m)
    \/ \E m \in inbox : SBegin(m)
    \/ \E r \in running : SKeep(r)
    \/ SCrash

Spec == Init /\ [][Next]_vars

Fair ==
    /\ \A i \in Inst : WF_vars(DJoin(i)) /\ WF_vars(DProve(i)) /\ WF_vars(DConfirm(i)) /\ WF_vars(DConfirmed(i))
                         /\ WF_vars(DFenced(i))
    /\ \A i \in Inst : WF_vars(\E m \in net : m.i = i /\ m.kind = "confirm" /\ SConfirm(m))

LiveSpec == Spec /\ Fair

-----------------------------------------------------------------------------
\* A person is told only for an incarnation they proved who they are for.
OnlyProved == \A k \in kept : k[2] # Nobody => <<k[2], k[1]>> \in proved

\* An incarnation acts for one person.
OnePerson == \A j, k \in kept : (j[1] = k[1] /\ Nobody \notin {j[2], k[2]}) => j[2] = k[2]

\* Nothing is kept under an incarnation that ended.
NothingAfterEnd == [][\A k \in kept' \ kept : k[1] \notin ended]_vars

\* What was made for a person is read by that person.
RightReader == \A r \in read : r[2] \in {r[1], Nobody}

\* Whoever is at a device when people stop changing is let in, or the incarnations are used up.
Enters == \A i \in Inst : <>[](at[i] = Nobody \/ cinc[i] # 0 \/ drawn[i] = MaxInc)

-----------------------------------------------------------------------------
\* What the protocol implements (docs/adr/0115, D2).
Service == INSTANCE PrincipalService WITH Incarnations <- Inst \X Incs

Refines == Service!Spec

=============================================================================
