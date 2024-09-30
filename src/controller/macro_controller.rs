use api::*;
use core::num::NonZeroU32;
use log::*;

use crate::command_scheduler::CommandScheduler;
use crate::controller::combat_controller::CombatController;
use crate::controller::spawn_controller::SpawnController;
use crate::controller::Controller;
use crate::game_info;
use crate::game_info::GameInfo;
use crate::location;
use crate::location::Location;
use crate::utils;

// radius around location to aggro on enemy squads
const CONTROL_AREA_AGGRO_RADIUS: f32 = 80.;
// required power before a well is built
const MIN_POWER_BUILD_WELL: f32 = 200.;
// minimum difference in tempo to consider it an advantage
const MIN_TEMPO_DIFF_ADVANTAGE: f32 = 0.;
// radius in which a location is considered under attack by enemy units
const DEFEND_LOCATION_AGGRO_RADIUS: f32 = 30.;

// locations to prioritize when ahead or even
const LOCATION_PRIOS_AHEAD: [Location; 10] = [
    Location::Center,
    Location::Centersouth,
    Location::Centernorth,
    Location::Southeast,
    Location::Southwest,
    Location::West,
    Location::East,
    Location::Northwest,
    Location::Northeast,
    Location::North,
];

// locations to prioritize when behind
const LOCATION_PRIOS_BEHIND: [Location; 10] = [
    Location::Southeast,
    Location::East,
    Location::Southwest,
    Location::West,
    Location::Centersouth,
    Location::Center,
    Location::Centernorth,
    Location::Northwest,
    Location::Northeast,
    Location::North,
];

#[derive(Default, Debug)]
enum MacroState {
    #[default]
    MatchStart,
    GroundPresenceNextLoc, // get ground presence at the next location not owned by me
    ControlArea,           // control the area by fighting enemy squds
    AttackLoc,             // attack a location
    TakeWell,              // take a power slot
    HealUnits,             // wait until all units are fully healed
    Defend,                // defend owned locations
}

pub struct MacroController {
    state: MacroState,
    focus_loc: Location,
    pub combat_controller: CombatController,
    pub spawn_controller: SpawnController,
}

impl MacroController {
    pub fn new() -> Self {
        MacroController {
            state: MacroState::MatchStart,
            focus_loc: LOCATION_PRIOS_AHEAD[0],
            combat_controller: CombatController::new(vec![]),
            spawn_controller: SpawnController::new(),
        }
    }

    pub fn tick(&mut self, game_info: &GameInfo, command_scheduler: &mut CommandScheduler) {
        if self.combat_controller.has_errored_squads() {
            // hacky -> remove spawn lock when a spawn command failed
            command_scheduler.unlock_card_spawn();
        }

        self.combat_controller
            .remove_dead_and_errored_squads(game_info);
        let current_pos = self.combat_controller.get_spawn_location(game_info);
        self.spawn_controller.set_spawn_pos(current_pos);

        match self.state {
            MacroState::MatchStart => self.run_match_start(),
            MacroState::GroundPresenceNextLoc => self.run_ground_presence_next_loc(game_info),
            MacroState::TakeWell => self.run_take_well(command_scheduler, game_info),
            MacroState::HealUnits => self.run_heal_units(game_info),
            MacroState::ControlArea => self.run_control_area(game_info),
            MacroState::AttackLoc => self.run_attack_loc(game_info),
            MacroState::Defend => self.run_defend(game_info),
        }

        for squad in self.spawn_controller.tick(command_scheduler, game_info) {
            self.combat_controller.add_squad(squad);
        }
        let squad_commands = self.combat_controller.tick(game_info);
        command_scheduler.schedule_commands(squad_commands);
    }

    fn run_match_start(&mut self) {
        self.enter_state(MacroState::GroundPresenceNextLoc);
    }

