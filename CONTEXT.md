# Glossary

One term = one concept. Use these terms verbatim in code, docs and commits.
New entries are proposed in batches and approved by the human at the milestone.
The Avoid column names the words never to use for a term; an empty cell avoids none.

Seeded from the product decisions in `docs/plans/0000-methodology.md`.

| Term | Definition | Avoid |
|---|---|---|
| World | One persistent simulated universe. Today, one world per player. | |
| Level of detail | Simulation fidelity of a region: level 0 (full fidelity around each player) to level 3 (history). | |
| Tile | One cell of the 3D grid. | |
| Chunk | A fixed block of tiles; the unit of sparse storage. | |
| Time unit | The unit of game time. | |
| Entity | A thing of a world that acts when it is due, at its own speed, under the same rules whatever its controller. Today, a character or a creature. | |
| Action | What an entity does when it is due. It takes effect at once, and its cost is the time the entity spends before it acts again. Today, move, wait or attack. | |
| Action cost | The number of time units an action takes: the time its entity spends before it is due again. | |
| Scheduler | What orders entities by the time they are due. Of two due at the same time, the one that became due first acts first. | |
| Controller | What drives an entity: a player connection or the simulation. The player is an entity with a controller, never a global. | |
| Catch-up | Lazy simulation of the time a world spent unloaded, run when the player returns. | |
| Quarantine | The state of a world whose simulation violated an invariant. Only that world stops. | |
| Character | The entity a player controls. | |
| Creature | An entity whose controller is the simulation. | monster |
| Speed | The divisor of an entity's base costs. Never perceived by another entity. | |
| Base cost | An action's cost at speed 1. | |
| Bump | A move into a tile another entity holds that is not an attack: the mover stays, pays the move, and both perceive it. | |
| Refusal | The rules turning down a move or an attack, judged on what the entity perceives. A refused move or attack is not an action: it costs nothing, changes nothing and advances no clock. | |
| Offer | An action the rules allow the awaited entity, with the cost it would pay. A controller chooses only among its offers; a wait is always one. | available action |
| Perception | What a character learns of the world. The only channel from the world to its player. | |
| Perceived change | One change a character perceives, stamped with its time and ordered. | |
| Map | The fixed terrain of a world: floor and wall tiles. | |
| Terrain | What a tile of the map is: floor or wall. | |
| Wander | A simulation controller that picks evenly among the actions the rules allow, except attacks. | |
| Snapshot | The whole state one character perceives, sent when its controller connects. | |
| Awaited | The entity whose turn it is. Time stops until its controller chooses. | |
| Resume token | Identifies one controller of one world across connections. | |
| Identity | The permanent, never-reused id of an entity. It outlives the entity's embodiment (ADR 0001). | |
| Embodiment | The presence of an entity in a store, with its components. | |
| Store | The world's tables that hold embodied entities and their components. Its layout is never observable. | |
| Effect | One change a rule returns. Only `apply` writes effects to the store. | |
| Total lookup | A lookup by id that returns "embodied here" or "issued but not embodied here", never a bare absence. | |
| Combat | The system of attacks between entities. | |
| Attack | An action that strikes an adjacent entity: it hits or it misses. A character's move into a tile held by a creature it perceives as hostile is an attack on that creature. | |
| Hit | An attack that lowers its target's health. | |
| Miss | An attack that does not hit. The attacker still pays the action cost. | |
| Damage | How much a hit lowers its target's health. Today, a fixed number of hit points. | |
| Health | The bodily state of an entity. A character knows its own in numbers and perceives another entity's only as a condition. | |
| Hit points | Today's measure of health: a current and a maximum number. A hit lowers the current number; at zero the entity dies. | HP |
| Condition | An entity's health as a kind, never a number: unhurt, wounded, badly wounded or dying. All a character perceives of another entity's health. | |
| Disposition | Whether a creature is hostile or neutral. A character perceives which. | |
| Death | The end of an entity's embodiment for good: the entity leaves the world. Today, at zero hit points. | |
| Outcome | What an action did: today stepped, waited, bumped, hit or missed. A character perceives the outcome, never the action submitted. | |
