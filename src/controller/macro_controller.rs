use api::*;
use log::*;

use crate::bot::BOT_ORBS;
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
const CONTROL_AREA_AGGRO_RADIUS: f32 = 60.;
// required power before a well is built
const MIN_POWER_BUILD_WELL: f32 = 200.;
// minimum difference in tempo to consider it an advantage
const MIN_TEMPO_DIFF_ADVANTAGE: f32 = 0.;
// radius in which a location is considered under attack by enemy units
const DEFEND_LOCATION_AGGRO_RADIUS: f32 = 50.;
// difference in number of squads to focus a well or orb instead of enemy squads
const NUM_SQUADS_CRITICAL_MASS: i32 = 6;

// locations to prioritize when ahead or even
const LOCATION_PRIOS_AHEAD_SOUTH_START: [Location; 9] = [
    Location::South,
    Location::Center,
    Location::Centersouth,
    Location::Centernorth,
    Location::Southeast,
    // Location::Southwest,
    Location::East,
    Location::West,
    Location::Northwest,
    // Location::Northeast,
    Location::North,
];
const LOCATION_PRIOS_AHEAD_NORTH_START: [Location; 9] = [
    Location::North,
    Location::Center,
    Location::Centernorth,
    Location::Centersouth,
    Location::Northwest,
    // Location::Northeast,
    Location::West,
    Location::East,
    Location::Southeast,
    // Location::Southwest,
    Location::South,
];

// locations to prioritize when behind
const LOCATION_PRIOS_BEHIND_SOUTH_START: [Location; 9] = [
    Location::South,
    Location::Southeast,
    Location::East,
    // Location::Southwest,
    Location::West,
    Location::Centersouth,
    Location::Center,
    Location::Centernorth,
    Location::Northwest,
    // Location::Northeast,
    Location::North,
];
const LOCATION_PRIOS_BEHIND_NORTH_START: [Location; 9] = [
    Location::North,
    Location::Northwest,
    Location::West,
    // Location::Northeast,
    Location::East,
    Location::Centernorth,
    Location::Center,
    Location::Centersouth,
    Location::Southeast,
    // Location::Southwest,
    Location::South,
];

#[derive(Default, Debug)]
enum MacroState {
    #[default]
    MatchStart,
    GroundPresenceNextLoc, // get ground presence at the next location not owned by me
    ControlArea,           // control the area by fighting enemy squds
    AttackLoc,             // attack a location
    TakeWell,              // take a power slot
    AdvanceTier,           // take a token slot
    HealUnits,             // wait until all units are fully healed
    Defend,                // defend owned locations
}

pub struct MacroController {
    state: MacroState,
    attack_focus_loc: Location,
    latest_owning_loc: Location,
    owning_loc_history: Vec<Location>,
    pub combat_controller: CombatController,
    pub spawn_controller: SpawnController,
}

impl MacroController {
    pub fn new() -> Self {
        MacroController {
            state: MacroState::MatchStart,
            attack_focus_loc: Location::Center,
            latest_owning_loc: Location::Center,
            owning_loc_history: vec![],
            combat_controller: CombatController::new(vec![]),
            spawn_controller: SpawnController::new(),
        }
    }

