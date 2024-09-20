use api::*;
use log::*;

use crate::controller::squad_controller::SquadController;
use crate::controller::Controller;
use crate::game_info::GameInfo;
use crate::location::*;
use crate::utils;

const DEFENSE_AGGRO_RADIUS: f32 = 20.;
const ATTACK_AGGR_RADIUS: f32 = 20.;

#[derive(Debug)]
pub struct CombatController {
    state: CombatControllerState,
    commands: Vec<Command>,
    squads: Vec<SquadController>,
}

#[derive(Debug, Default, PartialEq)]
enum CombatControllerState {
    #[default]
    Idling,
    SlotDefense,
    AreaControl,
    AttackSquad,
    AttackSlotFocus,
    AttackSlotControl,
}

impl CombatController {
    pub fn new(squads: Vec<SquadController>) -> CombatController {
        CombatController {
            state: CombatControllerState::Idling,
            commands: vec![],
            squads,
        }
    }

    pub fn add_squad(&mut self, squad: SquadController) {
        self.squads.push(squad);
    }

    pub fn get_squads(&self) -> &Vec<SquadController> {
        &self.squads
    }

    pub fn get_spawn_location(&self, game_info: &GameInfo) -> Position2D {
        let ready_squads: Vec<&SquadController> =
            self.squads.iter().filter(|s| s.initialized()).collect();
        if ready_squads.len() == 0 {
            // no squads assigned yet, return the start token location
            game_info
                .locations
                .get(&game_info.bot.start_location)
                .unwrap()
                .position()
        } else {
            // return the position of the first squad
            get_squad_position(ready_squads.first().unwrap().entity_id, game_info)
        }
    }

    pub fn defend(&mut self, location: &Location, game_info: &GameInfo) {
        if self.state != CombatControllerState::SlotDefense {
            self.enter_state(CombatControllerState::SlotDefense);
        }

        // ToDo: how to deal with attacks outside of this range, e.g. Firedancers?
        let location_pos = game_info.locations.get(location).unwrap().position();
        let mut enemy_squads_in_range =
            get_enemy_squads_in_range(&location_pos, DEFENSE_AGGRO_RADIUS, &game_info);

        if enemy_squads_in_range.len() == 0 {
            // no enemy in range -> stay close to the defending location
            for squad in &mut self.squads {
                squad.move_squad(game_info, location_pos);
            }
        } else if enemy_squads_in_range.len() == 1 {
            // one enemy in range -> attack that one
            for squad in &mut self.squads {
                squad.attack(&enemy_squads_in_range[0].entity.id);
            }
        } else {
            // multiple enemies in range -> attack the one with the highest threat score
            enemy_squads_in_range.sort_by(|s1, s2| {
                let threat_score1 = utils::threat_score(&location_pos, s1, true);
                let threat_score2 = utils::threat_score(&location_pos, s2, true);
                // sort in descending order
                threat_score2.partial_cmp(&threat_score1).unwrap()
            });

            for squad in &mut self.squads {
                squad.attack(&enemy_squads_in_range[0].entity.id);
            }
        }
    }

    pub fn control_area(
        &mut self,
        own_pos: &Position2D,
        center: &Position2D,
        radius: f32,
        game_info: &GameInfo,
    ) {
        if self.state != CombatControllerState::AreaControl {
            self.enter_state(CombatControllerState::AreaControl);
        }

        if utils::dist(own_pos, center) > radius {
            // outside of the area to control -> move there first
            for squad in &mut self.squads {
                squad.move_squad(game_info, *center);
            }
            return;
        }

        let mut enemy_squads = get_enemy_squads_in_range(center, radius, game_info);

        if enemy_squads.len() == 0 {
            // no enemies in range -> move there
            for squad in &mut self.squads {
                squad.move_squad(game_info, *center);
            }
            return;
        }

        // multiple enemy squads in range -> attack the one with the highest threat score
        enemy_squads.sort_by(|s1, s2| {
            let threat_score1 = utils::threat_score(own_pos, s1, false);
            let threat_score2 = utils::threat_score(own_pos, s2, false);
            // sort in descending order
            threat_score2.partial_cmp(&threat_score1).unwrap()
        });

        for squad in &mut self.squads {
            squad.attack(&enemy_squads[0].entity.id);
        }
    }

    pub fn attack_squad(&mut self, entity_id: &EntityId, game_info: &GameInfo) {
        // attack an enemy squad
        if !game_info.opponent.squads.contains_key(&entity_id) {
            warn!("Can not attack entity {:?} as it is not a squad", entity_id);
            return;
        }

        if self.state != CombatControllerState::AttackSquad {
            self.enter_state(CombatControllerState::AttackSquad);
        }

        for squad in &mut self.squads {
            squad.attack(entity_id);
        }
    }

