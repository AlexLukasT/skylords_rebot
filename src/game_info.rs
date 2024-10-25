use crate::card_data::CardData;
use api::*;
use log::*;
use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU32;

use crate::location::{get_location_positions, Location, LocationPosition, TokenSubLocation};
use crate::utils;

// minimum distance required to build structure
pub const GROUND_PRESENCE_MIN_DIST: f32 = 5.;

pub struct GameInfo {
    pub state: Option<GameState>,
    pub bot: PlayerInfo,
    pub opponent: PlayerInfo,
    pub current_tick: Option<Tick>,
    pub locations: BTreeMap<Location, LocationPosition>,
    pub figures: Vec<Figure>,
    pub card_data: CardData,
}

impl fmt::Debug for GameInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameInfo")
            .field("bot", &self.bot)
            .field("opponent", &self.opponent)
            .field("current_tick", &self.current_tick)
            .finish()
    }
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub id: EntityId,
    pub team: u8,
    pub power_slots: BTreeMap<EntityId, PowerSlot>,
    pub token_slots: BTreeMap<EntityId, TokenSlot>,
    pub power: f32,
    pub void_power: f32,
    pub tempo: f32, // Power + Bound Power - Void Power
    pub squads: BTreeMap<EntityId, Squad>,
    pub new_squad_ids: Vec<EntityId>, // Squads that were just spawned
    pub dead_squad_ids: Vec<EntityId>, // Squads that just died
    pub start_token: Option<EntityId>,
    pub start_location: Location,
    pub new_power_slot_ids: Vec<EntityId>,
    pub new_token_slot_ids: Vec<EntityId>,
    pub destroyed_power_slot_ids: Vec<EntityId>,
    pub destroyed_token_slot_ids: Vec<EntityId>,
}

impl GameInfo {
    pub fn new() -> GameInfo {
        let mut card_data = CardData::new();
        card_data.load();

        GameInfo {
            state: None,
            bot: PlayerInfo {
                id: EntityId(NonZeroU32::new(1).unwrap()),
                team: 0,
                power_slots: BTreeMap::new(),
                token_slots: BTreeMap::new(),
                power: 0.,
                void_power: 0.,
                tempo: 0.,
                squads: BTreeMap::new(),
                new_squad_ids: vec![],
                dead_squad_ids: vec![],
                start_token: None,
                start_location: Location::Center,
                new_power_slot_ids: vec![],
                new_token_slot_ids: vec![],
                destroyed_power_slot_ids: vec![],
                destroyed_token_slot_ids: vec![],
            },
            opponent: PlayerInfo {
                id: EntityId(NonZeroU32::new(1).unwrap()),
                team: 0,
                power_slots: BTreeMap::new(),
                token_slots: BTreeMap::new(),
                power: 0.,
                void_power: 0.,
                tempo: 0.,
                squads: BTreeMap::new(),
                new_squad_ids: vec![],
                dead_squad_ids: vec![],
                start_token: None,
                start_location: Location::Center,
                new_power_slot_ids: vec![],
                new_token_slot_ids: vec![],
                destroyed_power_slot_ids: vec![],
                destroyed_token_slot_ids: vec![],
            },
            current_tick: None,
            locations: BTreeMap::new(),
            figures: vec![],
            card_data,
        }
    }

