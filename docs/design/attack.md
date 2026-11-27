# Attack

Status: approved by the human on 2026-11-22. Brainstorm:
`docs/plans/0004-attack-brainstorm.md`. Amended on 2026-11-27: creatures
strike back, and an attacked neutral creature becomes hostile (rules 4, 8,
9, 11, 13 and 14). Brainstorm:
`docs/plans/0005-riposte-and-death-brainstorm.md`.

## Pillars served

- **3. Adventure is the turn.** Most of the world is hostile and combat is
  common. Every attack can miss, so each blow is a tense decision: keep
  fighting or flee.
- **4. Every consequence leaves a trace.** The character knows its own hit
  points as a number. It knows the state of others only through what it
  perceives: their condition, never a number.
- **2. The simulation never cheats.** Odds are fixed and the same for
  everyone; nothing is bent toward the player.
- **1. Death costs time.** A creature that dies is gone for good, and its
  identity is never given to another.

## Player experience

A wolf wanders near the character. The player moves into the wolf: the
character strikes. The blow misses. The player strikes again and hits; the
wolf now looks wounded. Two more turns, another hit: the wolf dies and
leaves the world.

A neutral creature blocks a corridor. The player moves into it: the
character bumps it, as before, and nothing else happens. To strike it, the
player must choose the attack action on purpose.

## Rules

1. A move into a tile held by a creature the character perceives as
   hostile is an attack on that creature.
2. A move into a tile held by any other entity is a bump (action and time,
   rule 13).
3. The attack action strikes an adjacent entity on purpose, hostile or
   not. It is always available when an entity is adjacent.
4. Each creature is either hostile or neutral, and the character perceives
   which. A neutral creature that a character attacks becomes hostile,
   whether the attack hits or misses. It stays hostile.
5. An attack has a base cost of 100, divided by the attacker's speed
   (action and time, rule 6). It takes effect at once (rule 7).
6. An attack hits 70% of the time. A hit removes 3 hit points from the
   target. A miss changes nothing else; the attacker still pays the cost.
7. The character has 10 hit points; a creature has 6. These numbers are
   provisional, tuned by playing.
8. At zero hit points, the entity dies (death design).
9. The character sees its own hit points as a number. When a hit lowers
   them and the character lives, it perceives the new number right after
   the hit. It never perceives its own condition.
10. The character perceives another entity's condition, never its hit
    points: unhurt at full, wounded below full, badly wounded below half,
    dying below a quarter.
11. A hit, a miss, a character's new hit points, a change of condition, a
    change of disposition and a death are perceived changes (action and
    time, rule 17), stamped with their time. The changes of one attack
    come in this order. First the hit or the miss. Then what the hit does
    to its target: the death, or else the new hit points that a struck
    character perceives (rule 9) and the change of condition that the
    others perceive. Then the change of disposition. A creature that dies
    of the attack does not change disposition.
12. The player never sees the odds of a hit as a number.
13. A hostile creature attacks a character on an adjacent tile. With
    several adjacent characters, it picks evenly among them. With none, it
    wanders. A neutral creature wanders. A creature never attacks a
    creature, and a creature's move into any entity stays a bump.
14. A creature's attack follows rules 5 and 6: the same cost rule, the
    same odds and the same damage as a character's.

## Interactions

- **Action and time.** Attack is a new action with a declared cost and an
  immediate effect, as that design requires of every new verb. Rule 13
  (bump) now applies only to entities the character does not perceive as
  hostile.
- **Perception.** Hostility and condition are new things a character
  perceives. Rule 18 still holds: no speed, cost or hit point of another
  entity is ever perceived as a number.
- **Death and traces.** A death removes the creature. The corpse and the
  trace it leaves belong to the death design.

## Out of scope

- Pursuit: a creature that walks toward a character.
- Body parts and localized wounds: hit points are a placeholder, to be
  replaced by body parts later.
- Skills, weapons and armor that change the odds or the damage.
- A qualitative estimate of the odds ("easy blow", "risky", "desperate"):
  it comes once the target's traits matter.
- Healing.

## Open questions

- **Why a creature is hostile.** A character cannot tell a creature made
  hostile from one created hostile. Decide in the traces design.
- **Adjacent across a wall corner.** A hostile creature can strike across
  a corner that no move can cut (action and time, rule 11). Issue #37
  holds the question.

None blocks a slice.
