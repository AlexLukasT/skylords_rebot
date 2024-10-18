use api::sr_libs::utils::card_templates::CardTemplate;
use api::*;
use log::*;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path;

use crate::game_info::PlayerInfo;

const CARD_INFO_FILE_PATH: &'static str = "data/cards.json";

pub struct CardData {
    data: serde_json::Value,
    power_cost_cache: BTreeMap<String, f32>,
    orb_requirement_cache: BTreeMap<String, CardOrbRequirements>,
}

#[derive(Clone, Copy)]
pub struct CardOrbRequirements {
    total: i32,
    neutral: i32,
    fire: i32,
    shadow: i32,
    nature: i32,
    frost: i32,
}

impl CardData {
    pub fn new() -> Self {
        CardData {
            data: serde_json::Value::Null,
            power_cost_cache: BTreeMap::new(),
            orb_requirement_cache: BTreeMap::new(),
        }
    }

    fn get_card(&self, card_template: &CardTemplate) -> Option<&serde_json::Value> {
        let name = card_template.name().to_lowercase();
        debug!("Searching for card name {name:?}");
        for card in self.data["data"].as_array().unwrap() {
            if card["cardSlug"].as_str().unwrap().replace("-", "") == name {
                return Some(card);
            }
        }

        warn!("Unable to find card {card_template:?} in card data");
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

    pub fn get_card_power_cost(&mut self, card_template: &CardTemplate) -> f32 {
        let name = card_template.name().to_lowercase().to_string();

        if let Some(power_cost) = self.power_cost_cache.get(&name) {
            return *power_cost;
        }

        if let Some(card) = self.get_card(card_template) {
            let power_cost = card["powerCost"].as_array().unwrap()[3].as_f64().unwrap() as f32;

            self.power_cost_cache.insert(name, power_cost);

            return power_cost;
        }

        0.
    }

    pub fn get_card_orbs(&mut self, card_template: &CardTemplate) -> CardOrbRequirements {
        let name = card_template.name().to_lowercase().to_string();

        if let Some(orb_requirements) = self.orb_requirement_cache.get(&name) {
            return *orb_requirements;
        }

        if let Some(card) = self.get_card(card_template) {
            let orb_requirements = CardOrbRequirements {
                total: card["orbsTotal"].as_f64().unwrap() as i32,
                neutral: card["orbsNeutral"].as_i64().unwrap() as i32,
                fire: card["orbsFire"].as_i64().unwrap() as i32,
                shadow: card["orbsShadow"].as_i64().unwrap() as i32,
                nature: card["orbsNature"].as_i64().unwrap() as i32,
                frost: card["orbsFrost"].as_i64().unwrap() as i32,
            };

            self.orb_requirement_cache.insert(name, orb_requirements);

            return orb_requirements;
        }

        CardOrbRequirements {
            total: 0,
            neutral: 0,
            fire: 0,
            shadow: 0,
            nature: 0,
            frost: 0,
        }
    }

    pub fn player_fullfills_orb_requirements(
        &mut self,
        card_template: &CardTemplate,
        player_info: &PlayerInfo,
    ) -> bool {
        let orb_requirements = self.get_card_orbs(card_template);

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
