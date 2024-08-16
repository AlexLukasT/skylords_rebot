use api::*;
use std::collections::HashMap;

use crate::utils;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Location {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
    Center,
    Centernorth,
    Centersouth,
}

#[derive(Debug, PartialEq)]
pub struct TokenSubLocation {
    pub position: Position2D,
    pub entity_id: Option<EntityId>,
}

#[derive(Debug, PartialEq)]
pub struct PowerSubLocation {
    pub position: Position2D,
    pub entity_id: Option<EntityId>,
}

#[derive(Debug)]
pub struct LocationPosition {
    pub location: Location,
    pub token: Option<TokenSubLocation>,
    pub powers: Vec<PowerSubLocation>,
}

impl LocationPosition {
    pub fn position(&self) -> Position2D {
        let mut positions: Vec<&Position2D> = vec![];
        if let Some(token) = self.token {
            positions.push(&token.position);
        }
        let mut power_positions: Vec<&Position2D> =
            self.powers.iter().map(|p| &p.position).collect();
        positions.append(&mut power_positions);
        utils::average_pos(positions)
    }
}

pub fn get_location_positions() -> HashMap<Location, LocationPosition> {
    HashMap::from([(
        Location::North,
        LocationPosition {
            location: Location::North,
            token: Some(TokenSubLocation {
                position: Position2D {
                    x: 176.4518,
                    y: 317.31332,
                },
                entity_id: None,
            }),
            powers: vec![
                PowerSubLocation {
                    position: Position2D {
                        x: 183.4518,
                        y: 317.31332,
                    },
                    entity_id: None,
                },
                PowerSubLocation {
                    position: Position2D {
                        x: 169.4518,
                        y: 317.31332,
                    },
                    entity_id: None,
                },
            ],
        },
    )])
}
