# 0001 — Pillars brainstorm

Date: 2026-11-17. Output: `docs/vision/pillars.md`, provisional.
This file keeps what the pillars do not say: the reasons behind the choices,
the challenge findings, and the questions left open.

## Choices made by the human

| Question | Choice |
|---|---|
| What happens at death | The world advances about one generation; play continues as an heir. |
| Who the heir is | Someone the character bound in life. A stranger when none is left. |
| Simulation honesty | Never cheats; the game helps the player find stories. |
| Readability of consequences | Traceable in the world, not through gauges. |
| Breadth or depth | Depth. A magic-like system is wanted. |
| Death in a shared world | Only the lineage waits; the world keeps its pace. |
| Heir protection during the ellipsis | None. |
| The character's own state | Shown in numbers. The world speaks through traces. |
| Moment-to-moment loop | Adventure is the turn; social life gives it a reason. |

## Challenge findings kept for later

An `advisor` challenged the first draft. Findings that are not pillar text:

- **Catch-up budget.** Each death simulates about 18 years while the player
  waits. Six deaths in one evening add about 108 years. Long Dwarf Fortress
  histories tend toward dead worlds, and pillar 2 forbids reseeding them.
  The first spec that touches death sets this budget.
- **First playable milestone.** Pillar 1 needs kinship, aging and a long
  catch-up before it shows once. That is the foundation trap of the two
  previous attempts. Succession as a stranger comes first.
- **Hidden odds look rigged.** Players call honest odds cheating when they
  cannot see them (Sid Meier, GDC 2010). Pillars 2 and 4 forbid both the
  fudge and the readout, so in-world risk cues carry that load.
- **Consistency between levels of detail.** Pillar 2 now requires equal odds
  at every level. This makes the consistency contract of the methodology
  (section 12) a hard requirement.
- **Quarantine.** Recovering a quarantined world means a rollback or a lost
  world. A rollback after a bug is not a player reload, but it must be
  designed with pillar 1 in mind.

## Open questions

- Scale of agency: can a lineage change a kingdom?
- The product-decision table of `0000-methodology.md` sits outside the
  authority order. Give it a rank (constitution change).
- Magic: its own brainstorm and design document. Under pillar 5, it draws
  on and leaves marks in the other systems.
