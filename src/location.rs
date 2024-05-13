use api::*;
use log::*;

use crate::game_info::GameInfo;

#[derive(Debug, PartialEq)]
pub enum Location {
    BotStartToken,
    OpponentStartToken,
    CenterToken,
}

impl Location {
    pub fn to_pos2d(&self, game_info: &GameInfo) -> Position2D {
        let dummy_pos = Position {
            x: 0.,
            y: 0.,
            z: 0.,
        }
        .to_2d();
        match self {
            Location::BotStartToken => {
                if let Some(start_token_id) = game_info.bot.start_token {
                    if let Some(start_token) = game_info.bot.token_slots.get(&start_token_id) {
                        start_token.entity.position.to_2d()
                    } else {
                        warn!("Unable to get start token for bot");
                        dummy_pos
                    }
                } else {
                    warn!("Unable to get start token id for bot");
                    dummy_pos
                }
            }
            Location::OpponentStartToken => {
                if let Some(start_token_id) = game_info.opponent.start_token {
                    if let Some(start_token) = game_info.opponent.token_slots.get(&start_token_id) {
                        start_token.entity.position.to_2d()
                    } else {
                        warn!("Unable to get start token for opponent");
                        dummy_pos
                    }
                } else {
                    warn!("Unable to get start token id for opponent");
                    dummy_pos
                }
            }
            // TODO: figure this out from the map token
            Location::CenterToken => Position2D { x: 177., y: 177. },
        }
    }
}
