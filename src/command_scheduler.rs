use api::sr_libs::utils::card_templates::CardTemplate;
use api::*;
use log::*;

use crate::game_info::GameInfo;

const CARD_PLAY_TICK_TIMEOUT: u32 = 10;

pub struct CommandScheduler {
    tick_last_played_card: Option<Tick>,
    waiting_for_card_spawn: bool,
    current_power: f32,
    scheduled_commands: Vec<Command>,
    current_tick: Option<Tick>,
}

impl CommandScheduler {
    pub fn new() -> CommandScheduler {
        CommandScheduler {
            tick_last_played_card: None,
            waiting_for_card_spawn: false,
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

        self.current_power = game_info.bot.power;
        self.current_tick = game_info.current_tick;
    }

    pub fn schedule_commands(&mut self, commands: Vec<Command>) {
        for command in commands {
            self.schedule_command(command);
        }
    }

    pub fn schedule_command(&mut self, command: Command) {
        match command {
            Command::ProduceSquad {
                card_position: _,
                xy: _,
            } => {
                self.waiting_for_card_spawn = true;
                self.tick_last_played_card = self.current_tick;
            }
            _ => {
                //
            }
        }
        self.scheduled_commands.push(command);
    }

    pub fn card_can_be_played(&self, card: CardTemplate) -> bool {
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

        // TODO: Fix this
        self.current_power >= 60.
    }
}
