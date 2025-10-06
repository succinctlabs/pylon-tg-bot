use std::sync::Arc;

use clap::Parser;
use notify::{RecursiveMode, Watcher};
use teloxide::{
    Bot,
    dispatching::{HandlerExt, UpdateFilterExt, dialogue::InMemStorage},
    dptree::{case, deps, entry},
    prelude::Dispatcher,
    types::{Message, Update},
};
use tokio::{
    select,
    sync::{RwLock, mpsc::unbounded_channel},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_appender::rolling;
use tracing_subscriber::{
    EnvFilter, Layer, fmt::layer, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{
    cli::Args,
    config::Config,
    endpoints::{
        AdminCommand, Command, State, handle_account_id_input, handle_bot_status_change,
        handle_callback, is_private_chat, is_public_chat, process_admin_command, process_command,
    },
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

    let args = Args::parse();

    let (file_layer, _guard) = if let Some(logs_path) = args.logs_path {
        // Create a rolling file appender
        let file_appender = rolling::never(logs_path, "logs.txt");

        // Create a layer that writes to the file
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let layer = layer()
            .compact()
            .with_target(false)
            .with_writer(non_blocking)
            .with_filter(LevelFilter::INFO);

        (Some(layer), Some(_guard))
    } else {
        (None, None)
    };

    tracing_subscriber::registry()
        .with(
            layer().compact().with_target(false).with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
        )
        .with(file_layer)
        .init();

    let settings_path = args
        .settings_path
        .unwrap_or_else(|| "./settings.toml".to_string());
    let config = Arc::new(Config::try_new(settings_path.clone())?);
    let latest_message: Arc<RwLock<Option<Message>>> = Arc::new(RwLock::new(None));
    let pylon_client = Arc::new(PylonClient::new(args.pylon_api_token.clone()));
    let config_reload = config.clone();
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
                                && config_reload.reload().await.is_ok() {
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
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(
            entry()
                .filter(is_public_chat)
                .filter_command::<Command>()
                .endpoint(process_command),
        )
        .branch(
            case![State::Start].branch(
                entry()
                    .filter(is_private_chat)
                    .filter_command::<AdminCommand>()
                    .endpoint(process_admin_command),
            ),
        )
        .branch(case![State::WaitingForAccountId { chat_id }].endpoint(handle_account_id_input))
        .branch(Update::filter_callback_query().endpoint(handle_callback))
        .branch(Update::filter_message().endpoint(handle_bot_status_change));

    info!("Starting bot...");
    Dispatcher::builder(bot, schema)
        .dependencies(deps![pylon_client, config, latest_message])
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
