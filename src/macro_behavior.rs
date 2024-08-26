use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::*;
use bonsai_bt::{Action, Behavior::*, Event, Status, Timer, UpdateArgs, BT};
use log::*;

use crate::command_scheduler::CommandScheduler;
use crate::controller::squad_controller::SquadController;
use crate::controller::Controller;
use crate::game_info::GameInfo;
use crate::location::Location;
use crate::utils;

#[derive(Clone, Debug, PartialEq)]
pub enum MacroAction {
    AttackCenter,
    TakeCenter,
    CenterTaken,
    CenterOwnedByMe,
    DefendCenter,
    Wait,
}

pub struct MacroState {
    pub squad_controllers: Vec<SquadController>,
}

#[derive(Debug)]
pub struct BlackBoardData {}

pub fn tick(
    bt: &mut BT<MacroAction, BlackBoardData>,
    game_info: &GameInfo,
    timer: &mut Timer,
    state: &mut MacroState,
    command_scheduler: &mut CommandScheduler,
) -> Status {
    // have bt advance dt seconds into the future
    let dt = timer.get_dt();
    // proceed to next iteration in event loop
    let e: Event = UpdateArgs { dt }.into();

    #[rustfmt::skip]
    let status = bt.tick(&e, &mut |args: bonsai_bt::ActionArgs<Event, MacroAction>, _| {
        match *args.action {
            MacroAction::CenterTaken => {
                info!("Checking if Center is taken");
                if state.get_center_owner(game_info).is_some() {
                    return (Status::Success, args.dt);
                } else {
                    return (Status::Failure, args.dt);
                }
            }
            MacroAction::TakeCenter => {
                info!("Taking Center");
                (Status::Success, args.dt)
            }
            MacroAction::CenterOwnedByMe => {
                info!("Checking if Center is owned by me");
                if let Some(center_owner) = state.get_center_owner(game_info) {
                    if center_owner == game_info.bot.id {
                        return (Status::Success, args.dt);
                    } else {
                        return (Status::Failure, args.dt);
                    }
                } else {
                    return (Status::Failure, args.dt);
                }
            }
            MacroAction::DefendCenter => {
                info!("Defending Center");
                (Status::Success, args.dt)
            }
            MacroAction::AttackCenter => {
                info!("Attacking Center");
                state.attack_center(game_info, command_scheduler);
                (Status::Running, args.dt) 
            }
            MacroAction::Wait => {
                info!("waiting");
                (Status::Running, args.dt)
            }
        }
    });

    status.0
}

pub fn create_behavior_tree() -> BT<MacroAction, BlackBoardData> {
    let tree = Select(vec![
        Sequence(vec![
            Action(MacroAction::CenterTaken),
            Select(vec![
                Sequence(vec![
                    Action(MacroAction::CenterOwnedByMe),
                    Action(MacroAction::DefendCenter),
                ]),
                Action(MacroAction::AttackCenter),
            ]),
        ]),
        Action(MacroAction::TakeCenter),
    ]);
    let blackboard = BlackBoardData {};
    let mut bt = BT::new(tree, blackboard);
    debug!("Macro behavior tree: {:?}", bt.get_graphviz());

    bt
}

impl MacroState {
    fn attack_center(&mut self, game_info: &GameInfo, command_scheduler: &mut CommandScheduler) {
        if command_scheduler.card_can_be_played(Dreadcharger) {
            let num_squads = self.squad_controllers.len();
            let mut new_dreadcharger =
                SquadController::new(format!("Dreadcharger{}", num_squads).to_string());
            new_dreadcharger.spawn(
                Dreadcharger,
                game_info
                    .locations
                    .get(&game_info.bot.start_location)
                    .unwrap()
                    .position(),
            );
            self.squad_controllers.push(new_dreadcharger);
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
            for squad in &mut self.squad_controllers {
                squad.attack(&target.unwrap());
            }
        } else {
            // neither one of the power wells nor the orb is taken, something is wrong
            error!("Unable to find target on center, this should not happen");
            return;
        }

        for squad in self.squad_controllers.iter_mut() {
            let commands = squad.tick(game_info);
            command_scheduler.schedule_commands(commands);
        }
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
