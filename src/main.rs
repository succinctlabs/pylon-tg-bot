use std::sync::Arc;

use clap::Parser;
use eyre::eyre;
use teloxide::{
    Bot,
    dispatching::{HandlerExt, MessageFilterExt, UpdateFilterExt},
    dptree::{deps, entry},
    prelude::{Dispatcher, Requester},
    types::{Message, MessageKind, Update, User},
    utils::command::BotCommands,
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::{cli::Args, config::Settings, pylon::PylonClient};

const BOT_USERNAME: &str = "SuccinctPylonBot";

mod cli;
mod config;
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

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Display this text.
    #[command(aliases = ["h", "?"])]
    Help,
    /// Create an issue.
    #[command()]
    Issue,
}

async fn process_command(
    bot: Bot,
    message: Message,
    cmd: Command,
    pylon_client: Arc<PylonClient>,
    settings: Arc<RwLock<Settings>>,
    latest_message: Arc<RwLock<Option<Message>>>,
) -> eyre::Result<()> {
    let settings = settings.read().await;

    match cmd {
        Command::Help => {
            bot.send_message(message.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Issue => {
            let message = if let Some(replied) = message.reply_to_message() {
                replied.clone()
            } else {
                let latest_message = latest_message.write().await;
                latest_message
                    .as_ref()
                    .ok_or_else(|| eyre!("No message"))?
                    .clone()
            };

            if let Some(message_text) = message.text() {
                info!("New message: {message_text}");
                let chat_title = message.chat.title().unwrap_or_default();

                if let Some(pylon_account) = settings
                    .tg_chats_to_pylon_accounts
                    .get(&message.chat.id.to_string())
                {
                    let username = message
                        .from
                        .clone()
                        .and_then(|u| u.username)
                        .unwrap_or_default();

                    pylon_client
                        .create_issue(
                            &format!("New issue from {username} on {chat_title}"),
                            message_text,
                            pylon_account,
                        )
                        .await?;
                } else {
                    warn!("No Pylon account defined for chat {chat_title}",);
                }
            }
        }
    };

    Ok(())
}

/// Replies to the user's text messages
async fn process_text_message(
    user: User,
    message: Message,
    settings: Arc<RwLock<Settings>>,
    latest_message: Arc<RwLock<Option<Message>>>,
) -> eyre::Result<()> {
    let settings = settings.read().await;

    if let Some(username) = &user.username {
        if settings.ignored_tg_usernames.contains(username) {
            debug!("User {username} message is ignored");

            return Ok(());
        }

        if message.text().is_some() {
            let _ = latest_message.write().await.replace(message);
        }
    }

    Ok(())
}

async fn handle_bot_status_change(message: Message) -> eyre::Result<()> {
    match message.kind {
        MessageKind::NewChatMembers(members) => {
            if members
                .new_chat_members
                .into_iter()
                .any(|m| m.username == Some(BOT_USERNAME.to_string()))
            {
                info!(
                    "Bot was added to chat: {}, {}",
                    message.chat.id,
                    message.chat.title().unwrap_or_default()
                );
            }
        }
        MessageKind::LeftChatMember(member) => {
            if member.left_chat_member.username == Some(BOT_USERNAME.to_string()) {
                warn!(
                    "Bot was removed from chat: {}, {}",
                    message.chat.id,
                    message.chat.title().unwrap_or_default()
                )
            }
        }
        _ => {}
    }

    Ok(())
}
