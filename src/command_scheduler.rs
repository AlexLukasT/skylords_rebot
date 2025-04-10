use api::sr_libs::utils::card_templates::CardTemplate;
use api::*;
use log::*;

use crate::card_data::CardData;
use crate::game_info::GameInfo;

const CARD_PLAY_TICK_TIMEOUT: u32 = 10;

pub struct CommandScheduler {
    tick_last_played_card: Option<Tick>,
    waiting_for_card_spawn: bool,
    waiting_for_power_slot: bool,
    token_slots_in_progress: Vec<EntityId>,
    current_power: f32,
    scheduled_commands: Vec<Command>,
    current_tick: Option<Tick>,
}

impl CommandScheduler {
    pub fn new() -> CommandScheduler {
        CommandScheduler {
            tick_last_played_card: None,
            waiting_for_card_spawn: false,
            waiting_for_power_slot: false,
            token_slots_in_progress: vec![],
            current_power: 0.,
            scheduled_commands: vec![],
            current_tick: None,
        }
    }

    pub fn get_scheduled_commands(&mut self) -> Vec<Command> {
        let new_commands = self.scheduled_commands.clone();
        self.scheduled_commands.clear();
        new_commands
    }

    pub fn update_state(&mut self, game_info: &GameInfo) {
        let new_squads = game_info.bot.new_squad_ids.len();
        if new_squads == 1 {
            // new squad was spawned
            self.waiting_for_card_spawn = false;
        } else if new_squads > 1 {
            warn!("More than 1 squad was spawned at the same time");
        }

        let new_power_slots = game_info.bot.new_power_slot_ids.len();
        if new_power_slots == 1 {
            // new power slot was created
            self.waiting_for_power_slot = false;
        } else if new_power_slots > 1 {
            warn!("More than 1 power slot was created at the same time");
        }

        let mut indices_to_remove: Vec<usize> = vec![];
        for (i, token_id) in self.token_slots_in_progress.iter().enumerate() {
            if let Some(token_slot) = game_info.bot.token_slots.get(token_id) {
                // orb was created
                match token_slot.state {
                    BuildState::Build => {
                        // orb was fully built
                        indices_to_remove.push(i);
                    }
                    _ => {}
                }
            }
        }

        // Sort indices in descending order to prevent issues with index shitfing
        // after removing one element.
        // Can use the unstable sort as there are no duplicate elements.
        indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices_to_remove {
            // remove fully built orbs
            let token_id = self.token_slots_in_progress[index];
            debug!("Token slot {:?} finished building", token_id);
            self.token_slots_in_progress.remove(index);
        }

        self.current_power = game_info.bot.power;
        self.current_tick = game_info.current_tick;
    }

    pub fn unlock_card_spawn(&mut self) {
        debug!("Spawn controller: unlocked card spawn");
        self.waiting_for_card_spawn = false;
    }

    pub fn schedule_commands(&mut self, commands: Vec<Command>) {
        for command in commands {
            self.schedule_command(command);
        }
    }

    pub fn schedule_command(&mut self, command: Command) {
        // TODO: schedule commands that require power
        match command {
            Command::ProduceSquad {
                card_position: _,
                xy: _,
            } => {
                self.waiting_for_card_spawn = true;
                self.tick_last_played_card = self.current_tick;
            }
            Command::PowerSlotBuild { slot_id: _ } => {
                self.waiting_for_power_slot = true;
            }
            Command::TokenSlotBuild {
                slot_id: entity_id,
                color: _,
            } => {
                self.token_slots_in_progress.push(entity_id);
                debug!("Token slot {:?} started building", entity_id);
            }
            _ => {
                //
            }
        }
        self.scheduled_commands.push(command);
    }

    pub fn card_can_be_played(&self, card: CardTemplate, game_info: &mut GameInfo) -> bool {
        if self.current_tick.is_none() {
            return false;
        }

        if self.waiting_for_card_spawn {
            return false;
        }

        if self.tick_last_played_card.is_some()
            && self.current_tick.unwrap().0.get()
                < self.tick_last_played_card.unwrap().0.get() + CARD_PLAY_TICK_TIMEOUT
        {
            return false;
        }

        if !game_info
            .card_data
            .player_fullfills_orb_requirements(&card, &game_info.bot)
        {
            return false;
        }

        let card_cost = game_info
            .card_data
            .get_card_info_from_id(card.id())
            .power_cost;

        self.current_power >= card_cost
    }

    pub fn power_slot_can_be_built(&self) -> bool {
        if self.waiting_for_power_slot {
            return false;
        }

        self.current_power >= 100.
    }

    pub fn waiting_for_power_slot_to_finish(&self) -> bool {
        self.waiting_for_power_slot
    }

    pub fn token_slot_can_be_built(&self, game_info: &GameInfo) -> bool {
        if self.token_slots_in_progress.len() > 0 {
            return false;
        }

        let num_token_slots = game_info.bot.token_slots.len();

        if num_token_slots == 1 {
            // advance to T2
            self.current_power >= 150.
        } else if num_token_slots == 2 {
            // advance to T3
            self.current_power >= 250.
        } else {
            // T3 is maximum
            false
        }
    }

    pub fn waiting_for_token_slot_to_finish(&self) -> bool {
        self.token_slots_in_progress.len() > 0
    }
}
