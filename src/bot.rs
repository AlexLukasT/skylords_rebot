use crate::game_info::GameInfo;
use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::Upgrade::U3;
use api::*;
use log::{debug, info, warn};
use std::borrow::Cow;

use crate::controller::squad_controller::SquadController;
use crate::controller::Controller;
use crate::location::Location;

// /AI: add SkylordsRebot Tutorial 4
const NAME: &'static str = "SkylordsRebot";

pub struct SkylordsRebot {
    deck: &'static Deck,
    game_info: GameInfo,
    controllers: Vec<SquadController>,
}

impl warp_wrapper::BotImpl for SkylordsRebot {
    fn name() -> &'static str {
        NAME
    }

    fn decks_for_map(_map_info: &MapInfo) -> &'static [Deck] {
        &[BOT_DECK]
    }

    fn prepare_for_battle(map_info: &MapInfo, deck: &'static Deck) -> Self {
        info!("Preparing for: {:?}?", map_info.map);
        SkylordsRebot {
            deck,
            game_info: GameInfo::new(),
            controllers: vec![],
        }
    }

    fn match_start(&mut self, state: GameStartState) {
        match_start(self, state)
    }

    fn tick(&mut self, state: GameState) -> Vec<Command> {
        on_tick(self, state)
    }
}

fn match_start(bot_state: &mut SkylordsRebot, state: GameStartState) {
    bot_state.game_info.init(state);
    let mut init_squad = SquadController::new("Dreadcharger1".to_string());
    init_squad.spawn(
        Dreadcharger,
        Location::BotStartToken.to_pos2d(&bot_state.game_info),
    );
    bot_state.controllers.push(init_squad);
}

fn on_tick(bot_state: &mut SkylordsRebot, state: GameState) -> Vec<Command> {
    bot_state.game_info.parse_state(state);

    let mut commands: Vec<Command> = vec![];

    for controller in bot_state.controllers.iter_mut() {
        controller.move_squad(
            &bot_state.game_info,
            Location::CenterToken.to_pos2d(&bot_state.game_info),
        );
        commands.extend(controller.tick(&bot_state.game_info));
    }

    commands
}

pub const BOT_DECK: Deck = Deck {
    name: Cow::Borrowed("ShadowNature"),
    cover_card_index: 0,
    cards: [
        CardId::new(Dreadcharger, U3),
        CardId::new(Forsaken, U3),
        CardId::new(NoxTrooper, U3),
        CardId::new(Motivate, U3),
        CardId::new(NastySurprise, U3),
        CardId::new(LifeWeaving, U3),
        CardId::new(EnsnaringRoots, U3),
        CardId::new(Hurricane, U3),
        CardId::new(SurgeOfLight, U3),
        CardId::new(CurseofOink, U3),
        CardId::new(Tranquility, U3),
        CardId::new(AuraofCorruption, U3),
        CardId::new(DarkelfAssassins, U3),
        CardId::new(Nightcrawler, U3),
        CardId::new(AmiiPaladins, U3),
        CardId::new(AmiiPhantom, U3),
        CardId::new(Burrower, U3),
        CardId::new(ShadowPhoenix, U3),
        CardId::new(CultistMaster, U3),
        CardId::new(AshbonePyro, U3),
    ],
};
