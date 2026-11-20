# 0002 — Action and time brainstorm

Date: 2026-11-20. Output: `docs/design/action-and-time.md`, the design source
of the phase 1 walking skeleton. This file keeps what the design does not say:
the reasons behind the choices and the challenge findings.

## Choices made by the human

| Question | Choice |
|---|---|
| What the skeleton world holds | The player's character and one creature driven by the simulation, on a small fixed map. |
| Two entities want the same tile | The move fails and still costs its time (a bump). A visible wall is refused at no cost. |
| What the player sees | The whole map, but only through perception: the server sends what the character perceives. |
| What the player knows of time | A clock, and the cost of each action before it is played. Never another entity's speed. |
| Disconnect and return | The world stays in memory while the server runs. |
| Shape of the design | One document per founding system, named after it. Other systems appear as hooks. |
| Corner cutting | Forbidden. |
| First turn order at time 0 (found by the spec review) | Creation order, the character first. |
| Two connections on one character (found by the spec review) | The newest connection takes the controller; the older one is closed. |

Diagonal moves cost the same as straight ones. The human did not decide this;
it is provisional.

## Why

- **One creature.** It is the smallest world that tests the controller
  abstraction and the time-cost scheduler. A lone character orders nothing,
  and its replay proves little. Death would pull in succession, combat and
  the ellipsis: each deserves its own design.
- **Perception as a gate from day one.** A protocol that sends the whole
  world in phase 1 would change for every client once perception narrows.
- **Named after the system, not the skeleton.** A design named after phase 1
  would be a dead source of truth once phase 1 ships.

## Challenge findings

An `advisor` challenged the draft. Changes adopted:

- **Effect timing and ties.** An action takes effect at once; its cost is
  recovery. Ties go to the entity that became due first.
- **The wall rule goes through perception.** Otherwise a narrower perception
  would allow free wall probing in the dark. NetHack charges a move into an
  unseen monster; Nystrom found charging for visible walls too punishing.
- **A refused move is not an action.** Otherwise a walled-in creature
  rerolls forever while the clock stands still.
- **Ordered, timestamped changes.** Speed cannot be inferred from snapshots:
  a creature that moved twice would appear to jump.
- **No verb on a bump.** A bump that becomes an attack on kin is
  irreversible under pillar 1. NetHack needed a "Really attack?" prompt.
- **Rewording.** "The world waits for the player" became "time stops while
  the character waits for its controller", to fit the shared-world bullet of
  pillar 1.
- **Corners.** Brogue forbids corner cutting; doorways become chokepoints.

Rejected: radius perception in the skeleton. It forces a remembered-map rule
and pulls the perception design into phase 1.

For the phase 1 spec: the perception gate must withhold something to be
testable. It withholds speed, next-due time and scheduler state; a negative
test proves the creature's speed never reaches the client.

Sources: journal.stuffwithstuff.com/2014/07/15/a-turn-based-game-loop/,
roguebasin.com/index.php/Time_Systems, riotgames.com (Valorant fog of war),
crawl.develz.org (trunk updates, 6 March 2022), brogue.wiki,
nethackwiki.com/wiki/Fight.
