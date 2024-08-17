use api::*;
use log::*;
use std::collections::HashMap;
use std::num::NonZeroU32;

use crate::location::{get_location_positions, Location, LocationPosition};

#[derive(Debug)]
pub struct GameInfo {
    pub state: Option<GameState>,
    pub bot: PlayerInfo,
    pub opponent: PlayerInfo,
    pub current_tick: Option<Tick>,
    pub locations: HashMap<Location, LocationPosition>,
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub id: EntityId,
    pub team: u8,
    pub power_slots: HashMap<EntityId, PowerSlot>,
    pub token_slots: HashMap<EntityId, TokenSlot>,
    pub power: f32,
    pub void_power: f32,
    pub tempo: f32, // Power + Bound Power - Void Power
    pub squads: HashMap<EntityId, Squad>,
    pub new_squad_ids: Vec<EntityId>, // Squads that were just spawned
    pub start_token: Option<EntityId>,
}

impl GameInfo {
    pub fn new() -> GameInfo {
        GameInfo {
            state: None,
            bot: PlayerInfo {
                id: EntityId(NonZeroU32::new(1).unwrap()),
                team: 0,
                power_slots: HashMap::new(),
                token_slots: HashMap::new(),
                power: 0.,
                void_power: 0.,
                tempo: 0.,
                squads: HashMap::new(),
                new_squad_ids: vec![],
                start_token: None,
            },
            opponent: PlayerInfo {
                id: EntityId(NonZeroU32::new(1).unwrap()),
                team: 0,
                power_slots: HashMap::new(),
                token_slots: HashMap::new(),
                power: 0.,
                void_power: 0.,
                tempo: 0.,
                squads: HashMap::new(),
                new_squad_ids: vec![],
                start_token: None,
            },
            current_tick: None,
            locations: HashMap::new(),
        }
    }

    pub fn init(&mut self, start_state: GameStartState) {
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
            info!(
                "Got power slot {:?} at {:?}",
                slot_id,
                power_slot.entity.position.to_2d()
            );

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
                        info!(
                            "Assigned power slot {:?} to location {:?}.{:?}",
                            slot_id, location, i
                        );
                    }
                }
            }

            if let Some(location) = found_location {
                location_positions.entry(location).and_modify(|pos| {
                    pos.powers[found_power_index.unwrap()].entity_id = found_entity_id
                });
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
            info!(
                "Got token slot {:?} at {:?}",
                slot_id,
                token_slot.entity.position.to_2d()
            );

            // assign token slot to it's location
            let mut found_location: Option<Location> = None;
            let mut found_entity_id: Option<EntityId> = None;
            for (location, location_pos) in location_positions.iter() {
                if let Some(pos_token_slot) = location_pos.token {
                    let power_slot_x = token_slot.entity.position.to_2d().x;
                    let power_slot_y = token_slot.entity.position.to_2d().y;
                    if power_slot_x == pos_token_slot.position.x
                        && power_slot_y == pos_token_slot.position.y
                    {
                        // set the entity id in location_positions
                        found_location = Some(*location);
                        found_entity_id = Some(slot_id);
                        info!(
                            "Assigned token slot {:?} to location {:?}",
                            slot_id, location
                        );
                    }
                }
            }

            if let Some(location) = found_location {
                location_positions
                    .entry(location)
                    .and_modify(|pos| pos.token.unwrap().entity_id = found_entity_id);
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
    }

    pub fn parse_state(&mut self, state: GameState) {
        self.current_tick = Some(state.current_tick);
        debug!("{:?}", self.current_tick.unwrap());

        // clear new squads as they are not new this tick anymore
        self.bot.new_squad_ids.clear();
        self.opponent.new_squad_ids.clear();

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
        for squad in state.entities.squads {
            let squad_entity_id = squad.entity.id;
            if let Some(squad_player_id) = squad.entity.player_entity_id {
                if squad_player_id == self.bot.id {
                    if let None = self.bot.squads.insert(squad_entity_id, squad) {
                        // the squad did not exist before
                        debug!("New squad {:?} was spawned for bot", squad_entity_id);
                        self.bot.new_squad_ids.push(squad_entity_id);
                    }
                } else if squad_player_id == self.opponent.id {
                    if let None = self.opponent.squads.insert(squad_entity_id, squad) {
                        // the squad did not exist before
                        debug!("New squad {:?} was spawned for opponent", squad_entity_id);
                        self.opponent.new_squad_ids.push(squad_entity_id);
                    }
                }
            } else {
                warn!("Found squad {:?} not belonging to any player", squad);
            }
        }

        // asign power slots
        for power_slot in state.entities.power_slots {
            let slot_id = power_slot.entity.id;
            if let Some(player_id) = power_slot.entity.player_entity_id {
                if player_id == self.bot.id {
                    if let None = self.bot.power_slots.insert(slot_id, power_slot) {
                        debug!("New power slot {:?} created for bot", slot_id);
                    }
                } else if player_id == self.opponent.id {
                    if let None = self.opponent.power_slots.insert(slot_id, power_slot) {
                        debug!("New power slot {:?} created for opponent", slot_id);
                    }
                }
            }
        }

        // assign token slots
        for token_slot in state.entities.token_slots {
            let slot_id = token_slot.entity.id;
            if let Some(player_id) = token_slot.entity.player_entity_id {
                if player_id == self.bot.id {
                    if let None = self.bot.token_slots.insert(slot_id, token_slot) {
                        debug!("New token slot {:?} created for bot", slot_id);
                    }
                } else if player_id == self.opponent.id {
                    if let None = self.opponent.token_slots.insert(slot_id, token_slot) {
                        debug!("New token slot {:?} created for opponent", slot_id);
                    }
                }
            }
        }

        // calculate and set tempo for each player
        self.bot.tempo = get_tempo(&self.bot);
        self.opponent.tempo = get_tempo(&self.opponent);

        // TODO: handle killed units
    }
}

fn get_tempo(player: &PlayerInfo) -> f32 {
    // Artificial quantity "Tempo" = Free Power + Bound Power - Void Power.
    // Primarily used to compare for each player on who currently has the tempo lead.
    let mut bound_power: f32 = 0.;
    for squad in player.squads.values() {
        bound_power += squad.bound_power;
    }
    player.power + bound_power - player.void_power
}