    pub fn attack_slot_focus(&mut self, entity_id: &EntityId, game_info: &GameInfo) {
        // attack an enemy slot and ignoring enemy squads
        if !self.slot_is_valid_target(entity_id, game_info) {
            warn!("Can not attack {:?} with slot focus", entity_id);
            return;
        }

        if self.state != CombatControllerState::AttackSlotFocus {
            self.enter_state(CombatControllerState::AttackSlotFocus);
        }

        for squad in &mut self.squads {
            squad.attack(entity_id);
        }
    }

    pub fn attack_slot_control(&mut self, entity_id: &EntityId, game_info: &GameInfo) {
        // attack an enemy slot, but focus enemy squads first
        if !self.slot_is_valid_target(entity_id, game_info) {
            warn!("Can not attack {:?} with slot control", entity_id);
            return;
        }

        if self.state != CombatControllerState::AttackSlotControl {
            self.enter_state(CombatControllerState::AttackSlotControl);
        }

        // ToDo: attack enemy squad based on threat score or slot
        let slot_position: Position2D;
        if game_info.opponent.power_slots.contains_key(&entity_id) {
            slot_position = game_info
                .opponent
                .power_slots
                .get(&entity_id)
                .unwrap()
                .entity
                .position
                .to_2d();
        } else {
            slot_position = game_info
                .opponent
                .token_slots
                .get(&entity_id)
                .unwrap()
                .entity
                .position
                .to_2d();
        }
        let mut enemy_squads_in_range =
            get_enemy_squads_in_range(&slot_position, ATTACK_AGGR_RADIUS, game_info);

        if enemy_squads_in_range.len() == 0 {
            // no enemy squads in range -> attack slot directly
            for squad in &mut self.squads {
                squad.attack(&entity_id);
            }
        } else if enemy_squads_in_range.len() == 1 {
            // exactly one enemy squad in range -> attack it
            for squad in &mut self.squads {
                squad.attack(&enemy_squads_in_range[0].entity.id);
            }
        } else {
            // multiple enemy squads in range -> attack the one with the highest threat score
            enemy_squads_in_range.sort_by(|s1, s2| {
                let threat_score1 = utils::threat_score(&slot_position, s1, false);
                let threat_score2 = utils::threat_score(&slot_position, s2, false);
                // sort in descending order
                threat_score2.partial_cmp(&threat_score1).unwrap()
            });

            for squad in &mut self.squads {
                squad.attack(&enemy_squads_in_range[0].entity.id);
            }
        }
    }

    fn remove_dead_squads(&mut self, game_info: &GameInfo) {
        let mut squad_indices_to_delete: Vec<usize> = vec![];
        for (index, squad) in self.squads.iter().enumerate() {
            if game_info.bot.dead_squad_ids.contains(&squad.entity_id) {
                squad_indices_to_delete.push(index);
            }
        }

        for index in squad_indices_to_delete {
            self.squads.remove(index);
        }
    }

    fn slot_is_valid_target(&mut self, entity_id: &EntityId, game_info: &GameInfo) -> bool {
        // return true if the entity is slot owned by the opponent
        if game_info.opponent.power_slots.contains_key(&entity_id)
            && game_info.opponent.token_slots.contains_key(&entity_id)
        {
            warn!("{:?} is neither a power nor a token slot", entity_id);
            return false;
        }

        if let Some(power_slot) = game_info.opponent.power_slots.get(&entity_id) {
            let power_slot_id = power_slot.entity.player_entity_id;
            if power_slot_id.is_none()
                || (power_slot_id.is_some() && power_slot_id.unwrap() != game_info.opponent.id)
            {
                warn!("{:?} is not owned by the opponent", entity_id);
                return false;
            }
        }

        if let Some(token_slot) = game_info.opponent.token_slots.get(&entity_id) {
            let token_slot_id = token_slot.entity.player_entity_id;
            if token_slot_id.is_none()
                || (token_slot_id.is_some() && token_slot_id.unwrap() != game_info.opponent.id)
            {
                warn!("{:?} is not owned by the opponent", entity_id);
                return false;
            }
        }

        true
    }

    fn enter_state(&mut self, new_state: CombatControllerState) {
        info!(
            "CombatController for Squads {:?} entered state {:?}",
            self.squads, new_state
        );
        self.state = new_state;
    }
}

impl Controller for CombatController {
    fn tick(&mut self, game_info: &GameInfo) -> Vec<Command> {
        self.remove_dead_squads(game_info);

        let mut commands: Vec<Command> = vec![];
        for squad in self.squads.iter_mut() {
            commands.extend(squad.tick(game_info));
        }
        commands
    }
}

// I tried making this a part of the CombatController struct but couldn't do it
// because of issues with an immutable and a mutable references of self.
fn get_enemy_squads_in_range(center: &Position2D, radius: f32, game_info: &GameInfo) -> Vec<Squad> {
    let mut enemy_squads_in_range: Vec<Squad> = vec![];
    for squad in game_info.opponent.squads.values() {
        let dist = utils::dist(center, &squad.entity.position.to_2d());
        if dist < radius {
            enemy_squads_in_range.push(squad.clone());
        }
    }
    enemy_squads_in_range
}
