use api::sr_libs::utils::card_templates::CardTemplate;
use api::*;
use log::*;
use std::env;
use std::fs;
use std::path;

const CARD_INFO_FILE_PATH: &'static str = "data/cards.json";

pub struct CardData {
    data: serde_json::Value,
}

pub struct CardOrbRequirements {
    total: i32,
    neutral: i32,
    fire: i32,
    shadow: i32,
    nature: i32,
    frost: i32,
    fireshadow: i32,
    naturefrost: i32,
    firenature: i32,
    shadowfrost: i32,
    shadownature: i32,
    firefrost: i32,
}

impl CardData {
    pub fn new() -> Self {
        CardData {
            data: serde_json::Value::Null,
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

    pub fn get_card_power_cost(&self, card_template: &CardTemplate) -> f32 {
        if let Some(card) = self.get_card(card_template) {
            return card["powerCost"].as_array().unwrap()[3].as_f64().unwrap() as f32;
        }
        0.
    }

    pub fn get_card_orbs(&self, card_template: &CardTemplate) -> CardOrbRequirements {
        if let Some(card) = self.get_card(card_template) {
            return CardOrbRequirements {
                total: card["orbsTotal"].as_i64().unwrap() as i32,
                neutral: card["orbsNeutral"].as_i64().unwrap() as i32,
                fire: card["orbsFire"].as_i64().unwrap() as i32,
                shadow: card["orbsShadow"].as_i64().unwrap() as i32,
                nature: card["orbsNature"].as_i64().unwrap() as i32,
                frost: card["orbsFrost"].as_i64().unwrap() as i32,
                fireshadow: card["orbsFireShadow"].as_i64().unwrap() as i32,
                naturefrost: card["orbsNatureFrost"].as_i64().unwrap() as i32,
                firenature: card["orbsFireNature"].as_i64().unwrap() as i32,
                shadowfrost: card["orbsShadowFrost"].as_i64().unwrap() as i32,
                shadownature: card["orbsShadowNature"].as_i64().unwrap() as i32,
                firefrost: card["orbsFireFrost"].as_i64().unwrap() as i32,
            };
        }
        CardOrbRequirements {
            total: 0,
            neutral: 0,
            fire: 0,
            shadow: 0,
            nature: 0,
            frost: 0,
            fireshadow: 0,
            naturefrost: 0,
            firenature: 0,
            shadowfrost: 0,
            shadownature: 0,
            firefrost: 0,
        }
    }
}
