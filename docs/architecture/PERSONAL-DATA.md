# Personal data in a passport — the design, and what it was chosen over

**Status:** standard. The rule, and the law behind it, are stated once, in the
`personal_data` module of `dpp-domain`. This document records the alternatives
that were weighed and why each was set aside, so a later change can see what it
is reopening.

---

## 1. The question

ESPR Art. 10(1)(e), toys Art. 20(10) and detergents Art. 22(h) forbid storing
customer personal data in a passport without explicit consent under GDPR Art. 6.
The Batteries Regulation has no such provision, but its passport is the one
that records a single unit in use. Before this design the workspace had no
position: GDPR was cited nowhere and nothing refused anything.

Three things had to be decided: whether anything refuses, what an operator
states, and where the personal data itself lives.

## 2. Whether anything refuses

| Option | Why not |
|---|---|
| **A stated position only.** Document that free-text fields must not carry customer personal data, and that the operator is the controller | Nothing enforces it. A prohibition that nothing can refuse is not answered by a sentence about it |
| **A lint.** Flag free text that looks like personal data at publish time | Whether text is personal data cannot be decided by a pattern, so it is both noisy and incomplete. It needs a bypass, and the bypass is where the consent question comes back, later and with data already stored. It runs at publish, while the prohibition is on storing |
| **A modelled statement** *(chosen)*. Schemas mark the fields, and a marked field with a value is refused at write time unless the operator states what is held | It asks the question the law asks, of the person the law asks it of, before anything is stored, and it never reads content |

## 3. What the operator states

| Option | Why not |
|---|---|
| **"Consent exists for these fields"** | Misdescribes a battery passport. There consent is one GDPR basis among several, and data the Regulation requires rests on a legal obligation instead |
| **The basis, from GDPR Art. 6(1)(a)–(f), with a per-act condition** *(chosen)* | One shape serves every regime. The consent-only rule of ESPR, toys and detergents is a condition each act records in its manifest, not a different statement |
| **A pointer to the controller's own evidence, such as its consent record** | The controller keeps that evidence after the data is erased, so a pointer to it in a signed passport would keep relating the passport to the person |

## 4. Where the personal data lives

| Option | Why not |
|---|---|
| **In the passport, beside the statement** | Consent can be withdrawn, and withdrawal obliges erasure where no other ground remains. A published passport is signed, frozen, archived, held by a back-up provider and kept by readers, so data in it could never be erased |
| **Neither data nor statement in the passport** | A reader could no longer tell a field with nothing to hold from one whose related data is withheld |
| **Statement in the passport, data in an erasable record outside it** *(chosen)* | Erasure removes the record and leaves the passport, and its signature, as they were |
| **A disclosure class instead** | Classes decide who is *shown* a field. The prohibition is on *storing*, and a serving-time filter cannot answer a storage-time rule |

## 5. Where the records are held

| Option | Why not |
|---|---|
| **The archive port** | Its contract is the opposite one: archived versions are kept for the passport's life, and these records must be erasable at any time |
| **Only an opaque reference, with the contract left to each host** | Erasability, tombstones and never serving a record are the parts that matter, and a host could get each one wrong with nothing in core saying otherwise |
| **A dedicated port** *(chosen)* | `PersonalDataPort` states the contract once: erase at any time, keep a tombstone without the content, list what a passport has, and never appear in a view |

## 6. Which fields are marked

A field is marked when all three hold: it is operator-written free text; it
describes one item's life after sale; and personal data is not what the
governing act requires it to carry. Marking more would refuse content the law
requires, such as battery Annex XIII point 2(b)'s spare-part contacts, or ask a
statement of measurements that cannot hold anything but a number. Marking less
would leave free text about one unit in use, where data about its owner
naturally ends up, with no question asked of it.
