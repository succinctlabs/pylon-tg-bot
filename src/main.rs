use std::sync::Arc;

use clap::Parser;
use notify::{RecursiveMode, Watcher};
use teloxide::{
    Bot,
    dispatching::{HandlerExt, UpdateFilterExt},
    dptree::{deps, entry},
    prelude::Dispatcher,
    types::{Message, Update},
};
use tokio::{
    select,
    sync::{RwLock, mpsc::unbounded_channel},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::{
    cli::Args,
    config::Settings,
    endpoints::{Command, handle_bot_status_change, process_command},
    pylon::PylonClient,
};

const BOT_USERNAME: &str = "SuccinctPylonBot";

mod cli;
mod config;
mod endpoints;
mod pylon;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Err(err) = dotenvy::dotenv() {
        warn!("Failed to load .env file: {err}")
    }

    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let settings_path = args
        .settings_path
        .unwrap_or_else(|| "./settings.toml".to_string());
    let settings = Arc::new(RwLock::new(confy::load_path::<Settings>(
        settings_path.clone(),
    )?));
    let latest_message: Arc<RwLock<Option<Message>>> = Arc::new(RwLock::new(None));
    let pylon_client = Arc::new(PylonClient::new(args.pylon_api_token.clone()));
    let settings_reload = settings.clone();
    let token = CancellationToken::new();
    let cloned_token = token.clone();

    // Watch settings file for updates
    tokio::spawn(async move {
        let (tx, mut rx) = unbounded_channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .unwrap();

        watcher
            .watch(settings_path.as_ref(), RecursiveMode::NonRecursive)
            .unwrap();

        loop {
            select! {
                _ = cloned_token.cancelled() => {
                     break;
                }
                Some(res) = rx.recv() => {
                    match res {
                        Ok(event) => {
                            if event.kind.is_modify()
                                && let Ok(settings) = confy::load_path::<Settings>(settings_path.clone())
                            {
                                let mut settings_reload = settings_reload.write().await;
                                *settings_reload = settings;
                                info!("Settings reloaded");
                            }
                        }
                        Err(e) => error!("watch error: {:?}", e),
                    }
                }
            }
        }
    });

    let bot = Bot::from_env();

    let schema = Update::filter_message()
        .filter_map(|update: Update| update.from().cloned())
        .branch(
            entry()
                .filter_command::<Command>()
                .endpoint(process_command),
        )
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

    token.cancel();

    Ok(())
}
