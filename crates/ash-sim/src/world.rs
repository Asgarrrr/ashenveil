//! One simulated world: its clock, entities, schedule and history.

use alloc::vec::Vec;

use ash_core::entity::{EntityId, EntityIds, Speed};
use ash_core::rng::Rng;
use ash_core::tile::{Terrain, TilePosition};
use ash_core::time::Time;

use crate::action::{self, Action, Offer, Refused};
use crate::combat::Health;
use crate::controller::{Controller, Disposition};
use crate::death::Corpse;
use crate::effect::{Effect, Effects};
use crate::map::Map;
use crate::perception::{PerceivedChange, Perception};
use crate::scheduler::Scheduler;
use crate::wander;

/// Where the character stands when the world begins: a floor tile of the
/// fixed map. The first character of every world starts there (D8).
pub(crate) const CHARACTER_START: TilePosition = TilePosition { x: 1, y: 1, z: 0 };

/// Where the hostile creature stands when the world begins: a floor tile of
/// the fixed map, across the room from the character.
const HOSTILE_START: TilePosition = TilePosition { x: 7, y: 4, z: 0 };

/// Where the neutral creature stands when the world begins: a floor tile of
/// the fixed map, down the west side from the character.
const NEUTRAL_START: TilePosition = TilePosition { x: 1, y: 4, z: 0 };

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entity {
    id: EntityId,
    speed: Speed,
    position: TilePosition,
    controller: Controller,
    health: Health,
}

/// F3: what a total lookup finds under an id the world issued. Never a bare
/// absence.
enum Lookup {
    /// The entity is embodied in this world: its row in `entities`.
    Embodied(usize),
    /// The world issued the id, and no entity is embodied under it: the
    /// entity died (A8).
    NotEmbodied,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    map: Map,
    /// F2: every id this world issued, to the living and to the dead.
    ids: EntityIds,
    /// The embodied entities, in creation order.
    entities: Vec<Entity>,
    /// Its next entry is the turn being played: the clock and the awaited
    /// entity derive from it.
    scheduler: Scheduler,
    rng: Rng,
    /// F6, D12: every corpse, each under its rank among the deaths of this
    /// world. The rank is the explicit key of their order (ADR 0001, point
    /// 5). It is the count of the earlier corpses: an effect that removes a
    /// corpse needs a saved counter of deaths first.
    corpses: Vec<(u64, Corpse)>,
    /// F6, D6: the stranger of each dead character, as (dead, stranger).
    strangers: Vec<(EntityId, EntityId)>,
    perceived_changes: Vec<PerceivedChange>,
    command_log: Vec<Action>,
}

impl World {
    /// The fixed map holding one character, due at time zero. A player
    /// connection controls it, so the world awaits it at once (R2).
    pub fn new(seed: u64, character_speed: Speed) -> Self {
        let mut world = Self {
            map: Map::skeleton(),
            ids: EntityIds::default(),
            entities: Vec::new(),
            scheduler: Scheduler::default(),
            rng: Rng::new(seed),
            corpses: Vec::new(),
            strangers: Vec::new(),
            perceived_changes: Vec::new(),
            command_log: Vec::new(),
        };
        world.create_entity(character_speed, CHARACTER_START, Controller::Player);
        world
    }

    /// The world of the skeleton, from `seed`: the character, then a
    /// hostile and a neutral creature (A4), both wandering at speed 2 (R6,
    /// R8). A replay of a world's accepted commands starts from it too.
    pub fn skeleton(seed: u64) -> Self {
        let mut world = Self::new(seed, Speed::new(1));
        world.create_entity(
            Speed::new(2),
            HOSTILE_START,
            Controller::Wander(Disposition::Hostile),
        );
        world.create_entity(
            Speed::new(2),
            NEUTRAL_START,
            Controller::Wander(Disposition::Neutral),
        );
        world
    }

