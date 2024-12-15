use clap::Parser;
use env_logger::Builder;
use log::{info, LevelFilter};
use std::fs::File;

use api::*;

mod bot;
mod card_data;
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
    let file_name = dbg!(chrono::Local::now().format("%Y-%m-%d_%H_%M_%S").to_string() + ".log");
    let log_file = Box::new(File::create(file_name).expect("Can't create log file"));
    let mut builder = Builder::new();
    // set logging for hyper::proto module to info so it doesn't clutter the debug log
    builder.filter_module("hyper::proto", LevelFilter::Info);
    builder.filter_level(LevelFilter::Info);
    // https://github.com/rust-cli/env_logger/issues/125#issuecomment-1406333500
    builder.target(env_logger::Target::Pipe(log_file));
    builder.init();

    let args = Args::parse();

    match args.implementation {
        BotImplementations::SkylordsRebot => {
            info!("running example bot");
            warp_wrapper::run::<bot::SkylordsRebot>(args.port).await
        }
    };
}
