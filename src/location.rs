use api::*;

use crate::game_info::GameInfo;

#[derive(Debug, PartialEq)]
pub enum Location {
    BotStartToken,
    OpponentStartToken,
    CenterToken,
}

impl Location {
    pub fn to_pos2d(&self, game_info: &GameInfo) -> Position2D {
        match self {
            Location::BotStartToken => game_info.bot.token_slots[0].entity.position.to_2d(),
            Location::OpponentStartToken => {
                game_info.opponent.token_slots[0].entity.position.to_2d()
            }
            // TODO: figure this out from the map token
            Location::CenterToken => Position2D { x: 177., y: 177. },
        }
    }
}
