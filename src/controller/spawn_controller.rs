use api::sr_libs::utils::card_templates::CardTemplate;
use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::*;
use log::*;

use crate::bot::*;
use crate::card_data::*;
use crate::command_scheduler::CommandScheduler;
use crate::controller::squad_controller::SquadController;
use crate::game_info::GameInfo;
use crate::utils;

// minimum difference in bound power to spawn a new squad when matching opponent's army size
const MIN_POWER_DIFF_SPAWN: f32 = 30.;

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
    tier3_offense_spawn_policy: Option<Vec<CardTemplate>>,
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
            tier3_offense_spawn_policy: None,
            in_offense: true,
        }
    }

    pub fn tick(
        &mut self,
        command_scheduler: &CommandScheduler,
        game_info: &mut GameInfo,
    ) -> Vec<SquadController> {
        // TODO: handle T2 + T3

        self.set_offense_spawn_policy(game_info);

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

    fn set_offense_spawn_policy(&mut self, game_info: &GameInfo) {
        let num_bot_token_slots = game_info.bot.token_slots.len();
        let num_opponent_token_slots = game_info.opponent.token_slots.len();

        if num_opponent_token_slots == 1 && self.tier1_offense_spawn_policy.is_none() {
            let token_slots: Vec<&TokenSlot> = game_info.opponent.token_slots.values().collect();
            let orb_color = token_slots[0].color;

            if orb_color == OrbColor::Starting {
                // wait for the first real orb
                return;
            }

            info!("Setting T1 offense spawn policy to {orb_color:?}");
            self.tier1_offense_spawn_policy =
                Some(SpawnController::get_tier1_offense_spawn_policy(orb_color));
        }

        if (num_bot_token_slots == 2 && self.tier2_offense_spawn_policy.is_none())
            || (num_opponent_token_slots == 2 && game_info.opponent.new_token_slot_ids.len() > 0)
        {
            // either me or my opponent reached T2
            if num_opponent_token_slots == 2 {
                let token_slots: Vec<&TokenSlot> =
                    game_info.opponent.token_slots.values().collect();
                let orb_colors = (token_slots[0].color, token_slots[1].color);
                info!("Setting T2 offense spawn policy to {orb_colors:?}");
                self.tier2_offense_spawn_policy =
                    Some(SpawnController::get_tier2_offense_spawn_policy(orb_colors));
            } else {
                info!("Setting T2 offense spawn policy to universal");
                self.tier2_offense_spawn_policy =
                    Some(SpawnController::get_tier2_univeral_spawn_policy());
            }
        }

        if (num_bot_token_slots == 3 && self.tier3_offense_spawn_policy.is_none())
            || (num_opponent_token_slots == 3 && game_info.opponent.new_token_slot_ids.len() > 0)
        {
            // either me or my opponent reached T3
            if num_opponent_token_slots == 3 {
                let token_slots: Vec<&TokenSlot> =
                    game_info.opponent.token_slots.values().collect();
                let orb_colors = (
                    token_slots[0].color,
                    token_slots[1].color,
                    token_slots[2].color,
                );
                info!("Setting T3 offense spawn policy to {orb_colors:?}");
                self.tier3_offense_spawn_policy =
                    Some(SpawnController::get_tier3_offense_spawn_policy(orb_colors));
            } else {
                info!("Setting T3 offense spawn policy to universal");
                self.tier3_offense_spawn_policy =
                    Some(SpawnController::get_tier3_univeral_spawn_policy());
            }
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

        if game_info.bot.token_slots.len() == 3 {
            // try T3 first
            if self.in_offense {
                // when in offense choose cards based on fixed policy
                card_policy = self.tier3_offense_spawn_policy.clone();
            } else {
                // when in defense choose cards based on enemy squads
                card_policy = Some(self.get_defense_spawn_policy(game_info, Tier::Tier3));
            }
        } else if game_info.bot.token_slots.len() == 2 {
            // T2
            if self.in_offense {
                card_policy = self.tier2_offense_spawn_policy.clone();
            } else {
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
        let defender_indices: Vec<usize> = match tier {
            Tier::Tier1 => BOT_T1_UNITS.to_vec(),
            Tier::Tier2 => BOT_T2_UNITS.to_vec(),
            Tier::Tier3 => BOT_T3_UNITS.to_vec(),
        };

        let opponent_squads: Vec<&Squad> = game_info.opponent.squads.values().collect();

        if opponent_squads.len() == 0 {
            return vec![BOT_CARDS[defender_indices[0]]];
        }

        let squad_ids: Vec<u32> = opponent_squads
            .iter()
            .map(|&s| game_info.card_data.get_card_info_from_id(s.card_id.0).id)
            .collect();
        let most_common_squad_id = utils::most_frequent_element(squad_ids);

        if let Some(squad_id) = most_common_squad_id {
            let attacker = game_info.card_data.get_card_info_from_id(squad_id);

            // best case: defender does not have matching defense type for attacker but
            // it's offense type matches
            for i in &defender_indices {
                let card_id = BOT_DECK.cards[*i];
                let defender = game_info.card_data.get_card_info_from_id(card_id.0);
                if attacker.offense_type.to_string() != defender.defense_type.to_string()
                    && attacker.defense_type.to_string() == defender.offense_type.to_string()
                {
                    return vec![BOT_CARDS[*i]];
                }
            }

            // next best case: defender has correct offense type, but not when attacker is
            // ranged and defender is melee (e.g. do not defend Sunstriders with Dreadcharger)
            for i in &defender_indices {
                let card_id = BOT_DECK.cards[*i];
                let defender = game_info.card_data.get_card_info_from_id(card_id.0);
                if attacker.defense_type.to_string() == defender.offense_type.to_string()
                    && !(!attacker.melee
                        && defender.melee
                        && attacker.offense_type.to_string() == defender.defense_type.to_string())
                {
                    return vec![BOT_CARDS[*i]];
                }
            }

            // least best case: defender does not have matching defense type
            for i in &defender_indices {
                let card_id = BOT_DECK.cards[*i];
                let defender = game_info.card_data.get_card_info_from_id(card_id.0);
                if attacker.offense_type.to_string() != defender.defense_type.to_string() {
                    return vec![BOT_CARDS[*i]];
                }
            }

            // still no defender found -> return the first one
            vec![BOT_CARDS[defender_indices[0]]]
        } else {
            warn!("Unable to find the most common attacker squad");
            return vec![BOT_CARDS[defender_indices[0]]];
        }
    }

    fn get_tier1_offense_spawn_policy(opponent_color: OrbColor) -> Vec<CardTemplate> {
        match opponent_color {
            OrbColor::Fire | OrbColor::Shadow => vec![Dreadcharger, Forsaken],
            OrbColor::Frost | OrbColor::Nature => vec![Dreadcharger, Forsaken, NoxTrooper],
            _ => vec![],
        }
    }

    fn get_tier2_univeral_spawn_policy() -> Vec<CardTemplate> {
        // used when bot is T2 but opponent is still T1
        vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
    }

    fn get_tier2_offense_spawn_policy(opponent_colors: (OrbColor, OrbColor)) -> Vec<CardTemplate> {
        match opponent_colors {
            // Pure Fire
            (OrbColor::Fire, OrbColor::Fire) => vec![Nightcrawler, LostReaverAShadow],

            // Pure Shadow
            (OrbColor::Shadow, OrbColor::Shadow) => {
                vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
            }

            // Pure Nature
            (OrbColor::Nature, OrbColor::Nature) => {
                vec![Nightcrawler, DarkelfAssassins, Nightcrawler]
            }

            // Pure Frost
            (OrbColor::Frost, OrbColor::Frost) => {
                vec![Nightcrawler, DarkelfAssassins]
            }

            // Fire Nature
            (OrbColor::Fire, OrbColor::Nature) | (OrbColor::Nature, OrbColor::Fire) => {
                vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
            }

            // Fire Shadow
            (OrbColor::Fire, OrbColor::Shadow) | (OrbColor::Shadow, OrbColor::Fire) => {
                vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
            }

            // Fire Frost
            (OrbColor::Fire, OrbColor::Frost) | (OrbColor::Frost, OrbColor::Fire) => {
                vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
            }

            // Shadow Nature
            (OrbColor::Shadow, OrbColor::Nature) | (OrbColor::Nature, OrbColor::Shadow) => {
                vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
            }

            // Shadow Frost
            (OrbColor::Shadow, OrbColor::Frost) | (OrbColor::Frost, OrbColor::Shadow) => {
                vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
            }

            // Nature Frost
            (OrbColor::Nature, OrbColor::Frost) | (OrbColor::Frost, OrbColor::Nature) => {
                vec![Nightcrawler, LostReaverAShadow, DarkelfAssassins]
            }

            _ => vec![],
        }
    }

    fn get_tier3_univeral_spawn_policy() -> Vec<CardTemplate> {
        // used when bot is T3 but opponent is still T1 or T2
        vec![SilverwindLancers, Tremor]
    }

    fn get_tier3_offense_spawn_policy(
        opponent_colors: (OrbColor, OrbColor, OrbColor),
    ) -> Vec<CardTemplate> {
        match opponent_colors {
            _ => vec![SilverwindLancers, Tremor],
        }
    }
}
