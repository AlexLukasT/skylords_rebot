use api::Upgrade::U3;
use api::*;
use log::*;
use sr_libs::utils::card_templates::CardTemplate;
use std::num::NonZeroU32;

use crate::game_info::GameInfo;

use crate::bot::BOT_DECK;
use crate::controller::Controller;
use crate::location::Location;
use crate::utils;

const DEST_REACHED_MARGIN: f32 = 5.;

#[derive(Debug)]
pub struct SquadController {
    entity_id: EntityId,
    state: SquadControllerState,
    commands: Vec<Command>,
    current_destination: Option<Position2D>,
    name: String,
}

#[derive(Debug, Default, PartialEq)]
enum SquadControllerState {
    #[default]
    NotInitialized,
    Idling,
    SpawnCommandSent,
    Moving,
    Attacking,
    Defending,
}

impl SquadController {
    pub fn new(name: String) -> SquadController {
        SquadController {
            entity_id: EntityId(NonZeroU32::new(1).unwrap()),
            state: SquadControllerState::NotInitialized,
            commands: vec![],
            current_destination: None,
            name,
        }
    }

    pub fn spawn(&mut self, card: CardTemplate, position: Position2D) {
        if self.state == SquadControllerState::NotInitialized {
            let card_id = CardId::new(card, U3);
            if let Some(card_pos) = BOT_DECK.cards.iter().position(|&c_id| c_id == card_id) {
                self.commands.push(Command::ProduceSquad {
                    card_position: card_pos as u8,
                    xy: position,
                });
                self.enter_state(SquadControllerState::SpawnCommandSent);
            } else {
                warn!("Unable to find deck position for card {:?}", card_id);
            }
        }
    }

    pub fn move_squad(&mut self, game_info: &GameInfo, new_dest: Position2D) {
        let new_destination_provided: bool;
        if let Some(cur_dest) = self.current_destination {
            if utils::dist(&cur_dest, &new_dest) < DEST_REACHED_MARGIN {
                // the new destination is not far enough from the current one
                new_destination_provided = false;
            } else {
                new_destination_provided = true;
            }
        } else {
            // no current destination set
            new_destination_provided = true;
        }

        if (self.state == SquadControllerState::Idling
            || self.state == SquadControllerState::Moving)
            && new_destination_provided
        {
            self.commands.push(Command::GroupGoto {
                squads: vec![self.entity_id],
                positions: vec![Location::CenterToken.to_pos2d(game_info)],
                walk_mode: WalkMode::Normal,
                orientation: 0.,
            });
            self.current_destination = Some(new_dest);
            self.enter_state(SquadControllerState::Moving);
            debug!(
                "{:?} ({:?})) moving towards {:?}",
                self.name, self.entity_id, self.current_destination
            );
        }
    }

    fn enter_state(&mut self, new_state: SquadControllerState) {
        info!(
            "{:?} ({:?}) entered state {:?}",
            self.name, self.entity_id, new_state
        );
        self.state = new_state;
    }
}

impl Controller for SquadController {
    fn tick(&mut self, game_info: &GameInfo) -> Vec<Command> {
        let mut new_commands = self.commands.clone();
        self.commands.clear();

        if self.state == SquadControllerState::SpawnCommandSent {
            let num_new_squads = game_info.bot.new_squad_ids.len();
            if num_new_squads == 1 {
                // found the squad this controller should manage
                self.entity_id = game_info.bot.new_squad_ids[0];
                debug!(
                    "Found new squad {:?} for SquadController {:?}",
                    self.entity_id, self.name
                );
                self.enter_state(SquadControllerState::Idling);
            } else if num_new_squads > 1 {
                // TODO: handle this properly. currently this relies
                // on exactly one squad being spawned per tick and the
                // game state containing exactly one squad at a later tick
                error!(
                    "Got {:?} new squads when assigning squad to controller",
                    num_new_squads
                )
            }
        }

        if self.state == SquadControllerState::Moving {
            if let Some(squad) = game_info.bot.squads.get(&self.entity_id) {
                if let Some(dest) = self.current_destination {
                    debug!("{:?}", utils::dist(&squad.entity.position.to_2d(), &dest));
                    if utils::dist(&squad.entity.position.to_2d(), &dest) < DEST_REACHED_MARGIN {
                        // squad is close enough to the destination that we handle this as if it
                        // reached it's destination
                        self.enter_state(SquadControllerState::Idling);
                        debug!(
                            "{:?} ({:?}) reached destination {:?}",
                            self.name, self.entity_id, self.current_destination
                        );
                    }
                }
            } else {
                warn!("Unable to find squad for controller in game info");
            }
        }

        new_commands
    }
}
