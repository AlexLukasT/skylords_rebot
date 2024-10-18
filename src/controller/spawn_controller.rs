use api::sr_libs::utils::card_templates::CardTemplate;
use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::*;
use log::*;

use crate::command_scheduler::CommandScheduler;
use crate::controller::squad_controller::SquadController;
use crate::game_info::GameInfo;

// minimum difference in bound power to spawn a new squad when matching opponent's army size
const MIN_POWER_DIFF_SPAWN: f32 = 50.;

#[derive(Debug)]
pub struct SpawnController {
    state: SpawnControllerState,
    spawn_pos: Position2D,
}

#[derive(Debug, Default, PartialEq)]
enum SpawnControllerState {
    #[default]
    Waiting,
    SingleUnit,
    SpawnMatchOpponent,
    SpawnOnLimit,
}

impl SpawnController {
    pub fn new() -> SpawnController {
        SpawnController {
            state: SpawnControllerState::Waiting,
            spawn_pos: Position2D { x: 0., y: 0. },
        }
    }

    pub fn tick(
        &mut self,
        command_scheduler: &CommandScheduler,
        game_info: &GameInfo,
    ) -> Vec<SquadController> {
        // TODO: handle T2 + T3

        let next_card: CardTemplate;
        if game_info.bot.squads.is_empty() {
            // no squads -> always start with a Dreadcharger
            next_card = Dreadcharger;
        } else {
            next_card = Forsaken;
        }
        let num_squads = game_info.bot.squads.keys().len();

        match self.state {
            SpawnControllerState::Waiting => {
                vec![]
            }

            SpawnControllerState::SingleUnit => {
                if game_info.bot.squads.len() == 0
                    && command_scheduler.card_can_be_played(next_card.clone(), game_info)
                {
                    vec![self.spawn_squad(next_card, num_squads, game_info)]
                } else {
                    vec![]
                }
            }

            SpawnControllerState::SpawnMatchOpponent => {
                let bound_power_diff =
                    game_info.opponent.bound_power() - game_info.bot.bound_power();

                if bound_power_diff >= MIN_POWER_DIFF_SPAWN
                    && command_scheduler.card_can_be_played(next_card.clone(), game_info)
                {
                    vec![self.spawn_squad(next_card, num_squads, game_info)]
                } else {
                    vec![]
                }
            }

            SpawnControllerState::SpawnOnLimit => {
                if command_scheduler.card_can_be_played(next_card.clone(), game_info) {
                    vec![self.spawn_squad(next_card, num_squads, game_info)]
                } else {
                    vec![]
                }
            }
        }
    }

    fn spawn_squad(
        &self,
        card: CardTemplate,
        num_squads: usize,
        game_info: &GameInfo,
    ) -> SquadController {
        let name = format!("{:?}{:?}", card, num_squads).to_string();
        let mut squad = SquadController::new(name);
        squad.spawn(card, self.spawn_pos, game_info);
        squad
    }

    pub fn set_spawn_pos(&mut self, spawn_pos: Position2D) {
        self.spawn_pos = spawn_pos;
    }

    pub fn spawn_single_unit(&mut self) {
        if self.state != SpawnControllerState::SingleUnit {
            self.enter_state(SpawnControllerState::SingleUnit);
        }
    }

    pub fn stop_spawn(&mut self) {
        if self.state != SpawnControllerState::Waiting {
            self.enter_state(SpawnControllerState::Waiting);
        }
    }

    pub fn match_opponent_spawn(&mut self) {
        if self.state != SpawnControllerState::SpawnMatchOpponent {
            self.enter_state(SpawnControllerState::SpawnMatchOpponent);
        }
    }

    pub fn spawn_on_limit(&mut self) {
        if self.state != SpawnControllerState::SpawnOnLimit {
            self.enter_state(SpawnControllerState::SpawnOnLimit);
        }
    }

    fn enter_state(&mut self, new_state: SpawnControllerState) {
        info!("SpawnController entered state {:?}", new_state);
        self.state = new_state;
    }
}