    pub fn init(&mut self, start_state: GameStartState) {
        debug!("Starting intializing game info");

        self.bot.id = start_state.your_player_id;

        // find the bot's team
        if let Some(bot_team) = start_state
            .players
            .iter()
            .find(|&p| p.entity.id == self.bot.id)
        {
            self.bot.team = bot_team.entity.team;
        } else {
            error!(
                "Unable to find team for Bot with id {:?} in GameStartState",
                self.bot.id
            );
        }

        // find the opponent player id
        let mut opponent_ids = vec![];
        for player in &start_state.players {
            if player.entity.id != self.bot.id {
                opponent_ids.push(player.entity.id);
            }
        }
        if opponent_ids.len() > 1 {
            warn!(
                "Found more than one opponent, choosing the first one ({:?})",
                opponent_ids[0]
            );
        }
        self.opponent.id = opponent_ids[0];

        // find the opponent's team
        if let Some(opponent_team) = start_state
            .players
            .iter()
            .find(|&p| p.entity.id == self.opponent.id)
        {
            self.opponent.team = opponent_team.entity.team;
        } else {
            error!(
                "Unable to find team for Opponent with id {:?} in GameStartState",
                self.opponent.id
            );
        }

        let mut location_positions = get_location_positions();

        for power_slot in start_state.entities.power_slots {
            let slot_id = power_slot.entity.id;

            // assign power slot to it's location
            let mut found_location: Option<Location> = None;
            let mut found_power_index: Option<usize> = None;
            let mut found_entity_id: Option<EntityId> = None;
            for (location, location_pos) in location_positions.iter() {
                for (i, pos_power_slot) in location_pos.powers.iter().enumerate() {
                    let power_slot_x = power_slot.entity.position.to_2d().x;
                    let power_slot_y = power_slot.entity.position.to_2d().y;
                    if power_slot_x == pos_power_slot.position.x
                        && power_slot_y == pos_power_slot.position.y
                    {
                        // set the entity id in location_positions
                        found_location = Some(*location);
                        found_power_index = Some(i);
                        found_entity_id = Some(slot_id);
                        debug!(
                            "Assigned power slot {:?} to location {:?}.{:?}",
                            slot_id, location, i
                        );
                    }
                }
            }

            if let Some(location) = found_location {
                location_positions.get_mut(&location).unwrap().powers[found_power_index.unwrap()]
                    .entity_id = found_entity_id;
            } else {
                warn!("Unable to find location for power slot {:?}", slot_id);
            }

            // find power slots for each player
            if let Some(power_slot_player_id) = power_slot.entity.player_entity_id {
                if power_slot_player_id == self.bot.id {
                    self.bot.power_slots.insert(slot_id, power_slot);
                } else if power_slot_player_id == self.opponent.id {
                    self.opponent.power_slots.insert(slot_id, power_slot);
                }
            }
        }

        for token_slot in start_state.entities.token_slots {
            let slot_id = token_slot.entity.id;

            // assign token slot to it's location
            let mut found_location: Option<Location> = None;
            let mut found_entity_id: Option<EntityId> = None;
            let mut found_pos_x: Option<f32> = None;
            let mut found_pos_y: Option<f32> = None;
            for (location, location_pos) in location_positions.iter() {
                if let Some(pos_token_slot) = location_pos.token {
                    let token_slot_x = token_slot.entity.position.to_2d().x;
                    let token_slot_y = token_slot.entity.position.to_2d().y;
                    if token_slot_x == pos_token_slot.position.x
                        && token_slot_y == pos_token_slot.position.y
                    {
                        // set the entity id in location_positions
                        found_location = Some(*location);
                        found_entity_id = Some(slot_id);
                        found_pos_x = Some(token_slot_x);
                        found_pos_y = Some(token_slot_y);
                        debug!(
                            "Assigned token slot {:?} to location {:?}",
                            slot_id, location
                        );
                    }
                }
            }

            if let Some(location) = found_location {
                location_positions.get_mut(&location).unwrap().token = Some(TokenSubLocation {
                    position: Position2D {
                        x: found_pos_x.unwrap(),
                        y: found_pos_y.unwrap(),
                    },
                    entity_id: found_entity_id,
                });
            } else {
                warn!("Unable to find location for token slot {:?}", slot_id);
            }

            // find token slots for each player
            if let Some(token_slot_player_id) = token_slot.entity.player_entity_id {
                if token_slot_player_id == self.bot.id {
                    self.bot.token_slots.insert(slot_id, token_slot);
                    self.bot.start_token = Some(slot_id);
                } else if token_slot_player_id == self.opponent.id {
                    self.opponent.token_slots.insert(slot_id, token_slot);
                    self.opponent.start_token = Some(slot_id);
                }
            }
        }

        self.locations = location_positions;

        // set stat locations
        let north_token_id = self
            .locations
            .get(&Location::North)
            .unwrap()
            .token
            .unwrap()
            .entity_id
            .unwrap();
        if self.bot.token_slots.contains_key(&north_token_id) {
            self.bot.start_location = Location::North;
            self.opponent.start_location = Location::South;
        } else if self.opponent.token_slots.contains_key(&north_token_id) {
            self.bot.start_location = Location::South;
            self.opponent.start_location = Location::North;
        } else {
            warn!("Unable to find start locations");
        }

        self.figures = start_state.entities.figures;

        debug!("Finished initializing game info");
    }

