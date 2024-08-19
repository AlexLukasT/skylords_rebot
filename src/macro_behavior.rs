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
                (Status::Success, args.dt)
            }
            MacroAction::TakeCenter => {
                info!("Taking Center");
                (Status::Success, args.dt)
            }
            MacroAction::CenterOwnedByMe => {
                info!("Checking if Center is owned by me");
                (Status::Success, args.dt)
            }
            MacroAction::DefendCenter => {
                info!("Defending Center");
                (Status::Success, args.dt)
            }
            MacroAction::AttackCenter => {
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
            // new_dreadcharger.spawn(Dreadcharger, Location::BotStartToken.to_pos2d(&game_info));
            self.squad_controllers.push(new_dreadcharger);
        }

        // find the power slot closes to the center of the map
        let mut targets: Vec<&PowerSlot> = game_info.opponent.power_slots.values().collect();

        // let center_pos = Location::CenterToken.to_pos2d(&game_info);
        // targets.sort_by(|a, b| {
        //     let dist_a = utils::dist(&a.entity.position.to_2d(), &center_pos);
        //     let dist_b = utils::dist(&b.entity.position.to_2d(), &center_pos);
        //     dist_a.partial_cmp(&dist_b).unwrap()
        // });
        // let target = targets.first();
        //
        // if let Some(attack_target) = target {
        //     for squad in self.squad_controllers.iter_mut() {
        //         squad.attack(&attack_target.entity.id);
        //     }
        // } else {
        //     warn!(
        //         "Unable to find center target in {:?}",
        //         game_info.opponent.power_slots
        //     );
        // }

        for squad in self.squad_controllers.iter_mut() {
            let commands = squad.tick(game_info);
            command_scheduler.schedule_commands(commands);
        }
    }
}
