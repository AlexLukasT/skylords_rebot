use api::*;

pub fn dist(pos1: &Position2D, pos2: &Position2D) -> f32 {
    f32::sqrt((pos1.x - pos2.x).powi(2) + (pos1.y - pos2.y).powi(2))
}

pub fn threat_score(own_pos: &Position2D, target: &Squad, defending: bool) -> f32 {
    // calculate the threat score of an enemy entity to decide which one to attack first
    let dist = dist(own_pos, &target.entity.position.to_2d());

    let mut threat_multiplier: f32 = 1.;
    // ToDo: apply modification based on damage/health, melee/range and siege modifiers

    threat_multiplier * (15. - dist)
}

pub fn average_pos(positions: Vec<Position2D>) -> Position2D {
    let mut sum_x: f32 = 0.;
    let mut sum_y: f32 = 0.;

    for pos in positions.iter() {
        sum_x += pos.x;
        sum_y += pos.y;
    }

    Position2D {
        x: sum_x / positions.len() as f32,
        y: sum_y / positions.len() as f32,
    }
}