    pub fn parse_state(&mut self, state: GameState) {
        self.current_tick = Some(state.current_tick);
        debug!("{:?}", self.current_tick.unwrap());

        // clear new squads as they are not new this tick anymore
        self.bot.new_squad_ids.clear();
        self.opponent.new_squad_ids.clear();

        // clear dead squads
        self.bot.dead_squad_ids.clear();
        self.opponent.dead_squad_ids.clear();

        // clear new power and token slots
        self.bot.new_power_slot_ids.clear();
        self.opponent.new_power_slot_ids.clear();
        self.bot.new_token_slot_ids.clear();
        self.opponent.new_token_slot_ids.clear();

        // clear destroyed power and token slots
        self.bot.destroyed_power_slot_ids.clear();
        self.opponent.destroyed_power_slot_ids.clear();
        self.bot.destroyed_token_slot_ids.clear();
        self.opponent.destroyed_token_slot_ids.clear();

        // set power for each player
        for player in &state.players {
            if player.id == self.bot.id {
                self.bot.power = player.power;
                self.bot.void_power = player.void_power;
            } else if player.id == self.opponent.id {
                self.opponent.power = player.power;
                self.opponent.void_power = player.void_power;
            }
        }

        // assign units
        for squad in state.entities.squads.iter() {
            let squad_entity_id = squad.entity.id;
            if let Some(squad_player_id) = squad.entity.player_entity_id {
                if squad_player_id == self.bot.id {
                    if let None = self.bot.squads.insert(squad_entity_id, squad.clone()) {
                        // the squad did not exist before
                        debug!("New squad {:?} was spawned for bot", squad_entity_id);
                        self.bot.new_squad_ids.push(squad_entity_id);
                    }
                } else if squad_player_id == self.opponent.id {
                    if let None = self.opponent.squads.insert(squad_entity_id, squad.clone()) {
                        // the squad did not exist before
                        debug!("New squad {:?} was spawned for opponent", squad_entity_id);
                        self.opponent.new_squad_ids.push(squad_entity_id);
                    }
                }
            } else {
                warn!("Found squad {:?} not belonging to any player", squad);
            }
        }

        // assign dead units
        for entity_id in self.bot.squads.keys() {
            let state_entity_ids: Vec<EntityId> =
                state.entities.squads.iter().map(|s| s.entity.id).collect();
            if !state_entity_ids.contains(entity_id) {
                // entity_id is not in the state anymore -> died
                self.bot.dead_squad_ids.push(*entity_id);
            }
        }
        for entity_id in self.opponent.squads.keys() {
            let state_entity_ids: Vec<EntityId> =
                state.entities.squads.iter().map(|s| s.entity.id).collect();
            if !state_entity_ids.contains(entity_id) {
                // entity_id is not in the state anymore -> died
                self.opponent.dead_squad_ids.push(*entity_id);
            }
        }

        // remove dead units
        for entity_id in self.bot.dead_squad_ids.iter() {
            if let Some(removed_entity) = self.bot.squads.remove(entity_id) {
                debug!(
                    "Removed dead squad {:?} from bot squads",
                    removed_entity.entity.id
                );
            } else {
                warn!("Did not find dead squad {:?} in bot squads", entity_id);
            }
        }
        for entity_id in self.opponent.dead_squad_ids.iter() {
            if let Some(removed_entity) = self.opponent.squads.remove(entity_id) {
                debug!(
                    "Removed dead squad {:?} from opponent squads",
                    removed_entity.entity.id
                );
            } else {
                warn!("Did not find dead squad {:?} in opponent squads", entity_id);
            }
        }

        // asign power slots
        for power_slot in state.entities.power_slots.iter() {
            let slot_id = power_slot.entity.id;
            if let Some(player_id) = power_slot.entity.player_entity_id {
                if player_id == self.bot.id {
                    if let None = self.bot.power_slots.insert(slot_id, power_slot.clone()) {
                        debug!("New power slot {:?} created for bot", slot_id);
                        self.bot.new_power_slot_ids.push(slot_id);
                    }
                } else if player_id == self.opponent.id {
                    if let None = self
                        .opponent
                        .power_slots
                        .insert(slot_id, power_slot.clone())
                    {
                        debug!("New power slot {:?} created for opponent", slot_id);
                        self.opponent.new_power_slot_ids.push(slot_id);
                    }
                }
            }
        }

        // assign token slots
        for token_slot in state.entities.token_slots.iter() {
            let slot_id = token_slot.entity.id;
            if let Some(player_id) = token_slot.entity.player_entity_id {
                if player_id == self.bot.id {
                    if let None = self.bot.token_slots.insert(slot_id, token_slot.clone()) {
                        debug!("New token slot {:?} created for bot", slot_id);
                        self.bot.new_token_slot_ids.push(slot_id);
                    }
                } else if player_id == self.opponent.id {
                    if let None = self
                        .opponent
                        .token_slots
                        .insert(slot_id, token_slot.clone())
                    {
                        debug!("New token slot {:?} created for opponent", slot_id);
                        self.opponent.new_token_slot_ids.push(slot_id);
                    }
                }
            }
        }

        // assign destroyed power slots
        for power_slot_id in self.bot.power_slots.keys() {
            for power_slot in state.entities.power_slots.iter() {
                if power_slot.entity.id == *power_slot_id
                    && power_slot.entity.player_entity_id.is_none()
                {
                    self.bot.destroyed_power_slot_ids.push(*power_slot_id);
                    break;
                }
            }
        }
        for power_slot_id in self.opponent.power_slots.keys() {
            for power_slot in state.entities.power_slots.iter() {
                if power_slot.entity.id == *power_slot_id
                    && power_slot.entity.player_entity_id.is_none()
                {
                    self.opponent.destroyed_power_slot_ids.push(*power_slot_id);
                    break;
                }
            }
        }

        // remove destroyed power slots
        for slot_id in self.bot.destroyed_power_slot_ids.iter() {
            if let Some(removed_slot) = self.bot.power_slots.remove(slot_id) {
                debug!(
                    "Removed destroyed power slot {:?} from bot",
                    removed_slot.entity.id
                );
            } else {
                warn!("Did not find destroyed power slot {:?} for bot", slot_id);
            }
        }
        for slot_id in self.opponent.destroyed_power_slot_ids.iter() {
            if let Some(removed_slot) = self.opponent.power_slots.remove(slot_id) {
                debug!(
                    "Removed destroyed power slot {:?} from opponent",
                    removed_slot.entity.id
                );
            } else {
                warn!(
                    "Did not find destroyed power slot {:?} for opponent",
                    slot_id
                );
            }
        }

        // assign destroyed token slots
        for token_slot_id in self.bot.token_slots.keys() {
            for token_slot in state.entities.token_slots.iter() {
                if token_slot.entity.id == *token_slot_id
                    && token_slot.entity.player_entity_id.is_none()
                {
                    self.bot.destroyed_token_slot_ids.push(*token_slot_id);
                    break;
                }
            }
        }
        for token_slot_id in self.opponent.token_slots.keys() {
            for token_slot in state.entities.token_slots.iter() {
                if token_slot.entity.id == *token_slot_id
                    && token_slot.entity.player_entity_id.is_none()
                {
                    self.opponent.destroyed_token_slot_ids.push(*token_slot_id);
                    break;
                }
            }
        }

        // remove destroyed token slots
        for slot_id in self.bot.destroyed_token_slot_ids.iter() {
            if let Some(removed_slot) = self.bot.token_slots.remove(slot_id) {
                debug!(
                    "Removed destroyed token slot {:?} from bot",
                    removed_slot.entity.id
                );
            } else {
                warn!("Did not find destroyed token slot {:?} for bot", slot_id);
            }
        }
        for slot_id in self.opponent.destroyed_token_slot_ids.iter() {
            if let Some(removed_slot) = self.opponent.token_slots.remove(slot_id) {
                debug!(
                    "Removed destroyed token slot {:?} from opponent",
                    removed_slot.entity.id
                );
            } else {
                warn!(
                    "Did not find destroyed token slot {:?} for opponent",
                    slot_id
                );
            }
        }

        // set figures
        self.figures = state.entities.figures;
    }