    /// Creates an entity at `position`, due now: after every entity created
    /// before it (R8). A position off the floor, or already occupied,
    /// crashes through `invariant!`.
    pub fn create_entity(
        &mut self,
        speed: Speed,
        position: TilePosition,
        controller: Controller,
    ) -> EntityId {
        if self.map.terrain(position) != Terrain::Floor || self.occupant(position).is_some() {
            ash_core::invariant!("no entity can be created on {position:?}");
        }
        self.apply(Effects {
            in_order: alloc::vec![Effect::Create {
                speed,
                position,
                controller,
                health: Health::full(controller),
            }],
            rng_after: self.rng.clone(),
        });
        // F2: `apply` issued the id of the entity it created here.
        match self.occupant(position) {
            Some(id) => id,
            None => ash_core::invariant!("no entity was created on {position:?}"),
        }
    }

    /// The time of the turn being played (R1).
    pub fn clock(&self) -> Time {
        self.turn().0
    }

    /// The character whose controller the world waits for. Time stops until
    /// it acts (R2).
    pub fn awaited(&self) -> EntityId {
        self.turn().1
    }

    /// The offers of the awaited character (R9). A move the rules would
    /// refuse is not offered (R14).
    pub fn offers(&self) -> Vec<Offer> {
        self.offers_of(self.awaited())
    }

    /// The offers of `entity`: the actions the rules allow it, with their
    /// costs (R9, R14).
    fn offers_of(&self, entity: EntityId) -> Vec<Offer> {
        let speed = self.speed(entity);
        action::offered(&Perception::whole_map(self), entity)
            .map(|action| Offer {
                action,
                cost: action.cost(speed),
            })
            .collect()
    }

    /// The map, which every character perceives whole (R16).
    pub(crate) fn map(&self) -> &Map {
        &self.map
    }

    /// The number of tiles in each row of the terrain.
    pub(crate) fn terrain_width(&self) -> u32 {
        self.map.width()
    }

    /// Every tile of level 0, row by row from the north, each row from the
    /// west. Under whole-map perception (R16), every character perceives it.
    pub(crate) fn terrain(&self) -> &[Terrain] {
        self.map.tiles()
    }

