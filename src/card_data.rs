use api::sr_libs::utils::card_templates::CardTemplate;
use api::*;
use log::*;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path;
use std::str::FromStr;

use crate::game_info::PlayerInfo;

const CARD_INFO_FILE_PATH: &'static str = "data/cards.json";

pub struct CardData {
    data: serde_json::Value,
    card_info_cache: BTreeMap<u32, CardInfo>,
}

#[derive(Debug, Clone, Copy)]
pub struct CardOrbRequirements {
    total: i32,
    neutral: i32,
    fire: i32,
    shadow: i32,
    nature: i32,
    frost: i32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum CardOffenseType {
    S,
    M,
    L,
    XL,
    Special,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum CardDefenseType {
    S,
    M,
    L,
    XL,
}

#[derive(Debug, Clone, Copy)]
pub struct CardInfo {
    pub id: u32,
    pub power_cost: f32,
    pub orb_requirements: CardOrbRequirements,
    pub offense_type: CardOffenseType,
    pub defense_type: CardDefenseType,
    pub melee: bool,
    pub siege: bool,
}

impl CardInfo {
    pub fn new() -> CardInfo {
        CardInfo {
            id: 0,
            power_cost: 0.,
            orb_requirements: CardOrbRequirements {
                total: 0,
                neutral: 0,
                fire: 0,
                shadow: 0,
                nature: 0,
                frost: 0,
            },
            offense_type: CardOffenseType::S,
            defense_type: CardDefenseType::S,
            melee: false,
            siege: false,
        }
    }

    pub fn from_card_json(card: &serde_json::Value) -> CardInfo {
        CardInfo {
            id: CardInfo::get_card_id(card),
            power_cost: CardInfo::get_card_power_cost(card),
            orb_requirements: CardInfo::get_card_orbs(card),
            offense_type: CardInfo::get_card_offense_type(card),
            defense_type: CardInfo::get_card_defense_type(card),
            melee: CardInfo::get_card_melee(card),
            siege: CardInfo::get_card_siege(card),
        }
    }

    fn get_card_id(card: &serde_json::Value) -> u32 {
        let ids = card["officialCardIds"].as_array().unwrap();

        if ids.len() == 0 {
            error!("Unable to find CardId for {card:?}");
            0
        } else if ids.len() > 1 {
            warn!("Got more than one CardId for {card:?}");
            0
        } else {
            ids.get(0).unwrap().as_i64().unwrap() as u32
        }
    }

    fn get_card_power_cost(card: &serde_json::Value) -> f32 {
        card["powerCost"].as_array().unwrap()[3].as_f64().unwrap() as f32
    }

    fn get_card_orbs(card: &serde_json::Value) -> CardOrbRequirements {
        CardOrbRequirements {
            total: card["orbsTotal"].as_i64().unwrap() as i32,
            neutral: card["orbsNeutral"].as_i64().unwrap() as i32,
            fire: card["orbsFire"].as_i64().unwrap() as i32,
            shadow: card["orbsShadow"].as_i64().unwrap() as i32,
            nature: card["orbsNature"].as_i64().unwrap() as i32,
            frost: card["orbsFrost"].as_i64().unwrap() as i32,
        }
    }

    fn get_card_offense_type(card: &serde_json::Value) -> CardOffenseType {
        let index = card["offenseType"].as_i64().unwrap();
        match index {
            0 => CardOffenseType::S,
            1 => CardOffenseType::M,
            2 => CardOffenseType::L,
            3 => CardOffenseType::XL,
            4 => CardOffenseType::Special,
            _ => {
                error!("Unable to find CardOffenseType for index {index:?}");
                CardOffenseType::S
            }
        }
    }

    fn get_card_defense_type(card: &serde_json::Value) -> CardDefenseType {
        let index = card["defenseType"].as_i64().unwrap();
        match index {
            0 => CardDefenseType::S,
            1 => CardDefenseType::M,
            2 => CardDefenseType::L,
            3 => CardDefenseType::XL,
            _ => {
                error!("Unable to find CardDefenseType for index {index:?}");
                CardDefenseType::S
            }
        }
    }

    fn get_card_siege(card: &serde_json::Value) -> bool {
        let card_name = card.get("cardSlug").unwrap().as_str().unwrap();
        if card_name.to_lowercase() == "sunstriders" {
            // Sunstriders also have an ability called "Siege", but it only adds a fixed
            // 1 damage to it's attack. We only care about Siege multipliers (like Firedancer),
            // so ignore this one.
            return false;
        }

        for ability in card.get("abilities").unwrap().as_array().unwrap() {
            let ability_name = ability.get("abilityName").unwrap().as_str().unwrap();
            if ability_name == "Siege" {
                return true;
            }
        }

        false
    }

    fn get_card_melee(card: &serde_json::Value) -> bool {
        let attack_type = card.get("attackType").unwrap().as_str().unwrap();
        match attack_type {
            "0" => return true,
            "1" => return false,
            _ => {
                let card_name = card.get("cardSlug").unwrap().as_str().unwrap();
                error!("Unable to evaluate attack type {attack_type:?} for card {card_name:?}");
                return true;
            }
        }
    }
}

impl FromStr for CardOffenseType {
    type Err = ();

    fn from_str(input: &str) -> Result<CardOffenseType, Self::Err> {
        match input {
            "S" => Ok(CardOffenseType::S),
            "M" => Ok(CardOffenseType::M),
            "L" => Ok(CardOffenseType::L),
            "XL" => Ok(CardOffenseType::XL),
            "Special" => Ok(CardOffenseType::Special),
            _ => Err(()),
        }
    }
}

impl ToString for CardOffenseType {
    fn to_string(&self) -> String {
        match self {
            CardOffenseType::S => "S".to_string(),
            CardOffenseType::M => "M".to_string(),
            CardOffenseType::L => "L".to_string(),
            CardOffenseType::XL => "XL".to_string(),
            CardOffenseType::Special => "Special".to_string(),
        }
    }
}

impl FromStr for CardDefenseType {
    type Err = ();

    fn from_str(input: &str) -> Result<CardDefenseType, Self::Err> {
        match input {
            "S" => Ok(CardDefenseType::S),
            "M" => Ok(CardDefenseType::M),
            "L" => Ok(CardDefenseType::L),
            "XL" => Ok(CardDefenseType::XL),
            _ => Err(()),
        }
    }
}

impl ToString for CardDefenseType {
    fn to_string(&self) -> String {
        match self {
            CardDefenseType::S => "S".to_string(),
            CardDefenseType::M => "M".to_string(),
            CardDefenseType::L => "L".to_string(),
            CardDefenseType::XL => "XL".to_string(),
        }
    }
}

impl CardData {
    pub fn new() -> Self {
        CardData {
            data: serde_json::Value::Null,
            card_info_cache: BTreeMap::new(),
        }
    }

    pub fn get_card_info_from_id(&mut self, card_id: u32) -> CardInfo {
        if let Some(card_info) = self.card_info_cache.get(&card_id) {
            return *card_info;
        }

        let card_option = self.get_card_from_id(card_id);
        if let Some(card) = card_option {
            let card_info = CardInfo::from_card_json(card);
            self.card_info_cache.insert(card_id, card_info);
            card_info
        } else {
            CardInfo::new()
        }
    }

    pub fn card_id_without_upgrade(id: u32) -> u32 {
        if id >= (Upgrade::U3 as u32) {
            return id - (Upgrade::U3 as u32);
        }
        id
    }

    fn get_card_from_id(&self, card_id: u32) -> Option<&serde_json::Value> {
        for card in self.data["data"].as_array().unwrap() {
            let ids = card["officialCardIds"].as_array().unwrap();
            for id in ids {
                if id.as_i64().unwrap() as u32 == CardData::card_id_without_upgrade(card_id) {
                    return Some(card);
                }
            }
        }

        error!("Unable to find card with ID {card_id:?}");
        None
    }

    pub fn load(&mut self) {
        debug!("Loading card data");

        self.data = serde_json::from_str(include_str!("../data/cards.json"))
            .expect("Unable to parse cards.json file");

        debug!("Finished loading card data");
    }

    pub fn player_fullfills_orb_requirements(
        &mut self,
        card_template: &CardTemplate,
        player_info: &PlayerInfo,
    ) -> bool {
        let orb_requirements = self
            .get_card_info_from_id(card_template.id())
            .orb_requirements;

        // fire, shadow, nature, frost
        let mut num_colors: Vec<i32> = vec![0; 4];
        let mut has_starting_orb = false;
        for token_slot in player_info.token_slots.values() {
            match token_slot.color {
                OrbColor::Fire => num_colors[0] += 1,
                OrbColor::Shadow => num_colors[1] += 1,
                OrbColor::Nature => num_colors[2] += 1,
                OrbColor::Frost => num_colors[3] += 1,
                OrbColor::Starting => has_starting_orb = true,
                _ => {}
            }
        }

        // for the starting orb any t1 unit can be played
        if has_starting_orb {
            return orb_requirements.total == 1;
        }

        // subtract "hard" color requirements
        num_colors[0] -= orb_requirements.fire;
        num_colors[1] -= orb_requirements.shadow;
        num_colors[2] -= orb_requirements.nature;
        num_colors[3] -= orb_requirements.frost;

        // at least one requirement can not be fullfilled if any values is below 0
        if num_colors.iter().any(|&n| n < 0) {
            return false;
        }

        // check if any possible neutral requirements can be fullfilled
        num_colors.iter().sum::<i32>() >= orb_requirements.neutral
    }
}
