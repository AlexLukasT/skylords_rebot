use api::sr_libs::utils::card_templates::CardTemplate;
use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::sr_libs::utils::card_templates::CardType;
use api::*;
use log::*;

use crate::bot::BOT_CARDS;
use crate::bot::BOT_DECK;
use crate::card_data::*;
use crate::command_scheduler::CommandScheduler;
use crate::controller::squad_controller::SquadController;
use crate::game_info::GameInfo;
use crate::utils;

// minimum difference in bound power to spawn a new squad when matching opponent's army size
const MIN_POWER_DIFF_SPAWN: f32 = 50.;

#[derive(PartialEq)]
enum Tier {
    Tier1,
    Tier2,
    Tier3,
}

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

        let next_card = self.get_next_card(game_info);

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

            if orb_color == OrbColor::Starting {
                // wait for the first real orb
                return;
            }

            info!("Setting T1 spawn policy to {orb_color:?}");
            self.tier1_offense_spawn_policy =
                Some(SpawnController::get_tier1_offense_spawn_policy(orb_color));
        }

        if game_info.opponent.token_slots.len() == 2 && self.tier2_offense_spawn_policy.is_none() {
            let token_slots: Vec<&TokenSlot> = game_info.opponent.token_slots.values().collect();
            let orb_colors = (token_slots[0].color, token_slots[1].color);
            info!("Setting T2 spawn policy to {orb_colors:?}");
            self.tier2_offense_spawn_policy =
                Some(SpawnController::get_tier2_offense_spawn_policy(orb_colors));
        }
    }

    pub fn set_in_offense(&mut self, in_offense: bool) {
        if in_offense != self.in_offense {
            info!("SpawnController setting offense to {in_offense:?}");
        }
        self.in_offense = in_offense;
    }

    fn get_next_card(&self, game_info: &mut GameInfo) -> CardTemplate {
        let card_policy: Option<Vec<CardTemplate>>;
        if game_info.bot.token_slots.len() == 2 {
            // try T2 first
            if self.in_offense {
                // when in offense choose cards based on fixed policy
                card_policy = self.tier2_offense_spawn_policy.clone();
            } else {
                // when in defense choose cards based on enemy squads
                card_policy = Some(self.get_defense_spawn_policy(game_info, Tier::Tier2));
            }
        } else {
            // T1
            if self.in_offense {
                card_policy = self.tier1_offense_spawn_policy.clone();
            } else {
                card_policy = Some(self.get_defense_spawn_policy(game_info, Tier::Tier1));
            }
        }

        if card_policy.is_none() {
            return Dreadcharger;
        }

        let num_squads = game_info.bot.squads.len();
        if num_squads < card_policy.clone().unwrap().len() {
            // play the first cards in the policy first
            card_policy.unwrap()[num_squads]
        } else {
            // infinitely repeat the last squad in the policy
            *card_policy.unwrap().last().unwrap()
        }
    }

    fn get_defense_spawn_policy(&self, game_info: &mut GameInfo, tier: Tier) -> Vec<CardTemplate> {
        // choose defending units based on attacking ones
        let opponent_squads: Vec<&Squad> = game_info.opponent.squads.values().collect();
        let squad_offense_types: Vec<CardOffenseType> = opponent_squads
            .iter()
            .map(|&s| {
                game_info
                    .card_data
                    .get_card_info_from_id(s.card_id.0)
                    .offense_type
            })
            .collect();
        let most_common_offense_type = utils::most_frequent_element(squad_offense_types);

        let squad_defense_types: Vec<CardDefenseType> = opponent_squads
            .iter()
            .map(|&s| {
                game_info
                    .card_data
                    .get_card_info_from_id(s.card_id.0)
                    .defense_type
            })
            .collect();
        let most_common_defense_type = utils::most_frequent_element(squad_defense_types);

        // TODO: figure this out dynamically
        let defender_indices: Vec<usize> = match tier {
            Tier::Tier1 => vec![0, 1, 2],
            Tier::Tier2 => vec![12, 13, 14, 15],
            Tier::Tier3 => vec![18, 19],
        };

        if most_common_offense_type.is_some() && most_common_defense_type.is_some() {
            // best case: defender does not have matching defense type for attacker but
            // it's offense type matches
            for i in &defender_indices {
                let card_id = BOT_DECK.cards[*i];
                let defender = game_info.card_data.get_card_info_from_id(card_id.0);
                if most_common_offense_type.unwrap().to_string()
                    != defender.defense_type.to_string()
                    && most_common_defense_type.unwrap().to_string()
                        == defender.offense_type.to_string()
                {
                    return vec![BOT_CARDS[*i]];
                }
            }

            // next best case: defender has correct offense type
            for i in &defender_indices {
                let card_id = BOT_DECK.cards[*i];
                let defender = game_info.card_data.get_card_info_from_id(card_id.0);
                if most_common_defense_type.unwrap().to_string()
                    == defender.offense_type.to_string()
                {
                    return vec![BOT_CARDS[*i]];
                }
            }

            // least best case: defender does not have matching defense type
            for i in &defender_indices {
                let card_id = BOT_DECK.cards[*i];
                let defender = game_info.card_data.get_card_info_from_id(card_id.0);
                if most_common_offense_type.unwrap().to_string()
                    != defender.defense_type.to_string()
                {
                    return vec![BOT_CARDS[*i]];
                }
            }

            // still no defender found -> return the first one
            vec![BOT_CARDS[defender_indices[0]]]
        } else {
            warn!("Unable to find offense and defense type for opponent squads");
            vec![BOT_CARDS[defender_indices[0]]]
        }
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
