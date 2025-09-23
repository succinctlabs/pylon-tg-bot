use std::sync::Arc;

use clap::Parser;
use teloxide::{
    Bot,
    dispatching::{HandlerExt, MessageFilterExt, UpdateFilterExt},
    dptree::{deps, entry},
    prelude::Dispatcher,
    types::{Message, Update},
};
use tokio::sync::RwLock;
use tracing::error;

use crate::{
    cli::Args,
    config::Settings,
    endpoints::{Command, handle_bot_status_change, process_command, process_text_message},
    pylon::PylonClient,
};

const BOT_USERNAME: &str = "SuccinctPylonBot";

mod cli;
mod config;
mod endpoints;
mod pylon;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let settings = Arc::new(RwLock::new(confy::load_path::<Settings>(
        "./settings.toml",
    )?));
    let latest_message: Arc<RwLock<Option<Message>>> = Arc::new(RwLock::new(None));
    let pylon_client = Arc::new(PylonClient::new(args.pylon_api_token.clone()));

    let bot = Bot::from_env();

    let schema = Update::filter_message()
        .filter_map(|update: Update| update.from().cloned())
        .branch(
            entry()
                .filter_command::<Command>()
                .endpoint(process_command),
        )
        .branch(Message::filter_text().endpoint(process_text_message))
        .branch(Update::filter_message().endpoint(handle_bot_status_change));

    Dispatcher::builder(bot, schema)
        .dependencies(deps![pylon_client, settings, latest_message])
        .enable_ctrlc_handler()
        .error_handler(Arc::new(|err| {
            error!("{err}");
            Box::pin(async {})
        }))
        .build()
        .dispatch()
        .await;

    Ok(())
}
