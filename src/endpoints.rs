use std::sync::Arc;

use eyre::eyre;
use teloxide::{
    Bot,
    prelude::Requester,
    types::{Message, MessageKind, User},
    utils::command::BotCommands,
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{BOT_USERNAME, config::Settings, pylon::PylonClient};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Display this text.
    #[command(aliases = ["h", "?"])]
    Help,
    /// Create an issue.
    #[command()]
    Issue,
}

pub async fn process_command(
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
pub async fn process_text_message(
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

pub async fn handle_bot_status_change(message: Message) -> eyre::Result<()> {
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
