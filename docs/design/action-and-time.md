# Action and time

Status: approved by the human on 2026-11-20; rule 8's creation order and
rule 20 decided the same day. First version: the rules of the
phase 1 walking skeleton. Later designs deepen this system; they do not
replace it. The brainstorm record is `docs/plans/0002-action-and-time-brainstorm.md`.

## Pillars served

- **3. Adventure is the turn.** Every action has a visible time cost, so
  each turn is a trade: move now, wait for an opening, or step aside.
- **4. Every consequence leaves a trace.** The character knows its own
  costs in numbers. It learns other entities only by perceiving what
  they do, in order, with the time it happened.
- **2. The simulation never cheats.** Every entity follows the same
  rules. Ties resolve in a fixed order. Nothing waits for the player
  except time itself.

## Player experience

The player's character stands in a small walled area. A creature
wanders nearby. Before each action, the player sees its cost: moving
costs 100, waiting costs 50. The clock advances by the cost of each
action. In between, the player watches each move of the creature, in
order and stamped with the time.

After a few actions, the player notices that the creature moves twice
for each of their moves. The number never appears; the pattern does.
The player tries to slip past it through a doorway. The creature steps
into the doorway first, and the player's move bumps into it: the
character stays put and loses the 100. The creature perceived it too.
Next time, the player waits 50 and lets it pass.

## Rules

### Time

1. Game time is counted in time units. The clock shows the current time.
2. Time stops while the next character to act waits for its controller.
3. Every entity follows the same rules below, whatever its controller.

### Actions

4. The skeleton has two actions: move one tile in one of eight
   directions, and wait.
5. Each action has a base cost: move 100, wait 50. A diagonal move
   costs the same as a straight one (provisional).
6. Each entity has a speed. Its action cost is the base cost divided by
   its speed. The player's character has speed 1. The skeleton creature
   has speed 2: it moves for 50 and waits for 25.
7. An action takes effect at once. Its cost is the time the entity must
   then spend before it acts again.
8. Entities act in order of the time they are due. When two are due at
   the same time, the one that became due first acts first.
   At world creation, entities become due in the order they are created;
   the character is created first.
9. The player sees the cost their character pays for each action before
   choosing it.

### Moving

10. A move into a tile the character perceives as impassable is refused.
    A refused move is not an action: it costs nothing and the clock does
    not advance.
11. A diagonal move is refused when either orthogonal neighbor it passes
    between is a wall. Corners cannot be cut.
12. A move into an obstacle the character does not perceive costs the
    move. The character stays in place and now perceives the obstacle.
13. A move into a tile occupied by another entity is a bump. The mover
    stays in place and pays the move. Both entities perceive the bump.
    A bump has no other effect.
14. A controller chooses only among actions that will not be refused.
    Waiting is always possible.

### Perception

15. The player learns everything outside their character through what
    the character perceives.
16. In the skeleton, the character perceives the whole map and every
    entity on it.
17. The player sees each perceived change in the order it happened,
    stamped with its time.
18. The character never perceives another entity's speed, cost, or next
    turn as a number. Only its actions are perceived.

### The world between sessions

19. When the player disconnects, their world keeps its state. Time stops
    while the character waits for its controller (rule 2). On return,
    everything is where it was.
20. A character has one controller at a time. A new connection for the
    same character takes it, and the older connection is closed.

## Interactions

This system is the spine other systems plug into. The skeleton fills each
hook with a placeholder rule; the named future design replaces it.

| Hook | Skeleton rule | Future design that replaces it |
|---|---|---|
| New actions | Move and wait only. | Every system that adds a verb (combat, work, magic, speech). Each new action declares its cost and when it takes effect. |
| Action cost | Base cost per action, divided by the entity's speed. | Fatigue, encumbrance, skills, terrain. They change costs, never rule 8. |
| Bump | Fails, costs the move, perceived by both. | Combat and social designs decide which verb a bump resolves to, if any. |
| Perception | Whole map, every entity (rule 16). | Perception design: sight, light, hearing, traces. Rules 10, 12, 15, 17 and 18 already hold under it. |
| Controller | The player, or a random wander for the creature. | Creature behavior design. Rule 14 binds every controller. |
| Map | One small fixed map with walls. | Terrain and world generation. Rule 11 binds every map. |
| Between sessions | State kept while the server runs. | Persistence and catch-up designs. |

## Out of scope

- Combat, damage, death and succession.
- Any perception narrower than the whole map, and any remembered map.
- Terrain types other than floor and wall; vertical movement.
- Creature behavior beyond a random wander.
- Saving to disk, catch-up, and shared worlds.
- What a time unit means in world seconds.

## Open questions

- **Duration of a time unit.** The calendar, aging and the catch-up budget
  need it. Decide in the first design that touches the calendar.
- **Prepare and recover.** Combat may need actions that take effect after
  a delay, so they can be seen coming or interrupted. Decide in the combat
  design.
- **Solvable chases.** Exact costs plus a 50-unit wait make chases
  solvable once a speed is inferred. Decide in the combat design.
- **Rounding of costs.** R6 divides a base cost by a speed. No rule says
  how to round when the division is not exact; phase 1 speeds divide
  exactly. Decide in the first design that adds another speed.
- **Diagonal cost.** Equal to a straight move for now. Decide in the
  terrain design.
