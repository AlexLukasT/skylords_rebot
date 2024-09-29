use clap::Parser;
use env_logger::Builder;
use log::{info, LevelFilter};

use api::*;

mod bot;
mod command_scheduler;
mod controller;
mod game_info;
mod location;
mod utils;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value_t = 7273)]
    port: u16,
    #[arg(short, long, value_enum, default_value = "skylords-rebot")]
    implementation: BotImplementations,
}

#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, clap::ValueEnum)]
enum BotImplementations {
    #[default]
    SkylordsRebot,
}

#[tokio::main]
async fn main() {
    let mut builder = Builder::new();
    // set logging for hyper::proto module to info so it doesn't clutter the debug log
    builder.filter_module("hyper::proto", LevelFilter::Info);
    builder.filter_level(LevelFilter::Debug);
    builder.init();

    let args = Args::parse();

    match args.implementation {
        BotImplementations::SkylordsRebot => {
            info!("running example bot");
            warp_wrapper::run::<bot::SkylordsRebot>(args.port).await
        }
    };
}
