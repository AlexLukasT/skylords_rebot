use api::*;
use log::*;

use crate::command_scheduler::CommandScheduler;
use crate::controller::combat_controller::CombatController;
use crate::controller::spawn_controller::SpawnController;
use crate::controller::Controller;
use crate::game_info::GameInfo;
use crate::location::Location;

const CONTROL_AREA_RADIUS: f32 = 30.;

pub struct MacroState {
    pub combat_controller: CombatController,
    pub spawn_controller: SpawnController,
}

pub fn tick(
    game_info: &GameInfo,
    state: &mut MacroState,
    command_scheduler: &mut CommandScheduler,
) {
    for squad in state.spawn_controller.tick(command_scheduler, game_info) {
        state.combat_controller.add_squad(squad);
    }

    if let Some(center_owner) = state.get_center_owner(game_info) {
        // center is taken
        if center_owner == game_info.bot.id {
            // owned by me
            if game_info.bot.tempo >= game_info.opponent.tempo {
                // tempo advantage -> attack nearest slot
                info!("Attacking nearest slot");
                state.attack_closest_slot(game_info, command_scheduler);
            } else {
                // tempo disadvantage -> defend center
                info!("Defending center");
                state.defend_center(game_info);
            }
        } else {
            // owned by the opponent
            info!("Attacking center");
            state.attack_center(game_info);
        }
    } else {
        // center is not taken
        info!("Contesting center");
        state.contest_center(game_info);
    }

    let commands = state.combat_controller.tick(game_info);
    command_scheduler.schedule_commands(commands);
}

impl MacroState {
    pub fn new() -> Self {
        MacroState {
            combat_controller: CombatController::new(vec![]),
            spawn_controller: SpawnController::new(),
        }
    }

    fn attack_center(&mut self, game_info: &GameInfo) {
        let spawn_pos = self.combat_controller.get_spawn_location(game_info);

        self.spawn_controller.spawn_on_limit();
        self.spawn_controller.set_spawn_pos(spawn_pos);

        let mut target: Option<EntityId> = None;
        let center = game_info.locations.get(&Location::Center).unwrap();

        // attack power slots first
        for power_slot in &center.powers {
            if game_info
                .opponent
                .power_slots
                .contains_key(&power_slot.entity_id.unwrap())
            {
                target = power_slot.entity_id;
            }
        }

        // power slots are not taken, attack the orb
        if target.is_none() {
            if let Some(token) = center.token {
                target = token.entity_id;
            } else {
                warn!("Can not find slot token to attack");
            }
        }

        if target.is_some() {
            self.combat_controller
                .attack_slot_control(&target.unwrap(), game_info);
        } else {
            // neither one of the power wells nor the orb is taken, something is wrong
            error!("Unable to find target on center, this should not happen");
            return;
        }
    }

    fn defend_center(&mut self, game_info: &GameInfo) {
        let spawn_pos = game_info
            .locations
            .get(&Location::Center)
            .unwrap()
            .position();
        self.spawn_controller.match_opponent_spawn();
        self.spawn_controller.set_spawn_pos(spawn_pos);
        self.combat_controller.defend(&Location::Center, game_info);
    }

    fn attack_closest_slot(
        &mut self,
        game_info: &GameInfo,
        command_scheduler: &mut CommandScheduler,
    ) {
        let current_pos = self.combat_controller.get_spawn_location(game_info);

        self.spawn_controller.spawn_on_limit();
        self.spawn_controller.set_spawn_pos(current_pos);

        if let Some(nearest_slot) = game_info.opponent.get_closest_slot(&current_pos) {
            self.combat_controller
                .attack_slot_control(&nearest_slot, game_info);
            let commands = self.combat_controller.tick(game_info);
            command_scheduler.schedule_commands(commands);
        } else {
            error!("Unable to attack a slot");
        }
    }

    fn contest_center(&mut self, game_info: &GameInfo) {
        let current_pos = self.combat_controller.get_spawn_location(game_info);
        let center_pos = game_info
            .locations
            .get(&Location::Center)
            .unwrap()
            .position();

        self.spawn_controller.match_opponent_spawn();
        self.spawn_controller.set_spawn_pos(current_pos);
        self.combat_controller.control_area(
            &current_pos,
            &center_pos,
            CONTROL_AREA_RADIUS,
            game_info,
        );
    }

    fn get_center_owner(&self, game_info: &GameInfo) -> Option<EntityId> {
        let center = game_info.locations.get(&Location::Center).unwrap();

        let power_slot_ids: Vec<EntityId> =
            center.powers.iter().map(|p| p.entity_id.unwrap()).collect();

        if game_info
            .bot
            .token_slots
            .contains_key(&center.token.unwrap().entity_id.unwrap())
        {
            return Some(game_info.bot.id);
        }

        for power_slot_id in &power_slot_ids {
            if game_info.bot.power_slots.contains_key(&power_slot_id) {
                return Some(game_info.bot.id);
            }
        }

        if game_info
            .opponent
            .token_slots
            .contains_key(&center.token.unwrap().entity_id.unwrap())
        {
            return Some(game_info.opponent.id);
        }

        for power_slot_id in &power_slot_ids {
            if game_info.opponent.power_slots.contains_key(&power_slot_id) {
                return Some(game_info.opponent.id);
            }
        }

        None
    }
}