    fn run_ground_presence_next_loc(&mut self, game_info: &GameInfo) {
        self.set_focus_loc(self.get_next_focus_loc(game_info));

        let current_pos = self.combat_controller.get_spawn_location(game_info);
        let loc_pos = game_info.locations.get(&self.focus_loc).unwrap().position();
        let dist_to_loc = utils::dist(&current_pos, &loc_pos);

        let loc_owner = location::get_location_owner(&self.focus_loc, game_info);
        let is_enemy_loc = loc_owner.is_some_and(|entity_id| entity_id == game_info.opponent.id);

        let enemy_squads_in_range =
            game_info.get_enemy_squads_in_range(&loc_pos, CONTROL_AREA_AGGRO_RADIUS);

        if is_enemy_loc && dist_to_loc < CONTROL_AREA_AGGRO_RADIUS {
            // location is controlled by enemy -> attack
            self.enter_state(MacroState::AttackLoc);
            return;
        }

        if enemy_squads_in_range.len() > 0 && dist_to_loc < CONTROL_AREA_AGGRO_RADIUS {
            // approaching location and enemies nearby -> control area
            self.enter_state(MacroState::ControlArea);
            return;
        }

        if enemy_squads_in_range.len() == 0 && dist_to_loc < game_info::GROUND_PRESENCE_MIN_DIST {
            // no enemies nearby and reached location
            if game_info.power_slot_diff() < 0 || game_info.bot.power > MIN_POWER_BUILD_WELL {
                // opponent has one or more wells or bot has enough power to defend an attack
                self.enter_state(MacroState::TakeWell);
                return;
            }
        }

        self.spawn_controller.spawn_single_unit();
        self.combat_controller.move_squads(loc_pos);
    }

    fn run_take_well(&mut self, command_scheduler: &mut CommandScheduler, game_info: &GameInfo) {
        if command_scheduler.waiting_for_power_slot_to_finish() {
            // waiting for power slot to be built -> stay in this state
            return;
        }

        if game_info.bot.new_power_slot_ids.len() > 0 {
            // new power slot was built -> advance to next state
            self.enter_state(MacroState::HealUnits);
            return;
        }

        if !game_info.has_ground_presence(&self.focus_loc) {
            // no own squad nearby -> get ground presence first
            self.enter_state(MacroState::GroundPresenceNextLoc);
            return;
        }

        if command_scheduler.power_slot_can_be_built() {
            let slot_id = location::get_next_free_power_slot(&self.focus_loc, game_info);

            if slot_id.is_none() {
                // no free power well -> focus on next location
                self.enter_state(MacroState::GroundPresenceNextLoc);
                return;
            }

            let command = Command::PowerSlotBuild {
                slot_id: slot_id.unwrap(),
            };
            command_scheduler.schedule_command(command);
        }
    }

    fn run_heal_units(&mut self, game_info: &GameInfo) {
        for squad_id in game_info.bot.squads.keys() {
            let (current_health, max_health) = game_info.get_squad_health(squad_id);
            if current_health < max_health {
                return;
            }
        }

        if MacroController::tempo_advantage(game_info) {
            self.enter_state(MacroState::GroundPresenceNextLoc);
        } else {
            self.spawn_controller.stop_spawn();
            self.enter_state(MacroState::Defend);
        }
    }

    fn run_control_area(&mut self, game_info: &GameInfo) {
        let current_pos = self.combat_controller.get_spawn_location(game_info);
        let loc_pos = game_info.locations.get(&self.focus_loc).unwrap().position();

        if self.combat_controller.get_squads().len() == 0 {
            // all own squads are dead -> attack again or defend
            if MacroController::tempo_advantage(game_info) {
                self.enter_state(MacroState::GroundPresenceNextLoc);
            } else {
                self.spawn_controller.stop_spawn();
                self.enter_state(MacroState::Defend);
            }
            return;
        }

        let enemy_squads_in_range =
            game_info.get_enemy_squads_in_range(&loc_pos, CONTROL_AREA_AGGRO_RADIUS);
        if enemy_squads_in_range.len() == 0 {
            // no more enemy squads in range -> take location
            self.enter_state(MacroState::GroundPresenceNextLoc);
            return;
        }

        self.spawn_controller.match_opponent_spawn();
        self.combat_controller.control_area(
            &current_pos,
            &loc_pos,
            CONTROL_AREA_AGGRO_RADIUS,
            game_info,
        );
    }