    pub fn tick(&mut self, game_info: &mut GameInfo, command_scheduler: &mut CommandScheduler) {
        if self.combat_controller.has_errored_squads() {
            // hacky -> remove spawn lock when a spawn command failed
            command_scheduler.unlock_card_spawn();
        }

        self.combat_controller
            .remove_dead_and_errored_squads(game_info);
        self.handle_destroyed_slots(game_info);
        let current_pos = self
            .combat_controller
            .get_spawn_location(game_info, &self.latest_owning_loc);
        self.spawn_controller.set_spawn_pos(current_pos);

        MacroController::repair_structures(game_info, command_scheduler);

        match self.state {
            MacroState::MatchStart => self.run_match_start(game_info),
            MacroState::GroundPresenceNextLoc => self.run_ground_presence_next_loc(game_info),
            MacroState::TakeWell => self.run_take_well(command_scheduler, game_info),
            MacroState::AdvanceTier => self.run_advance_tier(command_scheduler, game_info),
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

    fn run_match_start(&mut self, game_info: &GameInfo) {
        self.set_latest_owning_loc(game_info.bot.start_location);
        self.enter_state(MacroState::GroundPresenceNextLoc);
    }

    fn run_ground_presence_next_loc(&mut self, game_info: &GameInfo) {
        if self.get_locations_under_attack(game_info).len() > 0 {
            self.enter_state(MacroState::Defend);
            return;
        }

        if game_info.seconds_have_passed(180) && game_info.bot.token_slots.len() == 1 {
            self.enter_state(MacroState::AdvanceTier);
            return;
        }

        if game_info.seconds_have_passed(420) && game_info.bot.token_slots.len() == 2 {
            self.enter_state(MacroState::AdvanceTier);
            return;
        }

        self.spawn_controller.set_in_offense(true);
        self.set_attack_focus_loc(self.get_next_attack_focus_loc(game_info));

        let current_pos = self
            .combat_controller
            .get_spawn_location(game_info, &self.latest_owning_loc);
        let loc_pos = game_info
            .locations
            .get(&self.attack_focus_loc)
            .unwrap()
            .position();
        let dist_to_loc = utils::dist(&current_pos, &loc_pos);

        let loc_owner = location::get_location_owner(&self.attack_focus_loc, game_info);
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
            if game_info.token_slot_diff() < 0 {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.seconds_have_passed(180) && game_info.bot.token_slots.len() == 1 {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.seconds_have_passed(420) && game_info.bot.token_slots.len() == 2 {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.power_slot_diff() < 0 || game_info.bot.power > MIN_POWER_BUILD_WELL {
                // opponent has one or more wells or bot has enough power to defend an attack
                self.enter_state(MacroState::TakeWell);
                return;
            }
        }

        self.spawn_controller.spawn_single_unit();
        self.combat_controller.move_squads(loc_pos, false);
    }

    fn run_take_well(&mut self, command_scheduler: &mut CommandScheduler, game_info: &GameInfo) {
        if self.get_locations_under_attack(game_info).len() > 0 {
            self.enter_state(MacroState::Defend);
            return;
        }

        if command_scheduler.waiting_for_power_slot_to_finish() {
            // waiting for power slot to be built -> stay in this state
            return;
        }

        if game_info.bot.new_power_slot_ids.len() > 0 {
            // new power slot was built -> advance to next state
            self.enter_state(MacroState::HealUnits);
            return;
        }

        if command_scheduler.power_slot_can_be_built() {
            let offense_slot_id =
                location::get_next_free_power_slot(&self.attack_focus_loc, game_info);

            if offense_slot_id.is_some() && game_info.has_ground_presence(&self.attack_focus_loc) {
                let command = Command::PowerSlotBuild {
                    slot_id: offense_slot_id.unwrap(),
                };
                command_scheduler.schedule_command(command);
                self.set_latest_owning_loc(self.attack_focus_loc);
                return;
            }

            let defense_slot_id =
                location::get_next_free_power_slot(&self.latest_owning_loc, game_info);

            if defense_slot_id.is_some() {
                let command = Command::PowerSlotBuild {
                    slot_id: defense_slot_id.unwrap(),
                };
                command_scheduler.schedule_command(command);
                return;
            }

            // no free power well -> focus on next location
            self.enter_state(MacroState::GroundPresenceNextLoc);
        }
    }

    fn run_advance_tier(&mut self, command_scheduler: &mut CommandScheduler, game_info: &GameInfo) {
        if self.get_locations_under_attack(game_info).len() > 0 {
            self.enter_state(MacroState::Defend);
            return;
        }

        if command_scheduler.waiting_for_token_slot_to_finish() {
            // waiting for token slot to be built -> stay in this state
            return;
        }

        if game_info.bot.new_token_slot_ids.len() > 0 {
            // new token slot was built -> advance to next state
            self.enter_state(MacroState::HealUnits);
            return;
        }

        if game_info.bot.token_slots.len() == 0 || game_info.bot.token_slots.len() == 3 {
            // no token slots yet or already T3
            self.enter_state(MacroState::GroundPresenceNextLoc);
            return;
        }

        if command_scheduler.token_slot_can_be_built(game_info) {
            let offense_slot_id =
                location::get_next_free_token_slot(&self.attack_focus_loc, game_info);

            if offense_slot_id.is_some() {
                let command = Command::TokenSlotBuild {
                    slot_id: offense_slot_id.unwrap(),
                    color: BOT_ORBS[game_info.bot.token_slots.len()],
                };
                command_scheduler.schedule_command(command);
                self.set_latest_owning_loc(self.attack_focus_loc);
                return;
            }

            let defense_slot_id =
                location::get_next_free_token_slot(&self.latest_owning_loc, game_info);

            if defense_slot_id.is_some() {
                let command = Command::TokenSlotBuild {
                    slot_id: defense_slot_id.unwrap(),
                    color: BOT_ORBS[game_info.bot.token_slots.len()],
                };
                command_scheduler.schedule_command(command);
                return;
            }

            // no free orb -> focus on next location
            self.enter_state(MacroState::GroundPresenceNextLoc);
        }
    }

    fn run_heal_units(&mut self, game_info: &GameInfo) {
        self.spawn_controller.set_in_offense(false);

        if self.get_locations_under_attack(game_info).len() > 0 {
            self.enter_state(MacroState::Defend);
        }

        let pos = game_info
            .locations
            .get(&self.latest_owning_loc)
            .unwrap()
            .position();
        self.combat_controller.move_squads(pos, true);

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

    fn run_control_area(&mut self, game_info: &mut GameInfo) {
        self.spawn_controller.set_in_offense(false);

        let current_pos = self
            .combat_controller
            .get_spawn_location(game_info, &self.latest_owning_loc);
        let loc_pos = game_info
            .locations
            .get(&self.attack_focus_loc)
            .unwrap()
            .position();

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
            game_info.get_enemy_squads_in_range(&current_pos, CONTROL_AREA_AGGRO_RADIUS);
        if (game_info.bot.squads.len() as i64) - (enemy_squads_in_range.len() as i64) < -1 {
            // opponent has 2 or more squads more than me -> fight is lost, retreat
            self.spawn_controller.stop_spawn();
            self.enter_state(MacroState::HealUnits);
            return;
        }

        let enemy_squads_in_range =
            game_info.get_enemy_squads_in_range(&loc_pos, CONTROL_AREA_AGGRO_RADIUS);
        if enemy_squads_in_range.len() == 0 {
            // no more enemy squads in range -> take location
            self.spawn_controller.stop_spawn();

            if game_info.token_slot_diff() < 0 {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.seconds_have_passed(180) && game_info.bot.token_slots.len() == 1 {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.seconds_have_passed(420) && game_info.bot.token_slots.len() == 2 {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.power_slot_diff() < 0
                || game_info.bot.power > MIN_POWER_BUILD_WELL
                || game_info.bot.squads.len() >= 3
            {
                // opponent has one or more wells, bot has enough power to defend an attack or has
                // enough squads to defend a possible retaliation attack
                self.enter_state(MacroState::TakeWell);
                return;
            }
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

    fn run_attack_loc(&mut self, game_info: &mut GameInfo) {
        self.spawn_controller.set_in_offense(true);

        if location::get_location_owner(&self.attack_focus_loc, game_info).is_none() {
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

        let current_pos = self
            .combat_controller
            .get_spawn_location(game_info, &self.latest_owning_loc);
        let enemy_squads_in_range =
            game_info.get_enemy_squads_in_range(&current_pos, CONTROL_AREA_AGGRO_RADIUS);
        if (game_info.bot.squads.len() as i64) - (enemy_squads_in_range.len() as i64) < -1 {
            // opponent has 2 or more squads more than me -> fight is lost, retreat
            self.spawn_controller.stop_spawn();
            self.enter_state(MacroState::HealUnits);
            return;
        }

        self.spawn_controller.spawn_on_limit();

        let mut target: Option<EntityId> = None;
        let mut pos: Option<Position2D> = None;
        let loc = game_info.locations.get(&self.attack_focus_loc).unwrap();

        // attack power slots first
        for power_slot in &loc.powers {
            if game_info
                .opponent
                .power_slots
                .contains_key(&power_slot.entity_id.unwrap())
            {
                target = power_slot.entity_id;
                pos = Some(
                    game_info
                        .opponent
                        .power_slots
                        .get(&target.unwrap())
                        .unwrap()
                        .entity
                        .position
                        .to_2d(),
                );
            }
        }

        // power slots are not taken, attack the orb
        if target.is_none() {
            if let Some(token) = loc.token {
                target = token.entity_id;
                pos = Some(
                    game_info
                        .opponent
                        .token_slots
                        .get(&target.unwrap())
                        .unwrap()
                        .entity
                        .position
                        .to_2d(),
                );
            } else {
                warn!("Can not find slot token to attack");
            }
        }

        if target.is_some() {
            let num_enemy_squads_in_range = game_info
                .get_enemy_squads_in_range(&pos.unwrap(), CONTROL_AREA_AGGRO_RADIUS)
                .len() as i32;

            if (game_info.bot.squads.len() as i32) - num_enemy_squads_in_range
                >= NUM_SQUADS_CRITICAL_MASS
            {
                // reached a critical mass of own squads -> focus the well or orb
                self.combat_controller
                    .attack_slot_focus(&target.unwrap(), game_info);
            } else {
                // focus enemy squads first
                self.combat_controller
                    .attack_slot_control(&target.unwrap(), game_info);
            }
        } else {
            // neither one of the power wells nor the orb is taken, something is wrong
            error!(
                "Unable to find target to attack on location {:?}, this should not happen",
                self.attack_focus_loc
            );
        }
    }

    fn run_defend(&mut self, game_info: &mut GameInfo) {
        self.spawn_controller.set_in_offense(false);

        let locations_under_attack = self.get_locations_under_attack(game_info);

        if locations_under_attack.len() == 0 {
            // don't spawn any new units when the opponent is not attacking a location
            self.spawn_controller.stop_spawn();

            if game_info.token_slot_diff() < 0 {
                // opponent is a tier ahead -> build next orb
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.seconds_have_passed(180)
                && game_info.bot.token_slots.len() == 1
                && game_info.bot.power >= 200.
            {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

            if game_info.seconds_have_passed(420)
                && game_info.bot.token_slots.len() == 2
                && game_info.bot.power >= 300.
            {
                self.enter_state(MacroState::AdvanceTier);
                return;
            }

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

            if game_info.bot.power >= 300. {
                // lots of unspent power -> build a well or attack
                if game_info.bot.power_slots.len() < 7 {
                    self.enter_state(MacroState::TakeWell);
                    return;
                }

                self.enter_state(MacroState::GroundPresenceNextLoc);
                return;
            }

            return;
        }

        debug!("Locations under attack: {:?}", locations_under_attack);

        let loc_to_defend: Location;
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

    fn get_next_attack_focus_loc(&self, game_info: &GameInfo) -> Location {
        if MacroController::tempo_advantage(game_info) {
            let location_prios_ahead;
            if game_info.bot.start_location == Location::South {
                location_prios_ahead = LOCATION_PRIOS_AHEAD_SOUTH_START;
            } else if game_info.bot.start_location == Location::North {
                location_prios_ahead = LOCATION_PRIOS_AHEAD_NORTH_START;
            } else {
                error!("Unable to find prio locations based on start location, using south");
                location_prios_ahead = LOCATION_PRIOS_AHEAD_SOUTH_START;
            }

            // find the next location owned by the opponent if tempo is good
            for loc in location_prios_ahead[..(location_prios_ahead.len() - 1)].iter() {
                if let Some(owner_id) = location::get_location_owner(&loc, game_info) {
                    if owner_id == game_info.opponent.id {
                        // location is owned by the opponent
                        return *loc;
                    }
                }
            }

            // TODO: implement this properly
            if game_info.seconds_have_passed(300) {
                // hacky: allow targetting enemy base after 5 mins
                return location_prios_ahead[location_prios_ahead.len() - 1];
            }

            // find the location not owned by anyone
            for loc in location_prios_ahead.iter() {
                if let None = location::get_location_owner(&loc, game_info) {
                    return *loc;
                }
            }

            error!("Unable to find next free location from ahead");
            Location::North
        } else {
            let location_prios_behind;
            if game_info.bot.start_location == Location::South {
                location_prios_behind = LOCATION_PRIOS_BEHIND_SOUTH_START;
            } else if game_info.bot.start_location == Location::North {
                location_prios_behind = LOCATION_PRIOS_BEHIND_NORTH_START;
            } else {
                error!("Unable to find prio locations based on start location, using south");
                location_prios_behind = LOCATION_PRIOS_BEHIND_SOUTH_START;
            }

            // find the location not owned by anyone
            for loc in location_prios_behind.iter() {
                if let None = location::get_location_owner(&loc, game_info) {
                    return *loc;
                }
            }

            error!("Unable to find next free location from behind");
            Location::North
        }
    }

    fn repair_structures(game_info: &GameInfo, command_scheduler: &mut CommandScheduler) {
        // check power slots
        for power_slot in game_info.bot.power_slots.values() {
            let entity_id = power_slot.entity.id;
            let (cur_hp, max_hp) = game_info.get_structure_health(&entity_id);
            if cur_hp < max_hp {
                command_scheduler.schedule_command(Command::RepairBuilding {
                    building_id: entity_id,
                });
            }
        }

        // check token slots
        for token_slot in game_info.bot.token_slots.values() {
            let entity_id = token_slot.entity.id;
            let (cur_hp, max_hp) = game_info.get_structure_health(&entity_id);
            if cur_hp < max_hp {
                command_scheduler.schedule_command(Command::RepairBuilding {
                    building_id: entity_id,
                });
            }
        }
    }

    fn tempo_advantage(game_info: &GameInfo) -> bool {
        game_info.bot.get_tempo() - game_info.opponent.get_tempo() >= MIN_TEMPO_DIFF_ADVANTAGE
    }

    fn enter_state(&mut self, new_state: MacroState) {
        info!("MacroController entered state {:?}", new_state);
        self.state = new_state;
    }

    fn set_attack_focus_loc(&mut self, new_loc: Location) {
        if new_loc != self.attack_focus_loc {
            info!(
                "MacroController: focussing attacks on location {:?}",
                new_loc
            );
            self.attack_focus_loc = new_loc;
        }
    }

    fn set_latest_owning_loc(&mut self, new_loc: Location) {
        if new_loc != self.latest_owning_loc {
            info!("MacroController: controlling location {:?}", new_loc);
            self.latest_owning_loc = new_loc;
            if !self.owning_loc_history.contains(&new_loc) {
                self.owning_loc_history.push(new_loc);
                debug!("Owning location history: {:?}", self.owning_loc_history);
            }
        }
    }

    fn get_locations_under_attack(&self, game_info: &GameInfo) -> Vec<Location> {
        let location_prios;
        if game_info.bot.start_location == Location::South {
            location_prios = LOCATION_PRIOS_AHEAD_SOUTH_START;
        } else if game_info.bot.start_location == Location::North {
            location_prios = LOCATION_PRIOS_AHEAD_NORTH_START;
        } else {
            error!("Unable to find prio locations based on start location, using south");
            location_prios = LOCATION_PRIOS_AHEAD_SOUTH_START;
        }

        let owned_locations: Vec<Location> = location_prios
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
        locations_under_attack
    }

    fn handle_destroyed_slots(&mut self, game_info: &GameInfo) {
        if game_info.bot.destroyed_power_slot_ids.len() == 0
            && game_info.bot.destroyed_token_slot_ids.len() == 0
        {
            return;
        }

        // check history of owned locations
        let mut indices_to_delete: Vec<usize> = vec![];
        for (i, loc) in self.owning_loc_history.iter().enumerate() {
            let loc_owner = location::get_location_owner(&loc, game_info);
            if !loc_owner.is_some_and(|id| id == game_info.bot.id) {
                // location is not owned by me
                debug!("Lost location {:?}", loc);
                indices_to_delete.push(i);
            }
        }

        for index in indices_to_delete {
            if index == 0 {
                // bot just lost his only location -> game over either way
                continue;
            }
            if index == self.owning_loc_history.len() - 1 {
                // lost the latest owning location -> need to update it
                self.set_latest_owning_loc(self.owning_loc_history[index - 1]);
            }
            self.owning_loc_history.remove(index);
        }
    }
}
