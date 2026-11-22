# 0004 — Attack brainstorm

Status: in progress (started 2026-11-22). Produces `docs/design/attack.md`,
the design source of slice 1 in `docs/plans/0003-ecs-brainstorm.md`.

## Inputs

- Pillar 3, amended on 2026-11-22 (PR #29): most of the world is hostile,
  combat is common, and any fight can cost a generation.
- `docs/design/action-and-time.md`: bump (R13), refusals (R10, R14),
  speed never perceived (R18).
- ADR 0001 (PR #28): choose, resolve, apply; randomness is an explicit
  input; a refusal draws no randomness.

## Decisions

### A1 — Bump attacks what the character perceives as hostile

A move into a tile held by a creature the character perceives as hostile
is an attack. A move into a tile held by any other entity stays a bump
(R13). An explicit attack action lets the character strike a non-hostile
entity on purpose.

- Why: combat is common, so the frequent case must be one key. The rare
  non-hostile case must never be struck by mistake: no death can be undone
  (pillar 1).
- New concept: perceived hostility. The character knows whether a creature
  is hostile to it, without any number (R18). For this slice, each
  creature has a disposition: hostile or neutral.
- Rejected: bump always attacks (strikes allies by mistake); explicit verb
  only (a chore in a hostile world).

### A2 — Hit points for now, body parts later

An attack that lands removes hit points from its target. At zero hit
points, the target dies.

- Why: the simplest model for the first verb. The human wants body parts
  (Dwarf Fortress style) later; hit points are a placeholder to replace,
  not a model to extend.
- Constraint from pillar 4: the character knows its own hit points as a
  number. It perceives another entity's state only as a trace, never as
  a number.
- Constraint from pillar 3: pools stay small enough that any fight can
  cost a generation.
- Rejected for now: a wound ladder. Body parts: later, by the human's
  choice.

### A3 — An attack can miss

An attack has a chance to hit. A hit deals fixed damage. For this slice,
the chance and the damage are fixed values; skills and equipment set them
later.

- Why: a possible miss makes every attack a tense turn (pillar 3) and keeps
  fleeing a real choice. It exercises the explicit randomness of ADR 0001.
- Rejected: always hit (no tension), always hit with variable damage
  (tension without the risk of a wasted turn).

### A4 — The odds are never a number

For this slice, the player sees nothing of the odds and learns them from
hits and misses. Later, once the target matters (armor, agility), the
character gives a qualitative estimate ("easy blow", "risky",
"desperate"). A raw percentage is excluded for good.

- Why: odds will depend on the target, and pillar 4 lets the character
  know others only through traces. An estimate that never varies adds
  nothing today.

### A5 — The target's state is a perceived condition

The character perceives another entity's condition without numbers:
unhurt, wounded, badly wounded, dying. The condition derives from the hit
points. Crossing a threshold is a perceived change (R17), stamped with its
time like any other.

- Why: the simplest trace that supports the real tactical decision, keep
  fighting or flee (pillar 4). It maps to visible wounds once body parts
  arrive.
- Rejected: hits only (no basis to decide), nothing at all (no trace).

### A6 — Provisional numbers

| Value | Number |
|---|---|
| Attack base cost | 100, divided by speed (R6) |
| Chance to hit | 70% |
| Damage per hit | 3 |
| Character hit points | 10 |
| Creature hit points | 6 |
| Condition thresholds | unhurt at full, wounded below, badly wounded below half, dying below a quarter |

Tuned by playing.

### A7 — First slice scope: only the character attacks

In this slice, only the character attacks. Creatures keep wandering and
never strike back. A creature at zero hit points dies and leaves the
world. Its id is never reused (ADR 0001), so this slice brings
never-reused ids. The corpse and the trace stay for the death slice.

- Why: the human chose the smallest slice. Creature attacks and the
  character's death come with later slices.
- Consequence: the character's hit points exist and are shown, but nothing
  lowers them yet.
- Rejected: both sides attack with a provisional death screen; both sides
  attack without death.

## Open questions

None blocking. Design draft written for the human to place in
`docs/design/attack.md` (a protected path).
