use api::Position2D;

pub fn dist(pos1: &Position2D, pos2: &Position2D) -> f32 {
    f32::sqrt((pos1.x - pos2.x).powi(2) + (pos1.y - pos2.y).powi(2))
}