    /// Every embodied entity, in creation order. Under whole-map perception
    /// (R16), every character perceives each of them.
    pub(crate) fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().map(|entity| entity.id)
    }

    /// Every corpse, in the order of the deaths (D2). Under whole-map
    /// perception (R16), every character perceives each of them (D4).
    pub(crate) fn corpses(&self) -> Vec<Corpse> {
        let mut ranked = self.corpses.clone();
        ranked.sort_unstable_by_key(|&(rank, _)| rank);
        ranked.into_iter().map(|(_, corpse)| corpse).collect()
    }

    /// Where `entity`, which is embodied, stands. Under whole-map perception
    /// (R16), every character perceives it.
    pub fn position(&self, entity: EntityId) -> TilePosition {
        self.entity(entity).position
    }

    /// D6, F3: the stranger that follows `entity`, a dead character. `None`
    /// for an entity that lives, and for a dead creature. An id this world
    /// never issued crashes through `invariant!`.
    pub fn stranger_of(&self, entity: EntityId) -> Option<EntityId> {
        if !self.ids.has_issued(entity) {
            ash_core::invariant!("this world never issued {entity:?}");
        }
        self.strangers
            .iter()
            .find(|&&(dead, _)| dead == entity)
            .map(|&(_, stranger)| stranger)
    }

    /// The awaited character takes `action`, then time runs to the next
    /// character that awaits a player connection. A refused move or attack
    /// changes nothing (R10, R11, A3).
    pub fn submit(&mut self, action: Action) -> Result<(), Refused> {
        let perception = Perception::whole_map(self);
        let effects = action::resolve(self, &perception, self.awaited(), action, &self.rng)?;
        self.apply(effects);
        // Controller input, not world state (F6): replay evidence only.
        self.command_log.push(action);
        self.advance();
        Ok(())
    }

    /// Every change, in the order it happened (R17). Under whole-map
    /// perception (R16), every character perceives every change.
    pub fn perceived_changes(&self) -> &[PerceivedChange] {
        &self.perceived_changes
    }

    /// The accepted commands of player connections, in order: replaying them
    /// on a world with the same seed and entities gives the same state. The
    /// wander needs no log: the RNG replays it.
    pub fn command_log(&self) -> &[Action] {
        &self.command_log
    }

    /// Runs the clock entry by entry. A wandering entity acts at once; the
    /// clock stops at the first entity a player connection controls (R2).
    /// The entry of an entity that died is dropped: nothing acts and
    /// nothing draws (F3).
    fn advance(&mut self) {
        loop {
            let entity = self.turn().1;
            let effects = match self.lookup(entity) {
                Lookup::NotEmbodied => Effects {
                    in_order: alloc::vec![Effect::Unschedule],
                    rng_after: self.rng.clone(),
                },
                Lookup::Embodied(row) => match self.entities[row].controller {
                    Controller::Player => return,
                    Controller::Wander(_) => {
                        let offers = self.offers_of(entity);
                        let (action, rng) = wander::choose(&self.rng, &offers);
                        let perception = Perception::whole_map(self);
                        let Ok(effects) = action::resolve(self, &perception, entity, action, &rng)
                        else {
                            ash_core::invariant!("the wander of {entity:?} chose a refused action")
                        };
                        effects
                    }
                },
            };
            self.apply(effects);
        }
    }

    /// F4: the only write path. Writes each effect in order, then installs
    /// the RNG state after the draws.
    fn apply(&mut self, effects: Effects) {
        for effect in effects.in_order {
            match effect {
                Effect::Place { entity, position } => self.entity_mut(entity).position = position,
                Effect::Perceive(change) => self.perceived_changes.push(change),
                Effect::EndTurn { due } => {
                    let Some((_, entity)) = self.scheduler.pop() else {
                        ash_core::invariant!("no entity is scheduled")
                    };
                    self.scheduler.schedule(entity, due);
                }
                Effect::Unschedule => {
                    if self.scheduler.pop().is_none() {
                        ash_core::invariant!("no entity is scheduled");
                    }
                }
                Effect::Damage { entity, health } => self.entity_mut(entity).health = health,
                Effect::ChangeDisposition {
                    entity,
                    disposition,
                } => {
                    let creature = self.entity_mut(entity);
                    let Controller::Wander(_) = creature.controller else {
                        ash_core::invariant!("the character {entity:?} has no disposition")
                    };
                    creature.controller = Controller::Wander(disposition);
                }
                Effect::Remove { entity } => {
                    let row = self.row(entity);
                    self.entities.remove(row);
                }
                Effect::LeaveCorpse(corpse) => {
                    let Ok(rank) = u64::try_from(self.corpses.len()) else {
                        ash_core::invariant!("the deaths of this world outnumber a rank")
                    };
                    self.corpses.push((rank, corpse));
                }
                Effect::Create {
                    speed,
                    position,
                    controller,
                    health,
                } => {
                    let id = self.ids.issue();
                    self.embody(Entity {
                        id,
                        speed,
                        position,
                        controller,
                        health,
                    });
                }
                Effect::CreateStranger {
                    dead,
                    speed,
                    position,
                    health,
                } => {
                    let id = self.ids.issue();
                    self.embody(Entity {
                        id,
                        speed,
                        position,
                        controller: Controller::Player,
                        health,
                    });
                    self.strangers.push((dead, id));
                }
            }
        }
        self.rng = effects.rng_after;
    }

    /// Embodies `entity`, due now, after every entity already due then (R8).
    fn embody(&mut self, entity: Entity) {
        // The first entity has no turn to join: the world begins at time
        // zero.
        let now = self.scheduler.head().map_or(Time::ZERO, |(due, _)| due);
        self.scheduler.schedule(entity.id, now);
        self.entities.push(entity);
    }

    /// The scheduler's next entry: the time of the turn being played, and
    /// the entity it belongs to.
    pub(crate) fn turn(&self) -> (Time, EntityId) {
        match self.scheduler.head() {
            Some((due, entity)) => (due, entity),
            None => ash_core::invariant!("no entity is scheduled"),
        }
    }

    /// The entity standing at `position`, if any.
    pub(crate) fn occupant(&self, position: TilePosition) -> Option<EntityId> {
        self.entities
            .iter()
            .find(|entity| entity.position == position)
            .map(|entity| entity.id)
    }

    pub(crate) fn speed(&self, entity: EntityId) -> Speed {
        self.entity(entity).speed
    }

    /// What drives `entity`: a player connection for a character, the
    /// simulation for a creature.
    pub(crate) fn controller(&self, entity: EntityId) -> Controller {
        self.entity(entity).controller
    }

    /// The hit points of `entity` (A7). Only its own controller may learn
    /// them (R18).
    pub(crate) fn health(&self, entity: EntityId) -> Health {
        self.entity(entity).health
    }

    /// F3: the total lookup. An id this world never issued crashes through
    /// `invariant!`.
    fn lookup(&self, entity: EntityId) -> Lookup {
        let embodied = |candidate: &Entity| candidate.id == entity;
        match self.entities.iter().position(embodied) {
            Some(row) => Lookup::Embodied(row),
            None if self.ids.has_issued(entity) => Lookup::NotEmbodied,
            None => ash_core::invariant!("this world never issued {entity:?}"),
        }
    }

    /// The row of `entity`, which is embodied: a rule names only entities it
    /// perceives or the one whose turn it is.
    fn row(&self, entity: EntityId) -> usize {
        match self.lookup(entity) {
            Lookup::Embodied(row) => row,
            Lookup::NotEmbodied => ash_core::invariant!("{entity:?} is not embodied"),
        }
    }

    fn entity(&self, entity: EntityId) -> &Entity {
        &self.entities[self.row(entity)]
    }

    fn entity_mut(&mut self, entity: EntityId) -> &mut Entity {
        let row = self.row(entity);
        &mut self.entities[row]
    }
}

