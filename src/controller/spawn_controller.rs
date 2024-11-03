use api::sr_libs::utils::card_templates::CardTemplate;
use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::*;
use log::*;
use sr_libs::cff::card::Card;

use crate::command_scheduler::CommandScheduler;
use crate::controller::squad_controller::SquadController;
use crate::game_info::GameInfo;
use crate::card_data::*;

// minimum difference in bound power to spawn a new squad when matching opponent's army size
const MIN_POWER_DIFF_SPAWN: f32 = 50.;

#[derive(Debug)]
pub struct SpawnController {
    state: SpawnControllerState,
    spawn_pos: Position2D,
    tier1_offense_spawn_policy: Option<Vec<CardTemplate>>,
    tier2_offense_spawn_policy: Option<Vec<CardTemplate>>,
    in_offense: bool,
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
            tier1_offense_spawn_policy: None,
            tier2_offense_spawn_policy: None,
            in_offense: true,
        }
    }

    pub fn tick(
        &mut self,
        command_scheduler: &CommandScheduler,
        game_info: &mut GameInfo,
    ) -> Vec<SquadController> {
        // TODO: handle T2 + T3

        self.set_spawn_policy(game_info);

        let next_card = self.get_next_card();

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

    fn set_spawn_policy(&mut self, game_info: &GameInfo) {
        if game_info.opponent.token_slots.len() == 1 && self.tier1_offense_spawn_policy.is_none() {
            let token_slots: Vec<&TokenSlot> = game_info.opponent.token_slots.values().collect();
            let orb_color = token_slots[0].color;
            info!("Setting T1 spawn policy to {orb_color:?}");
            self.tier1_offense_spawn_policy =
                Some(SpawnController::get_tier1_offense_spawn_policy(orb_color));
        }

        if game_info.opponent.token_slots.len() == 2 && self.tier2_offense_spawn_policy.is_none() {
            let token_slots: Vec<&TokenSlot> = game_info.opponent.token_slots.values().collect();
            let orb_colors = (token_slots[0].color, token_slots[1].color);
            info!("Setting T2 spawn policy to {orb_colors:?}");
            self.tier1_offense_spawn_policy =
                Some(SpawnController::get_tier1_offense_spawn_policy(orb_colors));
        }
    }

    pub fn set_in_offense(&mut self, in_offense: bool) {
        self.in_offense = in_offense;
    }

    fn get_next_card(&self, game_info: &GameInfo) -> CardTemplate {
        let card_policy: Vec<CardTemplate>;
        if game_info.bot.token_slots.len() == 2 {
            // try T2 first
            if self.in_offense {
                // when in offense choose cards based on fixed policy
                card_policy = self.tier2_offense_spawn_policy;
            } else {
                // when in defense choose cards based on enemy squads
                card_policy = vec![];
            }
        } else {
            // T1
            if self.in_offense {
                card_policy = self.tier1_offense_spawn_policy;
            } else {
                card_policy = vec![];
            }
        }
    }

    fn get_t1_defense_spawn_policy(&self, game_info: &GameInfo) {
        // choose defending units based on attacking ones
        let opponent_squads: Vec<&Squad> = game_info.opponent.squads.values().collect();
        let squad_offense_types: Vec<CardOffenseType> = opponent_squads.iter().map(
            |&s| CardId::
        )
    }

    fn get_tier1_offense_spawn_policy(opponent_color: OrbColor) -> Vec<CardTemplate> {
        match opponent_color {
            OrbColor::Fire | OrbColor::Shadow => vec![Dreadcharger, Forsaken],
            OrbColor::Frost | OrbColor::Nature => vec![Dreadcharger, Forsaken, NoxTrooper],
            _ => vec![],
        }
    }

    fn get_tier2_offense_spawn_policy(opponent_colors: (OrbColor, OrbColor)) -> Vec<CardTemplate> {
        match opponent_colors {
            // Pure Fire
            (OrbColor::Fire, OrbColor::Fire) => vec![Nightcrawler, AmiiPhantom],

            // Pure Shadow
            (OrbColor::Shadow, OrbColor::Shadow) => {
                vec![Nightcrawler, DarkelfAssassins, AmiiPhantom]
            }

            // Pure Nature
            (OrbColor::Nature, OrbColor::Nature) => {
                vec![Burrower, DarkelfAssassins]
            }

            // Pure Frost
            (OrbColor::Frost, OrbColor::Frost) => {
                vec![AmiiPaladins, DarkelfAssassins]
            }

            // Fire Nature
            (OrbColor::Fire, OrbColor::Nature) | (OrbColor::Nature, OrbColor::Fire) => {
                vec![Nightcrawler, AmiiPhantom]
            }

            // Fire Shadow
            (OrbColor::Fire, OrbColor::Shadow) | (OrbColor::Shadow, OrbColor::Fire) => {
                vec![Nightcrawler, AmiiPhantom]
            }

            // Fire Frost
            (OrbColor::Fire, OrbColor::Frost) | (OrbColor::Frost, OrbColor::Fire) => {
                vec![Nightcrawler, DarkelfAssassins]
            }

            // Shadow Nature
            (OrbColor::Shadow, OrbColor::Nature) | (OrbColor::Nature, OrbColor::Shadow) => {
                vec![Nightcrawler, AmiiPhantom]
            }

            // Shadow Frost
            (OrbColor::Shadow, OrbColor::Frost) | (OrbColor::Frost, OrbColor::Shadow) => {
                vec![Nightcrawler, DarkelfAssassins, AmiiPhantom]
            }

            // Nature Frost
            (OrbColor::Nature, OrbColor::Frost) | (OrbColor::Frost, OrbColor::Nature) => {
                vec![Nightcrawler, AmiiPhantom]
            }

            _ => vec![],
        }
    }
}