    pub fn get_enemy_squads_in_range(&self, center: &Position2D, radius: f32) -> Vec<Squad> {
        let mut enemy_squads_in_range: Vec<Squad> = vec![];
        for squad in self.opponent.squads.values() {
            let dist = utils::dist(center, &squad.entity.position.to_2d());
            if dist < radius {
                enemy_squads_in_range.push(squad.clone());
            }
        }
        enemy_squads_in_range
    }

    pub fn has_ground_presence(&self, location: &Location) -> bool {
        let loc_pos = self.locations.get(location).unwrap().position();
        for squad in self.bot.squads.values() {
            let dist = utils::dist(&loc_pos, &squad.entity.position.to_2d());
            if dist < GROUND_PRESENCE_MIN_DIST {
                return true;
            }
        }
        false
    }

    pub fn get_squad_health(&self, entity_id: &EntityId) -> (f32, f32) {
        // get the current and max health of a squad
        let squad: &Squad;
        if self.bot.squads.contains_key(entity_id) {
            squad = self.bot.squads.get(entity_id).unwrap();
        } else if self.opponent.squads.contains_key(entity_id) {
            squad = self.opponent.squads.get(entity_id).unwrap();
        } else {
            error!(
                "Unable to get health for squad {:?} as it does not exist",
                entity_id
            );
            return (0., 0.);
        }

        let mut cur_hp: f32 = 0.;
        let mut max_hp: f32 = 0.;
        let mut found: usize = 0;
        for figure_id in squad.figures.iter() {
            for figure in self.figures.iter() {
                if figure.entity.id == *figure_id {
                    found += 1;
                    let mut found_aspects = false;
                    for aspect in figure.entity.aspects.iter() {
                        match aspect {
                            Aspect::Health {
                                current_hp,
                                cap_current_max,
                            } => {
                                found_aspects = true;
                                cur_hp += current_hp;
                                max_hp += cap_current_max;
                            }
                            _ => {}
                        }
                    }
                    if !found_aspects {
                        warn!("Unable to find health aspect for squad {:?}", entity_id);
                    }
                }
            }
        }

        if found == 0 {
            error!("Unable to find any figures for squad {:?}", entity_id);
            (0., 0.)
        } else {
            (cur_hp, max_hp)
        }
    }

