use crate::game_info::GameInfo;
use api::sr_libs::utils::card_templates::CardTemplate;
use api::sr_libs::utils::card_templates::CardTemplate::*;
use api::Upgrade::U3;
use api::*;
use log::{debug, info, warn};
use std::borrow::Cow;

use crate::command_scheduler::CommandScheduler;
use crate::controller::macro_controller::MacroController;

// /AI: add SkylordsRebot Tutorial 4
const NAME: &'static str = "SkylordsRebot";

pub struct SkylordsRebot {
    deck: &'static Deck,
    game_info: GameInfo,
    macro_controller: MacroController,
    command_scheduler: CommandScheduler,
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
            macro_controller: MacroController::new(),
            command_scheduler: CommandScheduler::new(),
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
}

fn on_tick(bot_state: &mut SkylordsRebot, state: GameState) -> Vec<Command> {
    if state.rejected_commands.len() > 0 {
        warn!("Rejected commands: {:?}", state.rejected_commands);
    }
    bot_state.game_info.parse_state(state);
    bot_state
        .command_scheduler
        .update_state(&bot_state.game_info);

    bot_state
        .macro_controller
        .tick(&mut bot_state.game_info, &mut bot_state.command_scheduler);

    let scheduled_commands = bot_state.command_scheduler.get_scheduled_commands();

    if scheduled_commands.len() > 0 {
        debug!("Sending commands: {:?}", scheduled_commands);
    }

    scheduled_commands
}

pub const BOT_DECK: Deck = Deck {
    name: Cow::Borrowed("ShadowFrost"),
    cover_card_index: 0,
    cards: [
        CardId::new(Dreadcharger, U3),
        CardId::new(Forsaken, U3),
        CardId::new(NoxTrooper, U3),
        CardId::new(Executor, U3),
        CardId::new(Motivate, U3),
        CardId::new(NastySurprise, U3),
        CardId::new(LifeWeaving, U3),
        CardId::new(FrostBiteAShadow, U3),
        CardId::new(GlacierShell, U3),
        CardId::new(KoboldTrick, U3),
        CardId::new(DarkelfAssassins, U3),
        CardId::new(Nightcrawler, U3),
        CardId::new(StormsingerANature, U3),
        CardId::new(LostReaverAShadow, U3),
        CardId::new(Coldsnap, U3),
        CardId::new(AuraofCorruption, U3),
        CardId::new(TimelessOne, U3),
        CardId::new(SilverwindLancers, U3),
        CardId::new(Tremor, U3),
        CardId::new(LostGrigoriAFire, U3),
    ],
};

pub const BOT_CARDS: [CardTemplate; 20] = [
    Dreadcharger,
    Forsaken,
    NoxTrooper,
    Executor,
    Motivate,
    NastySurprise,
    LifeWeaving,
    FrostBiteAShadow,
    GlacierShell,
    KoboldTrick,
    DarkelfAssassins,
    Nightcrawler,
    StormsingerANature,
    LostReaverAShadow,
    Coldsnap,
    AuraofCorruption,
    TimelessOne,
    SilverwindLancers,
    Tremor,
    LostGrigoriAFire,
];

pub const BOT_ORBS: [CreateOrbColor; 3] = [
    CreateOrbColor::Shadow,
    CreateOrbColor::Frost,
    CreateOrbColor::Frost,
];

// TODO: figure this out dynamically
pub const BOT_T1_UNITS: [usize; 4] = [0, 1, 2, 3];
pub const BOT_T2_UNITS: [usize; 4] = [10, 11, 12, 13];
pub const BOT_T3_UNITS: [usize; 3] = [17, 18, 19];
