use api::sr_libs::utils::card_templates::CardTemplate;
use api::*;
use log::*;

use crate::bot::BOT_DECK;
use crate::card_data::CardData;
use crate::game_info::GameInfo;

const CARD_PLAY_TICK_TIMEOUT: u32 = 10;

pub struct CommandScheduler {
    tick_last_played_card: Option<Tick>,
    waiting_for_card_spawn: bool,
    waiting_for_power_slot: bool,
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

        self.current_power >= game_info.card_data.get_card_power_cost(&card)
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
}
