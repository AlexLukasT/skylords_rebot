use api::*;
use log::*;
use std::collections::BTreeMap;

use crate::game_info::GameInfo;
use crate::utils;

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum Location {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
    Centernorth,
    Centersouth,
    Center,
}

#[derive(Debug, Copy, Clone)]
pub struct TokenSubLocation {
    pub position: Position2D,
    pub entity_id: Option<EntityId>,
}

#[derive(Debug, Copy, Clone)]
pub struct PowerSubLocation {
    pub position: Position2D,
    pub entity_id: Option<EntityId>,
}

#[derive(Debug)]
pub struct LocationPosition {
    pub token: Option<TokenSubLocation>,
    pub powers: Vec<PowerSubLocation>,
}

impl LocationPosition {
    pub fn position(&self) -> Position2D {
        let mut positions: Vec<Position2D> = vec![];
        if let Some(token) = self.token {
            positions.push(token.position);
        }
        let mut power_positions: Vec<Position2D> = self.powers.iter().map(|p| p.position).collect();
        positions.append(&mut power_positions);
        utils::average_pos(positions)
    }
}

pub fn get_squad_position(entity_id: EntityId, game_info: &GameInfo) -> Position2D {
    if game_info.bot.squads.contains_key(&entity_id) {
        game_info
            .bot
            .squads
            .get(&entity_id)
            .unwrap()
            .entity
            .position
            .to_2d()
    } else if game_info.opponent.squads.contains_key(&entity_id) {
        game_info
            .opponent
            .squads
            .get(&entity_id)
            .unwrap()
            .entity
            .position
            .to_2d()
    } else {
        error!("Unable to get position for entity {:?}", entity_id);
        Position2D { x: 0., y: 0. }
    }
}

pub fn get_location_positions() -> BTreeMap<Location, LocationPosition> {
    BTreeMap::from([
        (
            Location::North,
            LocationPosition {
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
        ),
        (
            Location::Northeast,
            LocationPosition {
                token: None,
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 239.8502,
                            y: 254.33868,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 233.54659,
                            y: 254.29355,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
        (
            Location::East,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D {
                        x: 292.79623,
                        y: 181.02464,
                    },
                    entity_id: None,
                }),
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 296.37576,
                            y: 176.02464,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 289.29202,
                            y: 176.02464,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
        (
            Location::Southeast,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D {
                        x: 238.3221,
                        y: 93.76264,
                    },
                    entity_id: None,
                }),
                powers: vec![PowerSubLocation {
                    position: Position2D {
                        x: 245.30684,
                        y: 87.248436,
                    },
                    entity_id: None,
                }],
            },
        ),
        (
            Location::South,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D {
                        x: 177.34,
                        y: 37.702557,
                    },
                    entity_id: None,
                }),
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 185.02084,
                            y: 37.60539,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 169.35567,
                            y: 37.71217,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
        (
            Location::Southwest,
            LocationPosition {
                token: None,
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 120.78301,
                            y: 98.97056,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 114.183,
                            y: 98.97056,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
        (
            Location::West,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D {
                        x: 72.055,
                        y: 176.0,
                    },
                    entity_id: None,
                }),
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 75.434,
                            y: 181.315,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 69.05,
                            y: 181.31488,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
        (
            Location::Northwest,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D {
                        x: 113.50901,
                        y: 261.2001,
                    },
                    entity_id: None,
                }),
                powers: vec![PowerSubLocation {
                    position: Position2D {
                        x: 106.23168,
                        y: 267.45325,
                    },
                    entity_id: None,
                }],
            },
        ),
        (
            Location::Centernorth,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D {
                        x: 176.4,
                        y: 238.6631,
                    },
                    entity_id: None,
                }),
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 183.4,
                            y: 237.1595,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 169.4,
                            y: 237.1595,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
        (
            Location::Centersouth,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D {
                        x: 176.67712,
                        y: 116.7765,
                    },
                    entity_id: None,
                }),
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 182.99454,
                            y: 117.625984,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 170.42061,
                            y: 117.62589,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
        (
            Location::Center,
            LocationPosition {
                token: Some(TokenSubLocation {
                    position: Position2D { x: 176.4, y: 177.8 },
                    entity_id: None,
                }),
                powers: vec![
                    PowerSubLocation {
                        position: Position2D {
                            x: 184.6759,
                            y: 181.14935,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 180.30853,
                            y: 185.29858,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 168.40605,
                            y: 174.49603,
                        },
                        entity_id: None,
                    },
                    PowerSubLocation {
                        position: Position2D {
                            x: 172.74492,
                            y: 169.98474,
                        },
                        entity_id: None,
                    },
                ],
            },
        ),
    ])
}
