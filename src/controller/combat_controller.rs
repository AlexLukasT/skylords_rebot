use api::*;
use log::*;

use crate::controller::squad_controller::SquadController;
use crate::controller::Controller;
use crate::game_info::GameInfo;
use crate::location::*;
use crate::utils;

const DEFENSE_AGGRO_RADIUS: f32 = 30.;
const ATTACK_AGGR_RADIUS: f32 = 30.;

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
    Moving,
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

    pub fn get_spawn_location(
        &self,
        game_info: &GameInfo,
        owned_location: &Location,
    ) -> Position2D {
        let ready_squads: Vec<&SquadController> =
            self.squads.iter().filter(|s| s.initialized()).collect();
        if ready_squads.len() == 0 {
            // no squads assigned yet, return the start token location
            game_info.locations.get(owned_location).unwrap().position()
        } else {
            // return the position of the first squad
            get_squad_position(ready_squads.first().unwrap().entity_id, game_info)
        }
    }

    pub fn move_squads(&mut self, pos: Position2D, force: bool) {
        if self.state != CombatControllerState::Moving {
            self.enter_state(CombatControllerState::Moving);
        }

        for squad in &mut self.squads {
            squad.move_squad(pos, force);
        }
    }

    pub fn defend(&mut self, location: &Location, game_info: &mut GameInfo) {
        if self.state != CombatControllerState::SlotDefense {
            self.enter_state(CombatControllerState::SlotDefense);
        }

        // ToDo: how to deal with attacks outside of this range, e.g. Firedancers?
        let location_pos = game_info.locations.get(location).unwrap().position();
        let mut enemy_squads_in_range =
            game_info.get_enemy_squads_in_range(&location_pos, DEFENSE_AGGRO_RADIUS);

        if enemy_squads_in_range.len() == 0 {
            // no enemy in range -> stay close to the defending location
            for squad in &mut self.squads {
                squad.move_squad(location_pos, false);
            }
        } else if enemy_squads_in_range.len() == 1 {
            // one enemy in range -> attack that one
            for squad in &mut self.squads {
                squad.attack(&enemy_squads_in_range[0].entity.id, false);
            }
        } else {
            // multiple enemies in range -> sort them ascending by threat scores
            enemy_squads_in_range.sort_by_key(|squad| {
                utils::threat_scores_defending(&location_pos, squad, game_info)
            });

            for squad in &mut self.squads {
                let entity_id = enemy_squads_in_range[0].entity.id;
                debug!("Defend: focusing attack on {:?}", entity_id);
                squad.attack(&entity_id, false);
            }
        }
    }

    pub fn control_area(
        &mut self,
        own_pos: &Position2D,
        center: &Position2D,
        radius: f32,
        game_info: &mut GameInfo,
    ) {
        if self.state != CombatControllerState::AreaControl {
            self.enter_state(CombatControllerState::AreaControl);
        }

        if utils::dist(own_pos, center) > radius {
            // outside of the area to control -> move there first
            for squad in &mut self.squads {
                squad.move_squad(*center, false);
            }
            return;
        }

        let mut enemy_squads = game_info.get_enemy_squads_in_range(center, radius);

        if enemy_squads.len() == 0 {
            // no enemies in range -> move there
            for squad in &mut self.squads {
                squad.move_squad(*center, false);
            }
            return;
        }

        // multiple enemy squads in range -> sort them ascending by threat scores
        enemy_squads.sort_by_key(|squad| utils::threat_scores_attacking(own_pos, squad, game_info));

        for squad in &mut self.squads {
            let entity_id = enemy_squads[0].entity.id;
            squad.attack(&entity_id, false);
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
            squad.attack(entity_id, true);
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
            squad.attack(entity_id, true);
        }
    }

    pub fn attack_slot_control(&mut self, entity_id: &EntityId, game_info: &mut GameInfo) {
        // attack an enemy slot, but focus enemy squads first
        if !self.slot_is_valid_target(entity_id, game_info) {
            warn!("Can not attack {:?} with slot control", entity_id);
            return;
        }

        if self.state != CombatControllerState::AttackSlotControl {
            self.enter_state(CombatControllerState::AttackSlotControl);
        }

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
            game_info.get_enemy_squads_in_range(&slot_position, ATTACK_AGGR_RADIUS);

        if enemy_squads_in_range.len() == 0 {
            // no enemy squads in range -> attack slot directly
            for squad in &mut self.squads {
                squad.attack(&entity_id, false);
            }
        } else if enemy_squads_in_range.len() == 1 {
            // exactly one enemy squad in range -> attack it
            for squad in &mut self.squads {
                squad.attack(&enemy_squads_in_range[0].entity.id, false);
            }
        } else {
            // multiple enemy squads in range -> sort them ascending by threat scores
            enemy_squads_in_range.sort_by_key(|squad| {
                utils::threat_scores_attacking(&slot_position, squad, game_info)
            });

            for squad in &mut self.squads {
                let entity_id = enemy_squads_in_range[0].entity.id;
                squad.attack(&enemy_squads_in_range[0].entity.id, false);
            }
        }
    }

    pub fn remove_dead_and_errored_squads(&mut self, game_info: &GameInfo) {
        let mut squad_indices_to_delete: Vec<usize> = vec![];
        for (index, squad) in self.squads.iter().enumerate() {
            if game_info.bot.dead_squad_ids.contains(&squad.entity_id) || squad.has_spawn_error() {
                squad_indices_to_delete.push(index);
            }
        }

        // Sort indices in descending order to prevent issues with index shitfing
        // after removing one element.
        // Can use the unstable sort as there are no duplicate elements.
        squad_indices_to_delete.sort_unstable_by(|a, b| b.cmp(a));
        for index in squad_indices_to_delete.iter() {
            self.squads.remove(*index);
        }
    }

    pub fn has_errored_squads(&self) -> bool {
        self.squads.iter().any(|s| s.has_spawn_error())
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
        let squad_ids: Vec<EntityId> = self.squads.iter().map(|s| s.entity_id).collect();
        info!(
            "CombatController for Squads {:?} entered state {:?}",
            squad_ids, new_state
        );
        self.state = new_state;
    }
}

impl Controller for CombatController {
    fn tick(&mut self, game_info: &GameInfo) -> Vec<Command> {
        let mut commands: Vec<Command> = vec![];
        for squad in self.squads.iter_mut() {
            commands.extend(squad.tick(game_info));
        }
        commands
    }
}
