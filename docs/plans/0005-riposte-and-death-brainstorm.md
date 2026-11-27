# 0005 — Riposte and death brainstorm

Status: the human approved the two drafts below on 2026-11-27. Produces an
amendment of `docs/design/attack.md` and a new `docs/design/death.md`. Both
paths are protected: the human places the drafts.

## Inputs

- Pillar 1: death costs time, never the world; with no heir left, the
  player continues as a stranger. Pillar 3: combat is common, and any fight
  can cost a generation. Pillar 4: a system that leaves no readable trace
  is not finished.
- `docs/design/attack.md`: A13 lets only the character attack "in this
  first version". Creature attacks, the character's death, corpses and
  traces are out of its scope.
- `docs/plans/0003-ecs-brainstorm.md`, "Open for the human": "A corpse is a
  new entity linked to its body's identity. To confirm with the death
  design."
- Code: the zero hit points of a character crash through `invariant!`
  (`crates/ash-sim/src/combat.rs`, `hit`).
- Issues: #42 (a struck character never receives its own hit points), #40
  (a creation during play is not in the replay inputs), #37 (whether
  diagonals stay).

## Decisions

### 1 — After its character dies, the player continues as a stranger

The world keeps its state. A new character with a new identity enters the
same world.

- Why: it is the last case of pillar 1 ("with no heir left"), so nothing is
  invented. No bond exists yet, so no heir can exist.
- Rejected: the world stops at the death (against "never the world"); an
  heir designed now (a bond, an ellipsis and time simulated with no player:
  three designs in one).

### 2 — A hostile creature attacks an adjacent character, otherwise it wanders

No pursuit in this version.

- Why: the smallest step. Stepping one tile away stays a real flight.
  Pursuit belongs to the creature behavior design.
- Rejected: pursuit (at speed 2 under whole-map perception, flight becomes
  impossible); pursuit within N tiles (a provisional distance rule that the
  perception design would replace).

### 3 — A neutral creature that a character attacks becomes hostile

Whether the attack hits or misses, and for good. The character perceives
the change.

- Why: the deliberate attack (A3) gains a consequence (pillars 2 and 4).
- Rejected: hostile only when hit (a free attempt about one time in three);
  never hostile (striking a neutral creature stays free).

### 4 — Every death leaves a corpse

On the tile of the death, for a creature and a character alike. A corpse
does not block, does not act and does not decay yet.

- Why: pillar 4. The stranger finds the corpse of the character before it.
- Rejected: a corpse for characters only (two death rules, against R3); no
  corpse yet (a declared gap against pillar 4).

### 5 — The stranger arrives at once

At the time of the death, due after the entities already due then. Its
perceived changes begin at its arrival.

- Why: pillar 1 makes the world advance "until your heir can carry on". A
  stranger has nothing to wait for. The world holds only wandering
  creatures: a delay would change almost nothing.
