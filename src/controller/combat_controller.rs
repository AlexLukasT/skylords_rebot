use api::*;
use log::*;

use crate::controller::squad_controller::SquadController;
use crate::game_info::GameInfo;
use crate::location::Location;
use crate::utils;

const DEFENSE_AGGRO_RADIUS: f32 = 20.;

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
    OpenFieldSkirmish,
    SlotDefense,
    SlotAttackFocus,
    SlotAttackControl,
}

impl CombatController {
    pub fn new(squads: Vec<SquadController>) -> CombatController {
        CombatController {
            state: CombatControllerState::Idling,
            commands: vec![],
            squads,
        }
    }

    pub fn defend(&mut self, location: &Location, game_info: &GameInfo) {
        if self.state != CombatControllerState::SlotDefense {
            self.enter_state(CombatControllerState::SlotDefense);
        }

        // ToDo: how to deal with attacks outside of this range, e.g. Firedancers?
        let mut enemy_squads_in_range: Vec<&Squad> = vec![];
        let location_pos = game_info.locations.get(location).unwrap().position();
        for squad in game_info.opponent.squads.values() {
            let dist = utils::dist(&location_pos, &squad.entity.position.to_2d());
            if dist < DEFENSE_AGGRO_RADIUS {
                enemy_squads_in_range.push(&squad);
            }
        }

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

    fn enter_state(&mut self, new_state: CombatControllerState) {
        info!(
            "CombatController for Squads {:?} entered state {:?}",
            self.squads, new_state
        );
        self.state = new_state;
    }
}