    pub fn get_structure_health(&self, entity_id: &EntityId) -> (f32, f32) {
        let entity: &Entity;
        if self.bot.power_slots.contains_key(&entity_id) {
            entity = &self.bot.power_slots.get(&entity_id).unwrap().entity;
        } else if self.opponent.power_slots.contains_key(&entity_id) {
            entity = &self.opponent.power_slots.get(&entity_id).unwrap().entity;
        } else if self.bot.token_slots.contains_key(&entity_id) {
            entity = &self.bot.token_slots.get(&entity_id).unwrap().entity;
        } else if self.opponent.token_slots.contains_key(&entity_id) {
            entity = &self.opponent.token_slots.get(&entity_id).unwrap().entity;
        } else {
            error!("Unable to get health for structure {entity_id:?} as it does not exist");
            return (0., 0.);
        }

        let mut found_health_aspect = false;
        let mut cur_hp: f32 = 0.;
        let mut max_hp: f32 = 0.;
        for aspect in entity.aspects.iter() {
            match aspect {
                Aspect::Health {
                    current_hp,
                    cap_current_max,
                } => {
                    found_health_aspect = true;
                    cur_hp = *current_hp;
                    max_hp = *cap_current_max;
                }
                _ => {}
            }
        }

        if !found_health_aspect {
            error!("Unable to find health aspect for power slot {entity_id:?}");
            (0., 0.)
        } else {
            (cur_hp, max_hp)
        }
    }

    pub fn power_slot_diff(&self) -> i32 {
        // Num(own power slots) - Num(opponent power slots)
        self.bot.power_slots.len() as i32 - self.opponent.power_slots.len() as i32
    }
}

impl PlayerInfo {
    pub fn get_closest_slot(&self, pos: &Position2D) -> Option<EntityId> {
        // find closest power slot
        let mut nearest_slot: Option<EntityId> = None;
        let mut current_dist: f32 = f32::INFINITY;

        for (entity_id, power_slot) in self.power_slots.iter() {
            let dist = utils::dist(&pos, &power_slot.entity.position.to_2d());
            if dist < current_dist {
                nearest_slot = Some(*entity_id);
                current_dist = dist;
            }
        }

        if nearest_slot.is_none() {
            // no power slot found, search for token slots
            for (entity_id, token_slot) in self.token_slots.iter() {
                let dist = utils::dist(&pos, &token_slot.entity.position.to_2d());
                if dist < current_dist {
                    nearest_slot = Some(*entity_id);
                    current_dist = dist;
                }
            }
        }

        if nearest_slot.is_none() {
            // this should not be possible as there would be no structure left and the game was won
            error!("Unable to find structure, this should not be possible");
            return None;
        }

        nearest_slot
    }

    pub fn bound_power(&self) -> f32 {
        let mut bound_power: f32 = 0.;
        for squad in self.squads.values() {
            bound_power += squad.bound_power;
        }
        bound_power
    }

    pub fn get_tempo(&self) -> f32 {
        // Artificial quantity "Tempo" = Free Power + Bound Power - Void Power.
        // Primarily used to compare for each player on who currently has the tempo lead.
        self.power + self.bound_power() - self.void_power
    }
}