    fn run_attack_loc(&mut self, game_info: &GameInfo) {
        if location::get_location_owner(&self.focus_loc, game_info).is_none() {
            // location is not owned by anyone anymore -> control area
            self.enter_state(MacroState::ControlArea);
            return;
        }

        if self.combat_controller.get_squads().len() == 0 {
            // all own squads are dead -> attack again or defend
            if MacroController::tempo_advantage(game_info) {
                self.enter_state(MacroState::GroundPresenceNextLoc);
            } else {
                self.spawn_controller.stop_spawn();
                self.enter_state(MacroState::Defend);
            }
            return;
        }

        self.spawn_controller.spawn_on_limit();

        let mut target: Option<EntityId> = None;
        let loc = game_info.locations.get(&self.focus_loc).unwrap();

        // attack power slots first
        for power_slot in &loc.powers {
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
            if let Some(token) = loc.token {
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
            error!(
                "Unable to find target to attack on location {:?}, this should not happen",
                self.focus_loc
            );
        }
    }

    fn run_defend(&mut self, game_info: &GameInfo) {
        if game_info.power_slot_diff() < 0 {
            // opponent has more power slots -> build another one
            self.enter_state(MacroState::TakeWell);
            return;
        }

        if MacroController::tempo_advantage(game_info) {
            // tempo advantage -> move towards next location
            self.enter_state(MacroState::GroundPresenceNextLoc);
            return;
        }

        let owned_locations: Vec<Location> = LOCATION_PRIOS_AHEAD
            .clone()
            .into_iter()
            .filter(|loc| {
                location::get_location_owner(loc, game_info)
                    .is_some_and(|id| id == game_info.bot.id)
            })
            .collect();
        let locations_under_attack: Vec<Location> = owned_locations
            .into_iter()
            .filter(|loc| {
                let loc_pos = game_info.locations.get(loc).unwrap().position();
                let enemies_in_range =
                    game_info.get_enemy_squads_in_range(&loc_pos, DEFEND_LOCATION_AGGRO_RADIUS);
                enemies_in_range.len() > 0
            })
            .collect();

        let loc_to_defend: Location;
        if locations_under_attack.len() == 0 {
            // don't spawn any new units when the opponent is not attacking a location
            self.spawn_controller.stop_spawn();
            return;
        }
        if locations_under_attack.len() == 1 {
            loc_to_defend = *locations_under_attack.first().unwrap();
        } else {
            warn!("Can not defend multiple locations, focusing on first one");
            loc_to_defend = *locations_under_attack.first().unwrap();
        }

        let loc_pos = game_info.locations.get(&loc_to_defend).unwrap().position();

        self.spawn_controller.match_opponent_spawn();
        self.spawn_controller.set_spawn_pos(loc_pos);
        self.combat_controller.defend(&loc_to_defend, game_info);
    }

    fn get_next_focus_loc(&self, game_info: &GameInfo) -> Location {
        if MacroController::tempo_advantage(game_info) {
            // find the next location owned by the opponent if tempo is good
            for loc in LOCATION_PRIOS_AHEAD[..9].iter() {
                if let Some(owner_id) = location::get_location_owner(&loc, game_info) {
                    if owner_id == game_info.opponent.id {
                        // location is owned by the opponent
                        return *loc;
                    }
                }
            }

            // TODO: implement this properly
            // 1 Tick = 10 ms -> 100 Ticks = 1s
            let five_mins_in_ticks = NonZeroU32::new(5 * 60 * 100).unwrap();
            if game_info
                .current_tick
                .is_some_and(|tick| tick.0 > five_mins_in_ticks)
            {
                // hacky: allow targetting enemy base after 5 mins
                return Location::North;
            }

            // find the location not owned by anyone
            for loc in LOCATION_PRIOS_AHEAD.iter() {
                if let None = location::get_location_owner(&loc, game_info) {
                    return *loc;
                }
            }

            error!("Unable to find next free location from ahead");
            Location::North
        } else {
            // find the location not owned by anyone
            for loc in LOCATION_PRIOS_BEHIND.iter() {
                if let None = location::get_location_owner(&loc, game_info) {
                    return *loc;
                }
            }

            error!("Unable to find next free location from behind");
            Location::North
        }
    }

    fn tempo_advantage(game_info: &GameInfo) -> bool {
        game_info.bot.get_tempo() - game_info.opponent.get_tempo() >= MIN_TEMPO_DIFF_ADVANTAGE
    }

    fn enter_state(&mut self, new_state: MacroState) {
        info!("MacroController entered state {:?}", new_state);
        self.state = new_state;
    }

    fn set_focus_loc(&mut self, new_loc: Location) {
        if new_loc != self.focus_loc {
            info!("MacroController: focussing on location {:?}", new_loc);
            self.focus_loc = new_loc;
        }
    }
}
