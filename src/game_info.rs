use api::*;
use log::*;
use std::collections::HashMap;
use std::num::NonZeroU32;

#[derive(Debug)]
pub struct GameInfo {
    pub state: Option<GameState>,
    pub bot: PlayerInfo,
    pub opponent: PlayerInfo,
    pub current_tick: Option<Tick>,
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub id: EntityId,
    pub team: u8,
    pub power_slots: Vec<PowerSlot>,
    pub token_slots: Vec<TokenSlot>,
    pub power: f32,
    pub squads: HashMap<String, Squad>,
}

impl GameInfo {
    pub fn new() -> GameInfo {
        GameInfo {
            state: None,
            bot: PlayerInfo {
                id: EntityId(NonZeroU32::new(1).unwrap()),
                team: 0,
                power_slots: vec![],
                token_slots: vec![],
                power: 0.,
                squads: HashMap::new(),
            },
            opponent: PlayerInfo {
                id: EntityId(NonZeroU32::new(1).unwrap()),
                team: 0,
                power_slots: vec![],
                token_slots: vec![],
                power: 0.,
                squads: HashMap::new(),
            },
            current_tick: None,
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

        // find power slots for each player
        for power_slot in start_state.entities.power_slots {
            if let Some(power_slot_player_id) = power_slot.entity.player_entity_id {
                if power_slot_player_id == self.bot.id {
                    self.bot.power_slots.push(power_slot);
                } else if power_slot_player_id == self.opponent.id {
                    self.opponent.power_slots.push(power_slot);
                }
            }
        }

        // find token slots for each player
        for token_slot in start_state.entities.token_slots {
            if let Some(token_slot_player_id) = token_slot.entity.player_entity_id {
                if token_slot_player_id == self.bot.id {
                    self.bot.token_slots.push(token_slot);
                } else if token_slot_player_id == self.opponent.id {
                    self.opponent.token_slots.push(token_slot);
                }
            }
        }
    }

    pub fn parse_state(&mut self, state: GameState) {
        self.current_tick = Some(state.current_tick);

        // set power for each player
        for player in &state.players {
            if player.id == self.bot.id {
                self.bot.power = player.power;
            } else if player.id == self.opponent.id {
                self.opponent.power = player.power;
            }
        }

        // assign units
        for squad in state.entities.squads {
            if let Some(squad_player_id) = squad.entity.player_entity_id {
                if squad_player_id == self.bot.id
                    && self.bot.squads.contains_key(&squad.entity.id.0.to_string())
                {
                    self.bot.squads.insert(squad.entity.id.0.to_string(), squad);
                }
            } else {
                warn!("Found squad {:?} not belonging to any player", squad);
            }
        }
    }
}