#[cfg(test)]
mod tests {
    use ash_core::tile::Direction;

    use super::*;
    use crate::combat::Condition;
    use crate::perception::{ChangeKind, Outcome};

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn an_entity_created_on_another_crashes() {
        let mut world = World::new(0, Speed::new(1));
        world.create_entity(
            Speed::new(2),
            CHARACTER_START,
            Controller::Wander(Disposition::Neutral),
        );
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn an_entity_created_on_a_wall_crashes() {
        let mut world = World::new(0, Speed::new(1));
        let wall = TilePosition { x: 0, y: 0, z: 0 };
        world.create_entity(
            Speed::new(2),
            wall,
            Controller::Wander(Disposition::Neutral),
        );
    }

    #[test]
    fn a_removed_entity_is_issued_but_not_embodied_and_stays_scheduled() {
        let mut world = World::new(0, Speed::new(1));
        let character = world.awaited();
        let [removed, second, third] = [2, 4, 6].map(|x| {
            world.create_entity(
                Speed::new(2),
                TilePosition { x, y: 1, z: 0 },
                Controller::Wander(Disposition::Hostile),
            )
        });
        let scheduled = world.scheduler.clone();

        world.apply(Effects {
            in_order: alloc::vec![Effect::Remove { entity: removed }],
            rng_after: world.rng.clone(),
        });

        // F3: the two cases of a total lookup.
        assert!(matches!(world.lookup(removed), Lookup::NotEmbodied));
        assert!(matches!(world.lookup(character), Lookup::Embodied(_)));
        // The others stay in creation order.
        let embodied: Vec<EntityId> = world.entities().collect();
        assert_eq!(embodied, [character, second, third]);
        // The entry outlives the embodiment: `advance` drops it later.
        assert_eq!(world.scheduler, scheduled);
    }

    #[test]
    fn a_passed_dead_entry_acts_nothing_and_leaves_one_entry_per_embodied_entity() {
        let mut world = World::new(0, Speed::new(1));
        let character = world.awaited();
        let [dead, wounded] = [2, 4].map(|x| {
            world.create_entity(
                Speed::new(2),
                TilePosition { x, y: 1, z: 0 },
                Controller::Wander(Disposition::Hostile),
            )
        });
        // One creature left the world, as at its death (A8). A hit wounded
        // the other, which lives (A6).
        world.apply(Effects {
            in_order: alloc::vec![
                Effect::Remove { entity: dead },
                Effect::Damage {
                    entity: wounded,
                    health: Health { current: 3, max: 6 },
                },
            ],
            rng_after: world.rng.clone(),
        });

        // The entries of both creatures are due before the next turn of the
        // character.
        assert_eq!(world.submit(Action::Wait), Ok(()));

        let acted = |entity| {
            world
                .perceived_changes
                .iter()
                .any(|change| change.entity == entity && matches!(change.kind, ChangeKind::Act(_)))
        };
        assert!(acted(wounded));
        assert!(!acted(dead));
        // From the next to act: the awaited character, then the creature.
        let mut rest = world.scheduler.clone();
        let scheduled: Vec<EntityId> = core::iter::from_fn(|| rest.pop())
            .map(|(_, entity)| entity)
            .collect();
        assert_eq!(scheduled, [character, wounded]);
    }

    /// A11: a hit is followed by a change of condition only when it crosses
    /// a threshold of A10. Of 20 hit points, half is 10 and a quarter is 5;
    /// a hit removes 3 (A6).
    #[test]
    fn a_hit_changes_the_condition_only_when_it_crosses_a_threshold() {
        let hits = [
            (20, Some(Condition::Wounded)),
            (17, None),
            // To exactly half, then below it.
            (13, None),
            (12, Some(Condition::BadlyWounded)),
            (9, None),
            // To exactly a quarter, then below it.
            (8, None),
            (7, Some(Condition::Dying)),
            (4, None),
        ];
        let east = TilePosition { x: 2, y: 1, z: 0 };
        for (current, crossed) in hits {
            // The first draw of seed 182 hits (A6). The target is a
            // character, so the world stops at its turn.
            let mut world = World::new(182, Speed::new(1));
            let attacker = world.awaited();
            world.apply(Effects {
                in_order: alloc::vec![Effect::Create {
                    speed: Speed::new(1),
                    position: east,
                    controller: Controller::Player,
                    health: Health { current, max: 20 },
                }],
                rng_after: world.rng.clone(),
            });
            let target = world.occupant(east).expect("the target stands east");

            assert_eq!(world.submit(Action::Attack(Direction::East)), Ok(()));

            let mut expected = alloc::vec![PerceivedChange {
                time: Time::ZERO,
                entity: attacker,
                kind: ChangeKind::Act(Outcome::Hit(target)),
            }];
            expected.extend(crossed.map(|condition| PerceivedChange {
                time: Time::ZERO,
                entity: target,
                kind: ChangeKind::Condition(condition),
            }));
            assert_eq!(world.perceived_changes, expected, "{current} of 20");
        }
    }

    /// A10, A11: a hit that crosses two thresholds at once changes the
    /// condition to the one the hit points left show, not to the next worse
    /// one. Of 4 hit points, a hit leaves 1 (A6): below half, and not below
    /// a quarter.
    #[test]
    fn a_hit_across_two_thresholds_changes_the_condition_to_the_one_reached() {
        // The first draw of seed 182 hits (A6). The target is a character,
        // so the world stops at its turn.
        let mut world = World::new(182, Speed::new(1));
        let attacker = world.awaited();
        let east = TilePosition { x: 2, y: 1, z: 0 };
        world.apply(Effects {
            in_order: alloc::vec![Effect::Create {
                speed: Speed::new(1),
                position: east,
                controller: Controller::Player,
                health: Health { current: 4, max: 4 },
            }],
            rng_after: world.rng.clone(),
        });
        let target = world.occupant(east).expect("the target stands east");

        assert_eq!(world.submit(Action::Attack(Direction::East)), Ok(()));

        assert_eq!(
            world.perceived_changes,
            [
                PerceivedChange {
                    time: Time::ZERO,
                    entity: attacker,
                    kind: ChangeKind::Act(Outcome::Hit(target)),
                },
                PerceivedChange {
                    time: Time::ZERO,
                    entity: target,
                    kind: ChangeKind::Condition(Condition::BadlyWounded),
                },
            ]
        );
    }

    /// A11: a creature that dies of the attack does not change disposition.
    /// No play reaches this case: the first attack on a neutral creature
    /// provokes it, and one hit cannot kill a creature at full hit points.
    #[test]
    fn a_neutral_creature_that_the_hit_kills_changes_no_disposition() {
        // The first draw of seed 182 hits (A6), and a hit removes 3 hit
        // points.
        let mut world = World::new(182, Speed::new(1));
        let attacker = world.awaited();
        let east = TilePosition { x: 2, y: 1, z: 0 };
        world.apply(Effects {
            in_order: alloc::vec![Effect::Create {
                speed: Speed::new(2),
                position: east,
                controller: Controller::Wander(Disposition::Neutral),
                health: Health { current: 3, max: 6 },
            }],
            rng_after: world.rng.clone(),
        });
        let target = world.occupant(east).expect("the target stands east");

        assert_eq!(world.submit(Action::Attack(Direction::East)), Ok(()));

        // The hit, then the death: the attacker alone is left, and awaited.
        assert_eq!(
            world.perceived_changes,
            [
                PerceivedChange {
                    time: Time::ZERO,
                    entity: attacker,
                    kind: ChangeKind::Act(Outcome::Hit(target)),
                },
                PerceivedChange {
                    time: Time::ZERO,
                    entity: target,
                    kind: ChangeKind::Death,
                },
            ]
        );
        // D6: a dead creature has no stranger.
        let embodied: Vec<EntityId> = world.entities().collect();
        assert_eq!(embodied, [attacker]);
        assert_eq!(world.awaited(), attacker);
    }

    /// F4: `apply` writes the health a damage carries, as carried, and
    /// decides nothing. The rule returns the change of condition.
    #[test]
    fn a_damage_writes_the_health_it_carries_and_perceives_nothing() {
        let mut world = World::new(0, Speed::new(1));
        let creature = world.create_entity(
            Speed::new(2),
            TilePosition { x: 2, y: 1, z: 0 },
            Controller::Wander(Disposition::Hostile),
        );
        // Not what a hit leaves of 6 hit points (A6), and below every
        // threshold (A10).
        let health = Health { current: 1, max: 6 };

        world.apply(Effects {
            in_order: alloc::vec![Effect::Damage {
                entity: creature,
                health,
            }],
            rng_after: world.rng.clone(),
        });

        assert_eq!(world.health(creature), health);
        assert_eq!(world.perceived_changes, []);
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn a_lookup_of_an_id_the_world_never_issued_crashes() {
        // The world issued one id, the character's: the next one is the
        // nearest id it never issued.
        let world = World::new(0, Speed::new(1));
        let _ = world.lookup(EntityId::new(1));
    }

    /// F3: `stranger_of` answers for an id the world issued, never with a
    /// bare absence for another.
    #[test]
    #[should_panic(expected = "this world never issued")]
    fn the_stranger_of_an_id_the_world_never_issued_crashes_as_never_issued() {
        let world = World::new(0, Speed::new(1));
        let _ = world.stranger_of(EntityId::new(1));
    }

    /// F3: a write names which of the two failures it met.
    #[test]
    #[should_panic(expected = "this world never issued")]
    fn a_damage_to_an_id_the_world_never_issued_crashes_as_never_issued() {
        let mut world = World::new(0, Speed::new(1));
        world.apply(Effects {
            in_order: alloc::vec![Effect::Damage {
                entity: EntityId::new(1),
                health: Health { current: 3, max: 6 },
            }],
            rng_after: world.rng.clone(),
        });
    }

    #[test]
    #[should_panic(expected = "this world never issued")]
    fn a_removal_of_an_id_the_world_never_issued_crashes_as_never_issued() {
        let mut world = World::new(0, Speed::new(1));
        world.apply(Effects {
            in_order: alloc::vec![Effect::Remove {
                entity: EntityId::new(1),
            }],
            rng_after: world.rng.clone(),
        });
    }

    #[test]
    fn a_wandering_creature_next_to_an_entity_is_offered_the_attack() {
        // R3: the offers do not depend on the controller. The wander sets
        // the attack aside itself (A13).
        let mut world = World::new(0, Speed::new(1));
        let east = TilePosition { x: 2, y: 1, z: 0 };
        let creature = world.create_entity(
            Speed::new(2),
            east,
            Controller::Wander(Disposition::Neutral),
        );
        let attacks: Vec<Action> = world
            .offers_of(creature)
            .into_iter()
            .map(|offer| offer.action)
            .filter(|action| matches!(action, Action::Attack(_)))
            .collect();
        assert_eq!(attacks, [Action::Attack(Direction::West)]);
    }
}