- Rejected: a fixed delay (an arbitrary number, and nobody perceives that
  time); an arrival the player triggers (a world that runs with no awaited
  character touches R2 and the world lifecycle, #15).

### 6 — Numbers stay provisional

No number changes: 70% to hit, 3 damage, 10 hit points at speed 1 for a
character, 6 hit points at speed 2 for a creature. No healing exists.

A simulation of 200 000 duels, one character against one unhurt creature,
both striking every turn:

| Who strikes first | The character wins | Its hit points left, when it wins |
|---|---|---|
| The character | 75% | 4.7 |
| The creature, once | 65% | 3.3 |
| The creature, twice | 46% | 2.6 |

A first fight is won three times in four. The winner leaves it half dead
and cannot heal, so a second fight is usually lost. This matches "any
fight can cost a generation". Tuned by playing.

## Challenge pass

One `advisor` (Opus) read the draft against the pillars, ADR 0001, both
designs, the glossary and the code. Its claims on the code were checked.

Rewordings taken into the drafts:

- "Its player" is not a thing of the world: the controller passes to the
  stranger (R20).
- "Map entry" is no glossary term, and "nearest" had no metric and no
  tie-break. Death rule 8 names the tile, the metric and the tie-break.
- A struck character perceives its new hit points as a perceived change
  right after the hit, so R17 orders it. A lethal hit sends no "zero hit
  points": the hit, then the death.
- "A corpse keeps the identity" read as "the corpse is the entity". A
  corpse is not an entity; it names the entity that died.
- The change of disposition needed an order among the changes of one
  attack. Attack rule 11 gives it.

Consequences for the spec, not for the designs:

- The server binds one character to a world for good
  (`crates/ash-server/src/registry.rs`, `HostedWorld::character`). After a
  death in play, `step` would refuse every command and a reconnection would
  crash on the dead character's snapshot. The first slice moves this
  binding, and its acceptance test runs over WebSocket.
- The stranger comes from an effect of the lethal hit, inside `resolve`. A
  server call to `create_entity` would break the replay (#40).
- Perception becomes per character: the stranger's changes begin at its
  arrival, and a struck character perceives a number that others do not.
  The resume count of `Hello` counts the changes of a world today.
- The glossary entry of wander says "except attacks". The controller of a
  hostile creature needs a term. Stranger, corpse and arrival need entries.

Dissent, recorded:

- The death of a character costs no time and loses nothing, and it is the
  only way to regain hit points: a character at 1 hit point that dies comes
  back as a stranger at 10. See "Open for the human".
- A stranger cannot tell a creature made hostile from one created hostile:
  a gap against pillar 4. Open question of the attack amendment.
- Once items exist, a corpse that keeps what its character carried makes a
  deliberate death an item transfer. NetHack limits its bones files for
  this reason. Open question of the death design.
- The advisor wanted #42 in its own slice before the riposte. Taken: S2.

Prior art read by the advisor:
`https://minecraft.wiki/w/Gamerule` (`forgive_dead_players`,
`universal_anger`), `https://nethackwiki.com/wiki/Bones`,
`https://www.gamedeveloper.com/design/designing-death-in-co-op-roguelikes`
(players revive at full health and "take turns dying").

## Draft: amendment of `docs/design/attack.md`

Rule numbers stay: spec 0002 cites them.

- Rule 4 reads: "Each creature is either hostile or neutral, and the
  character perceives which. A neutral creature that a character attacks
  becomes hostile, whether the attack hits or misses. It stays hostile."
- Rule 8 reads: "At zero hit points, the entity dies (death design)."
- Rule 9 reads: "The character sees its own hit points as a number. When a
  hit lowers them and the character lives, it perceives the new number
  right after the hit. It never perceives its own condition."
- Rule 11 reads: "A hit, a miss, a character's new hit points, a change of
  condition, a change of disposition and a death are perceived changes
  (action and time, rule 17), stamped with their time. The changes of one
  attack come in this order. First the hit or the miss. Then what the hit
  does to its target: the death, or else the new hit points that a struck
  character perceives (rule 9) and the change of condition that the others
  perceive. Then the change of disposition. A creature that dies of the
  attack does not change disposition."
- Rule 13 reads: "A hostile creature attacks a character on an adjacent
  tile. With several adjacent characters, it picks evenly among them. With
  none, it wanders. A neutral creature wanders. A creature never attacks a
  creature, and a creature's move into any entity stays a bump."
- Rule 14, new: "A creature's attack follows rules 5 and 6: the same cost
  rule, the same odds and the same damage as a character's."
- Out of scope loses "Creatures attacking the character, and the
  character's death" and "Corpses and traces of a death". It gains
  "Pursuit: a creature that walks toward a character".
- Open questions gain:
  - "Why a creature is hostile. A character cannot tell a creature made
    hostile from one created hostile. Decide in the traces design."
  - "Adjacent across a wall corner. A hostile creature can strike across a
    corner that no move can cut (R11). Issue #37 holds the question."

## Draft: `docs/design/death.md`

```markdown
# Death

Status: draft. Brainstorm: `docs/plans/0005-riposte-and-death-brainstorm.md`.

## Pillars served

- **1. Death costs time, never the world.** In part. No death can be
  undone, and the world outlives the character: the creatures, their
  wounds, their dispositions and the corpses stay. In this version a death
  costs no time, and it is the only way to regain hit points. The lineage
  design brings the cost.
- **4. Every consequence leaves a trace.** A death leaves a corpse where it
  happened. A later character finds it.
- **2. The simulation never cheats.** A character dies under the same rule
  as a creature. Nothing spares it.
- **3. Adventure is the turn.** Any fight can cost a character.

## Player experience

The character fights a wolf and loses. The player sees the last hit, then
the death of their character. A stranger now stands where the first
character started. Across the room lies a corpse: a character died there.
The wolf still wanders, still wounded, still hostile. The stranger knows
none of what happened; the player does.

## Rules

1. At zero hit points an entity dies, a character or a creature alike. It
   leaves the world. Its identity is never given to another entity.
2. A death leaves a corpse on the tile where the entity died, from the
   time of the death. A corpse is not an entity. It names the entity that
   died.
3. A corpse never acts and never blocks. An entity can stand on its tile,
   and several corpses can lie on one tile. In this version a corpse never
   decays.
4. A character perceives every corpse on the map and what died there: a
   character or a creature (action and time, rule 16). A character that
   perceived the entity alive recognizes its corpse.
5. The hit and its own death are the last changes a dying character
   perceives.
6. When a character dies, the controller that drove it drives a stranger
   from then on: a new character with a new identity, full hit points and
   the speed of a character. No other controller changes.
7. The stranger arrives at the time of the death.
8. The stranger arrives on the floor tile where the world's first character
   started. If an entity holds that tile, it arrives on the free floor tile
   the fewest moves away, counted as if no entity stood on the map (action
   and time, rules 4 and 11). Among equals, it takes the first in row order
   from the north, each row from the west.
9. The stranger is due at the time of its arrival, after every entity
   already due then (action and time, rule 8). It waits for its turn like
   any entity.
10. An arrival is a perceived change, stamped with its time. The changes of
    a character's death come in this order: the hit, the death, the
    arrival.
11. The stranger knows nothing of the past. It perceives the world as it is
    when it arrives. Its perceived changes begin with its own arrival.
12. The world keeps everything: the clock, the creatures, their hit points,
    their dispositions and the corpses. A creature that the dead character
    made hostile is hostile to the stranger.
13. A stranger dies like any character. Rule 6 then applies again.
14. No death can be undone.

## Interactions

- **Attack.** Attack rule 8 sends here. A hostile creature can now kill a
  character (attack rule 13).
- **Action and time.** The stranger enters the order of rule 8 like any
  entity created during play. Rule 17 now holds per character: each
  character has its own first perceived change. Rule 20 still holds: one
  controller at a time, now across two characters in a row.
- **Perception.** A corpse is a new thing a character perceives.
- **Lineage (future).** An heir replaces the stranger when one exists. The
  ellipsis and the cost of a death come with it.

## Out of scope

- The heir, the lineage and the ellipsis.
- Decay, moving a corpse, and anything taken from a corpse.
- Healing.
- A cause of death other than a hit.

## Open questions

- **The cost of a death.** A death costs no time and loses nothing, and no
  healing exists. A character at 1 hit point that dies comes back as a
  stranger at full hit points. Decide in the lineage design or in the
  healing design, whichever comes first.
- **Corpses and items.** Once a character carries things, a corpse that
  keeps them makes a deliberate death a transfer. Decide in the items
  design.
- **Corpses without bound.** A corpse never decays, so a world keeps every
  corpse. Decide with decay.
- **What a corpse tells.** A corpse shows what died, not how or when. A
  stranger cannot walk the cause back. Decide in the traces design.
- **A map with no free floor tile.** Rule 8 then has no tile to give. It
  cannot happen today: the map has 26 floor tiles and a world holds three
  entities. Decide in the first design that lets entities multiply.

None blocks the first slice.
```

## Decisions after the challenge pass

### 7 — The free death stays, as a declared gap

Decisions 1 and 5 together make a death the only way to regain hit points.
The human keeps both. The death design says it serves pillar 1 in part and
holds the cost of a death as an open question.

- Why: the world still keeps the consequences (hostile creatures, wounds,
  corpses), and nothing is added only to close the gap.
- Rejected: a fixed delay (it costs the player nothing while the world
  holds only wandering creatures); healing designed first (one more design
  before the riposte).

### 8 — A corpse is not an entity

A corpse is a thing on a tile that names the entity that died. This answers
the question `0003` left for the death design: the corpse is not "a new
entity".

- Why: the glossary says an entity "acts when it is due", and a corpse
  never acts. "Names" also survives body parts, which will need identities
  of their own.
- Deferred: whether a corpse has an id of its own waits for the first rule
  that needs one (moving it, cutting it).

## Second-model read

`codex exec` (gpt-6-sol, read-only) read this document against the sources
and the code, on 2026-11-27.

Taken into the drafts:

- Attack rule 11 left the new hit points of a struck character out of its
  order. The order now holds them.
- The death rules did not order the changes of a character's death. Death
  rule 10 does: the hit, the death, the arrival.
- "Its controller passes" did not say what happens with two characters.
  Death rule 6 now names the controller that drove the dead character and
  leaves every other one alone. Rule 9 says the stranger waits for its
  turn.
- "Fewest moves" was unclear when entities stand on the way. Death rule 8
  counts moves as if no entity stood on the map.
- A corpse showed only "a character or a creature". Death rule 4 adds that
  a character recognizes the corpse of an entity it perceived alive. The
  cause and the time of a death stay an open question.
- No rule said what a stranger's own death does. Death rule 13 does.
- A map with no free floor tile is now an open question of the death
  design. It cannot happen on today's map.
- The slice order let the human play deaths before corpses existed, and
  attacks on neutral creatures before they turned hostile. The riposte now
  comes last.

Dissent, recorded and not taken:

- "Death has no time cost" is blocking for a death design, in the reader's
  view. The advisor raised the same point. The human decided it (decision
  7): a declared gap, held as an open question.
- A slice that the human cannot play is not a slice, in the reader's view.
  The methodology asks for one coherent, verifiable behavior change per
  slice; the human plays the milestone. S3 to S5 stay verifiable by tests
  on a world with two characters.

## Slice list

`slice-check` (2026-11-27): the first list had five slices. Its first
slice held the death, the stranger, the server binding and per-character
perception: too much for one session. The second-model read then found
that the order let the human play an unfinished death. The list below has
six slices, and each fills the four lines. The spec owns the final slicing
and the test names; the names here are proposals.

```
S1 — A death leaves a corpse (death 2 to 4)
  Data:     the corpses of a world; the corpse in the snapshot
  Behavior: every death puts a corpse on its tile; an entity can stand on
            it; the web page draws it
  Test:     after the death of a creature, the snapshot of a later
            connection holds a corpse on that tile that names what died;
            a move onto the tile is accepted
  Exit:     `cargo xtask accept corpse`; the human can kill a creature and
            see its corpse

S2 — An attacked neutral creature becomes hostile (attack 4, 11)
  Data:     a disposition that changes; the change of disposition as a
            kind of perceived change
  Behavior: an attack on a neutral creature, hit or miss, makes it hostile
            after the other changes of that attack; a move into it is then
            an attack (attack 1)
  Test:     a missed attack on a neutral creature is followed by the
            change of disposition; a creature killed by the attack
            changes nothing
  Exit:     `cargo xtask accept provoked`; playable

S3 — A character dies and a stranger arrives, in the simulation
     (death 1, 6 to 9, 13, 14)
  Data:     the effects of a lethal hit on a character: its removal, its
            corpse, the creation of the stranger
  Behavior: `combat::hit` returns them instead of crashing; the stranger
            takes the start tile or the free tile rule 8 gives, and its
            turn in the order of R8
  Test:     on a world with two characters, a lethal hit is followed by
            the death, then by a stranger with a new id, full hit points
            and the tile of rule 8; a replay from the seed and the command
            log gives the same world
  Exit:     `cargo xtask accept character_death`

S4 — The controller follows the stranger, which knows nothing of the past
     (death 5, 6, 10, 11)
  Data:     the character a controller drives, which the server holds
            fixed today; the first perceived change of each character; the
            arrival as a kind of perceived change; the resume count of
            `Hello`
  Behavior: after a death, the server sends the hit and the death, then
            the arrival, the stranger's snapshot and its turn; a
            reconnection receives no change from before the arrival; the
            web page shows the stranger
  Test:     over WebSocket, on a test world with two characters, the
            controller of the dead character receives these messages in
            that order, and its next command moves the stranger
  Exit:     `cargo xtask accept character_death_over_websocket`

S5 — A struck character perceives its own hit points (attack 9, #42)
  Data:     a perceived change that carries the struck character's hit
            points, for its controller alone
  Behavior: a hit on a living character sends the number right after the
            hit; its own condition is never sent to it
  Test:     on the two-character world, the struck controller receives
            "7 of 10" after the hit; the other receives the condition
  Exit:     `cargo xtask accept own_hit_points_over_websocket`

S6 — A hostile creature strikes back (attack 13, 14)
  Data:     the controller of a hostile creature, apart from the wander
  Behavior: it attacks an adjacent character, picks evenly among several,
            and wanders otherwise; a neutral creature never attacks
  Test:     a hostile creature next to the character strikes on its turn,
            with the odds and the damage of attack 6; a character in play
            reaches zero hit points and the stranger arrives
  Exit:     `cargo xtask accept riposte`, then the human plays the
            milestone
```

The riposte comes last: no merged slice lets a played world crash at zero
hit points, and the first death the human plays already leaves a corpse
and a stranger. S1 and S2 are playable alone. S3 to S5 are reachable only
by tests until S6.
