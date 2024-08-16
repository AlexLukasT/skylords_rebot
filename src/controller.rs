pub mod combat_controller;
pub mod squad_controller;
use api::*;

use crate::game_info::GameInfo;

pub trait Controller {
    fn tick(&mut self, game_info: &GameInfo) -> Vec<Command>;
}
