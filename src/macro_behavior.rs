use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::*;
use log::*;

use crate::command_scheduler::CommandScheduler;
use crate::controller::combat_controller::CombatController;
use crate::controller::squad_controller::SquadController;
use crate::controller::Controller;
use crate::game_info::GameInfo;
use crate::location::Location;

pub struct MacroState {
    pub combat_controller: CombatController,
}

pub fn tick(
    game_info: &GameInfo,
    state: &mut MacroState,
    command_scheduler: &mut CommandScheduler,
) {
    let center_owner_opt = state.get_center_owner(game_info);
    if let Some(center_owner) = center_owner_opt {
        // center is taken
        if center_owner == game_info.bot.id {
            // owned by me
        } else {
            // owned by the opponent
            info!("Attacking center");
            state.attack_center(game_info, command_scheduler);
        }
    } else {
        // center is not taken
    }
}

impl MacroState {
    pub fn new() -> Self {
        MacroState {
            combat_controller: CombatController::new(vec![]),
        }
    }
    fn attack_center(&mut self, game_info: &GameInfo, command_scheduler: &mut CommandScheduler) {
        let num_squads = self.combat_controller.get_squads().len();

        if num_squads == 0 && command_scheduler.card_can_be_played(Dreadcharger) {
            // first squad is always Dreadcharger
            let mut new_dreadcharger =
                SquadController::new(format!("Dreadcharger{}", num_squads).to_string());
            new_dreadcharger.spawn(
                Dreadcharger,
                self.combat_controller.get_spawn_location(game_info),
            );
            self.combat_controller.add_squad(new_dreadcharger);
        } else if num_squads > 0 && command_scheduler.card_can_be_played(Forsaken) {
            // subsequent squads are always Forsaken
            let mut new_forsaken =
                SquadController::new(format!("Forsaken{}", num_squads).to_string());
            new_forsaken.spawn(
                Forsaken,
                self.combat_controller.get_spawn_location(game_info),
            );
            self.combat_controller.add_squad(new_forsaken);
        }

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

        let commands = self.combat_controller.tick(game_info);
        command_scheduler.schedule_commands(commands);
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
