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
    card_info_cache: BTreeMap<String, CardInfo>,
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
    id: i32,
    power_cost: f32,
    orb_requirements: CardOrbRequirements,
    offense_type: CardOffenseType,
    defense_type: CardDefenseType,
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

impl CardData {
    pub fn new() -> Self {
        CardData {
            data: serde_json::Value::Null,
            card_info_cache: BTreeMap::new(),
        }
    }

    pub fn get_card_info_from_id(&self, card_id: i32) -> CardInfo {
        let card_option = self.get_card_from_id(card_id);
        if let Some(card) = card_option {
            self.get_card_info_from_card(card)
        } else {
            CardInfo::new()
        }
    }

    pub fn get_card_info_card_template(&self, card_template: &CardTemplate) -> CardInfo {
        let name = card_template.name().to_lowercase().to_string();
        self.get_card_info_from_name(name)
    }

    fn get_card_info_from_name(&mut self, name: String) -> CardInfo {
        if let Some(card_info) = self.card_info_cache.get(&name) {
            return *card_info;
        }

        if let Some(card) = self.get_card_from_name(&name) {
            let card_info = self.get_card_info_from_card(card);
            self.card_info_cache.insert(name, card_info);
            card_info
        } else {
            CardInfo::new()
        }
    }

    fn get_card_info_from_card(&self, card: &serde_json::Value) -> CardInfo {
        CardInfo {
            id: self.get_card_id(card),
            power_cost: self.get_card_power_cost(card),
            orb_requirements: self.get_card_orbs(card),
            offense_type: self.get_card_offense_type(card),
            defense_type: self.get_card_defense_type(card),
        }
    }

    pub fn card_id_without_upgrade(id: i32) -> i32 {
        if id >= (Upgrade::U3 as i32) {
            return id - (Upgrade::U3 as i32);
        }
        id
    }

    fn get_card_from_id(&self, card_id: i32) -> Option<&serde_json::Value> {
        for card in self.data["data"].as_array().unwrap() {
            let ids = card["officialCardIds"].as_array().unwrap();
            for id in ids {
                if id.as_i64().unwrap() as i32 == CardData::card_id_without_upgrade(card_id) {
                    return Some(card);
                }
            }
        }

        error!("Unable to find card with ID {card_id:?}");
        None
    }

    fn get_card_from_name(&self, name: &String) -> Option<&serde_json::Value> {
        for card in self.data["data"].as_array().unwrap() {
            if card["cardSlug"].as_str().unwrap().replace("-", "") == *name {
                return Some(card);
            }
        }

        error!("Unable to find card {name:?} in card data");
        None
    }

    pub fn load(&mut self) {
        let root_dir = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let full_path = root_dir.join(CARD_INFO_FILE_PATH);
        debug!("Loading card data from {full_path:?}");

        let file = fs::File::open(full_path).expect("Unable to open cards.json file");
        let json: serde_json::Value =
            serde_json::from_reader(file).expect("Unable to parse cards.json file");
        self.data = json;

        debug!("Finished loading card data");
    }

    fn get_card_id(&self, card: &serde_json::Value) -> i32 {
        let ids = card["officialCardIds"].as_array().unwrap();

        if ids.len() == 0 {
            error!("Unable to find CardId for {card:?}");
            0
        } else if ids.len() > 1 {
            warn!("Got more than one CardId for {card:?}");
            0
        } else {
            ids.get(0).unwrap().as_i64().unwrap() as i32
        }
    }

    fn get_card_power_cost(&self, card: &serde_json::Value) -> f32 {
        card["powerCost"].as_array().unwrap()[3].as_f64().unwrap() as f32
    }

    fn get_card_orbs(&self, card: &serde_json::Value) -> CardOrbRequirements {
        CardOrbRequirements {
            total: card["orbsTotal"].as_i64().unwrap() as i32,
            neutral: card["orbsNeutral"].as_i64().unwrap() as i32,
            fire: card["orbsFire"].as_i64().unwrap() as i32,
            shadow: card["orbsShadow"].as_i64().unwrap() as i32,
            nature: card["orbsNature"].as_i64().unwrap() as i32,
            frost: card["orbsFrost"].as_i64().unwrap() as i32,
        }
    }

    fn get_card_offense_type(&self, card: &serde_json::Value) -> CardOffenseType {
        let offense_types = self.data.get("enums").unwrap().get("offenseType").unwrap();
        let index = card["offenseType"].as_i64().unwrap();
        let offense_type = offense_types
            .get(index.to_string())
            .unwrap()
            .as_str()
            .unwrap();
        CardOffenseType::from_str(offense_type).unwrap()
    }

    fn get_card_defense_type(&self, card: &serde_json::Value) -> CardDefenseType {
        let defense_types = self.data.get("enums").unwrap().get("defenseType").unwrap();
        let index = card["defenseType"].as_i64().unwrap();
        let defense_type = defense_types
            .get(index.to_string())
            .unwrap()
            .as_str()
            .unwrap();
        CardDefenseType::from_str(defense_type).unwrap()
    }

    pub fn player_fullfills_orb_requirements(
        &mut self,
        card_template: &CardTemplate,
        player_info: &PlayerInfo,
    ) -> bool {
        let orb_requirements = self.get_card_info(card_template).orb_requirements;

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
