use api::*;
use log::*;

use crate::controller::squad_controller::SquadController;
use crate::game_info::GameInfo;
use crate::location::Location;
use crate::utils;

const DEFENSE_AGGRO_RADIUS: f32 = 20.;
const ATTACK_AGGR_RADIUS: f32 = 20.;

#[derive(Debug)]
pub struct CombatController<'a> {
    state: CombatControllerState,
    commands: Vec<Command>,
    squads: Vec<SquadController>,
    game_info: &'a GameInfo,
}

#[derive(Debug, Default, PartialEq)]
enum CombatControllerState {
    #[default]
    Idling,
    SlotDefense,
    AttackSquad,
    AttackSlotFocus,
    AttackSlotControl,
}

impl<'a> CombatController<'a> {
    pub fn new(squads: Vec<SquadController>, game_info: &GameInfo) -> CombatController {
        CombatController {
            state: CombatControllerState::Idling,
            commands: vec![],
            squads,
            game_info,
        }
    }

    pub fn defend(&mut self, location: &Location) {
        if self.state != CombatControllerState::SlotDefense {
            self.enter_state(CombatControllerState::SlotDefense);
        }

        // ToDo: how to deal with attacks outside of this range, e.g. Firedancers?
        let location_pos = self.game_info.locations.get(location).unwrap().position();
        let mut enemy_squads_in_range =
            get_enemy_squads_in_range(&location_pos, DEFENSE_AGGRO_RADIUS, &self.game_info);

        if enemy_squads_in_range.len() == 0 {
            // no enemy in range -> stay close to the defending location
            for squad in &mut self.squads {
                squad.move_squad(self.game_info, location_pos);
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

    pub fn attack_squad(&mut self, entity_id: &EntityId) {
        // attack an enemy squad
        if !self.game_info.opponent.squads.contains_key(&entity_id) {
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

    pub fn attack_slot_focus(&mut self, entity_id: &EntityId) {
        // attack an enemy slot and ignoring enemy squads
        if !self.slot_is_valid_target(entity_id) {
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

    pub fn attack_slot_control(&mut self, entity_id: &EntityId) {
        // attack an enemy slot, but focus enemy squads first
        if !self.slot_is_valid_target(entity_id) {
            warn!("Can not attack {:?} with slot control", entity_id);
            return;
        }

        if self.state != CombatControllerState::AttackSlotControl {
            self.enter_state(CombatControllerState::AttackSlotControl);
        }

        // ToDo: attack enemy squad based on threat score or slot
        let slot_position: Position2D;
        if self.game_info.opponent.power_slots.contains_key(&entity_id) {
            slot_position = self
                .game_info
                .opponent
                .power_slots
                .get(&entity_id)
                .unwrap()
                .entity
                .position
                .to_2d();
        } else {
            slot_position = self
                .game_info
                .opponent
                .token_slots
                .get(&entity_id)
                .unwrap()
                .entity
                .position
                .to_2d();
        }
        let mut enemy_squads_in_range =
            get_enemy_squads_in_range(&slot_position, ATTACK_AGGR_RADIUS, &self.game_info);

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

    fn slot_is_valid_target(&mut self, entity_id: &EntityId) -> bool {
        // return true if the entity is slot owned by the opponent
        if !self.game_info.opponent.power_slots.contains_key(&entity_id)
            && !self.game_info.opponent.token_slots.contains_key(&entity_id)
        {
            warn!("{:?} is neither a power nor a token slot", entity_id);
            return false;
        }

        if let Some(power_slot) = self.game_info.opponent.power_slots.get(&entity_id) {
            let power_slot_id = power_slot.entity.player_entity_id;
            if power_slot_id.is_none()
                || (power_slot_id.is_some() && power_slot_id.unwrap() != self.game_info.opponent.id)
            {
                warn!("{:?} is not owned by the opponent", entity_id);
                return false;
            }
        }

        if let Some(token_slot) = self.game_info.opponent.token_slots.get(&entity_id) {
            let token_slot_id = token_slot.entity.player_entity_id;
            if token_slot_id.is_none()
                || (token_slot_id.is_some() && token_slot_id.unwrap() != self.game_info.opponent.id)
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
