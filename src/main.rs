use clap::Parser;
use log::{info, LevelFilter};

use api::*;

mod bot;
mod game_info;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value_t = 7273)]
    port: u16,
    #[arg(short, long, value_enum, default_value = "example")]
    implementation: BotImplementations,
}

#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq, clap::ValueEnum)]
enum BotImplementations {
    #[default]
    SkylordsRebot,
}

#[tokio::main]
async fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let args = Args::parse();

    match args.implementation {
        BotImplementations::SkylordsRebot => {
            info!("running example bot");
            warp_wrapper::run::<bot::SkylordsRebot>(args.port).await
        }
    };
}
