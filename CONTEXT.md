# Glossary

One term = one concept. Use these terms verbatim in code, docs and commits.
New entries are proposed in batches and approved by the human at the milestone.

Seeded from the product decisions in `docs/plans/0000-methodology.md`.

| Term | Definition |
|---|---|
| World | One persistent simulated universe. Today, one world per player. |
| Level of detail | Simulation fidelity of a region: level 0 (full fidelity around each player) to level 3 (history). |
| Tile | One cell of the 3D grid. |
| Chunk | A fixed block of tiles; the unit of sparse storage. |
| Time unit | The unit of game time. |
| Action cost | The number of time units an action takes; the scheduler orders actions by it. |
| Controller | What drives an entity: a player connection or the simulation. The player is an entity with a controller, never a global. |
| Catch-up | Lazy simulation of the time a world spent unloaded, run when the player returns. |
| Quarantine | The state of a world whose simulation hit a violated invariant. Only that world stops. |
| Character | The entity a player controls. |
| Creature | An entity whose controller is the simulation. |
| Speed | The divisor of an entity's base costs. Never perceived by another entity. |
| Base cost | An action's cost at speed 1. |
| Bump | A move into a tile another entity holds: the mover stays, pays the move, and both perceive it. |
| Perception | What a character learns of the world. The only channel from the world to its player. |
| Perceived change | One change a character perceives, stamped with its time and ordered. |
| Map | The fixed terrain of a world: floor and wall tiles. |
| Terrain | What a tile of the map is: floor or wall. |
| Wander | A simulation controller that picks evenly among the actions the rules allow. |
| Snapshot | The whole perceived state, sent when a controller connects. |
| Awaited | The entity whose turn it is. Time stops until its controller chooses. |
| Resume token | Identifies one controller of one world across connections. |
