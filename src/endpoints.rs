use std::sync::Arc;

use teloxide::{
    Bot,
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{Message, MessageKind, ParseMode},
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
) -> eyre::Result<()> {
    let settings = settings.read().await;

    match cmd {
        Command::Help => {
            bot.send_message(message.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Issue => {
            let username = message
                .from
                .clone()
                .and_then(|u| u.username)
                .unwrap_or_default();

            let chat_title = message.chat.title().unwrap_or_default();

            let message = if let Some(replied) = message.reply_to_message() {
                replied.clone()
            } else {
                warn!("/issue called without replying by {username} in {chat_title}");

                return Ok(());
            };

            if let Some(message_text) = message.text() {
                info!("New message: {message_text}");

                if let Some(pylon_account) = settings
                    .tg_chats_to_pylon_accounts
                    .get(&message.chat.id.to_string())
                {
                    let response = pylon_client
                        .create_issue(
                            &format!("New issue from {username} on {chat_title}"),
                            message_text,
                            pylon_account,
                        )
                        .await?;

                    bot.send_message(
                        message.chat.id,
                        format!(
                            "✅ New issue [\\#{}]({}) created in Pylon",
                            response.number.unwrap_or_default(),
                            response.link.unwrap_or_default()
                        ),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
                } else {
                    warn!("No Pylon account defined for chat {chat_title}");
                }
            } else {
                debug!("Not a text message")
            }
        }
    };

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
