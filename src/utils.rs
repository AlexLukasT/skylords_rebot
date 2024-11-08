use api::*;
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

use crate::game_info::GameInfo;

pub fn dist(pos1: &Position2D, pos2: &Position2D) -> f32 {
    f32::sqrt((pos1.x - pos2.x).powi(2) + (pos1.y - pos2.y).powi(2))
}

pub fn threat_scores_defending(
    own_pos: &Position2D,
    target: &Squad,
    game_info: &mut GameInfo,
) -> (i32, OrderedFloat<f32>, OrderedFloat<f32>) {
    /*
    Return a Vector of scores which are used to sort enemy squads based on their
    threat level when defending.
    They are supposed to be sorted first by the first score, than by the second,
    and so on.
    The lowest score means the highest threat.

    1. 0: siege, 1: melee, 2: ranged
    2. 0-1: current percent health (finish low-health squads first)
    3. 0-?: distance to own position (focus closer squads)
    */
    let card_info = game_info.card_data.get_card_info_from_id(target.card_id.0);

    let attack_type_score: i32 = match (card_info.siege, card_info.melee) {
        (true, _) => 0,
        (false, true) => 1,
        (false, false) => 2,
    };

    let (cur_hp, max_hp) = game_info.get_squad_health(&target.entity.id);
    let health_score = cur_hp / max_hp;

    let dist_score = dist(own_pos, &target.entity.position.to_2d());

    (
        attack_type_score,
        OrderedFloat(health_score),
        OrderedFloat(dist_score),
    )
}

pub fn threat_scores_attacking(
    own_pos: &Position2D,
    target: &Squad,
    game_info: &mut GameInfo,
) -> (i32, OrderedFloat<f32>, OrderedFloat<f32>) {
    /*
    Return a Vector of scores which are used to sort enemy squads based on their
    threat level when attacking.
    They are supposed to be sorted first by the first score, than by the second,
    and so on.
    The lowest score means the highest threat.

    1. 0: melee, 2: ranged
    2. 0-1: current percent health (finish low-health squads first)
    3. 0-?: distance to own position (focus closer squads)
    */
    let card_info = game_info.card_data.get_card_info_from_id(target.card_id.0);

    let melee_score: i32 = match card_info.melee {
        true => 0,
        false => 1,
    };

    let (cur_hp, max_hp) = game_info.get_squad_health(&target.entity.id);
    let health_score = cur_hp / max_hp;

    let dist_score = dist(own_pos, &target.entity.position.to_2d());

    (
        melee_score,
        OrderedFloat(health_score),
        OrderedFloat(dist_score),
    )
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

pub fn most_frequent_element<T>(v: Vec<T>) -> Option<T>
where
    T: Ord,
{
    let mut counts: BTreeMap<T, usize> = BTreeMap::new();
    for el in v {
        counts.entry(el).and_modify(|curr| *curr += 1).or_insert(1);
    }

    counts.into_iter().max_by_key(|(_, v)| *v).map(|(k, _)| k)
}
