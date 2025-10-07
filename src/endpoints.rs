use std::sync::Arc;

use serde::{Deserialize, Serialize};
use teloxide::{
    Bot,
    dispatching::dialogue::{GetChatId, InMemStorage},
    payloads::SendMessageSetters,
    prelude::{Dialogue, Requester},
    types::{
        CallbackQuery, ChatId, ChatKind, ChatMemberStatus, InlineKeyboardButton, Message,
        MessageKind, ParseMode,
    },
    utils::command::BotCommands,
};
use tracing::{debug, info, warn};

use crate::{
    BOT_USERNAME,
    config::{Config, Settings},
    pylon::PylonClient,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Display this text.
    #[command(aliases = ["h", "?"])]
    Help,

    /// Create an issue.
    #[command()]
    Issue(String),
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum AdminCommand {
    /// Display this text.
    #[command(aliases = ["h", "?"])]
    Help,

    /// List all chats.
    #[command()]
    List,

    /// Link a chat to a Pylon account.
    #[command()]
    Link,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub enum State {
    #[default]
    Start,
    WaitingForAccountId {
        chat_id: String,
    },
}

type LinkToPylonAccountDialogue = Dialogue<State, InMemStorage<State>>;

pub async fn process_command(
    bot: Bot,
    message: Message,
    cmd: Command,
    pylon_client: Arc<PylonClient>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    match cmd {
        Command::Help => {
            bot.send_message(message.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Issue(title) => new_issue(title, &bot, message, pylon_client, config).await?,
    };

    Ok(())
}

pub async fn process_admin_command(
    bot: Bot,
    message: Message,
    cmd: AdminCommand,
    pylon_client: Arc<PylonClient>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    if is_public_chat(message.clone()) {
        warn!("Admin commands are only authorized in a private chat with the bot");
        return Ok(());
    }

    let settings = config.get().await;

    if !message
        .from
        .and_then(|user| user.username)
        .map(|username| settings.bot_admins.contains(&username))
        .unwrap_or_default()
    {
        warn!("Unauthorized call to admin command");
        return Ok(());
    }

    match cmd {
        AdminCommand::Help => {
            bot.send_message(message.chat.id, AdminCommand::descriptions().to_string())
                .await?;
        }
        AdminCommand::List => list_accounts(&bot, message.chat.id, pylon_client, settings).await?,
        AdminCommand::Link => link_chat_to_account(&bot, message.chat.id, settings).await?,
    }

    Ok(())
}

pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    dialogue: LinkToPylonAccountDialogue,
) -> eyre::Result<()> {
    if let Some(chat_id) = q.data {
        // Answer the callback to remove loading state
        bot.answer_callback_query(q.id).await?;

        // Update dialogue state
        dialogue
            .update(State::WaitingForAccountId { chat_id })
            .await?;

        // Prompt for account ID
        if let Some(message) = q.message
            && let Some(chat_id) = message.chat().chat_id()
        {
            bot.send_message(chat_id, "Please enter the account ID to link this chat to:")
                .await?;
        } else {
            warn!("Can't ask for account id")
        }
    }

    Ok(())
}

pub async fn handle_account_id_input(
    bot: Bot,
    message: Message,
    dialogue: LinkToPylonAccountDialogue,
    chat_id: String,
    pylon_client: Arc<PylonClient>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    if let Some(account_id) = message.text() {
        let account_id = account_id.trim().to_string();
        let mut settings = config.get().await;

        if let Some(account) = pylon_client.get_account(&account_id).await? {
            settings
                .tg_chats_to_pylon_accounts
                .insert(chat_id.clone(), account_id);

            config.save(settings)?;

            bot.send_message(
                message.chat.id,
                format!(
                    "✅ Chat linked to Pylon account '{}'",
                    account.name.clone().unwrap_or_default(),
                ),
            )
            .await?;

            // Reset dialogue to start
            dialogue.update(State::Start).await?;

            info!(
                "Chat '{chat_id}' linked to Pylon Account '{}'",
                account.name.unwrap_or_default()
            );
        } else {
            bot.send_message(message.chat.id, "⚠️ Account not found in Pylon")
                .await?;
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

async fn new_issue(
    title: String,
    bot: &Bot,
    message: Message,
    pylon_client: Arc<PylonClient>,
    config: Arc<Config>,
) -> eyre::Result<()> {
    let settings = config.get().await;

    let username = message
        .from
        .clone()
        .and_then(|u| u.username)
        .unwrap_or_default();
    let chat_title = message.chat.title().unwrap_or_default();
    let message_title = title.trim();

    let message_title = if message_title.is_empty() {
        format!("New issue from {username} on {chat_title}")
    } else {
        message_title.to_string()
    };

    let message = if let Some(replied) = message.reply_to_message() {
        replied.clone()
    } else {
        warn!("/issue called without replying by {username} in {chat_title}");

        return Ok(());
    };

    if let Some(message_text) = message.text() {
        info!("New message from {username} in {chat_title}: {message_text}");

        if let Some(pylon_account) = settings
            .tg_chats_to_pylon_accounts
            .get(&message.chat.id.to_string())
        {
            let response = pylon_client
                .create_issue(&message_title, message_text, pylon_account)
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

    Ok(())
}

async fn list_accounts(
    bot: &Bot,
    chat_id: ChatId,
    pylon_client: Arc<PylonClient>,
    settings: Settings,
) -> eyre::Result<()> {
    let mut linked = String::new();
    let mut not_linked = String::new();
    let mut bot_not_member = String::new();

    for (chat_id, pylon_account_id) in &settings.tg_chats_to_pylon_accounts {
        let chat = bot.get_chat(chat_id.clone()).await?;
        let chat_title = escape_markdown_v2(chat.title().unwrap_or_default());

        if pylon_account_id.is_empty() {
            not_linked.push_str(&format!("• {chat_title}\n"));
        } else if let Some(pylon_account) = pylon_client.get_account(pylon_account_id).await? {
            linked.push_str(&format!(
                "• {chat_title} ➡️ {}\n",
                escape_markdown_v2(pylon_account.name.unwrap_or_default().as_str())
            ));
        } else {
            warn!("Account '{pylon_account_id}' not found in Pylon")
        }

        if !is_bot_member(bot, chat.id).await? {
            bot_not_member.push_str(&format!("• {chat_title}\n"));
        }
    }

    let linked_table = if linked.is_empty() {
        String::from("No data\n")
    } else {
        linked
    };
    let not_linked_table = if not_linked.is_empty() {
        String::from("No data\n")
    } else {
        not_linked
    };
    let bot_not_member_table = if bot_not_member.is_empty() {
        String::from("No data\n")
    } else {
        bot_not_member
    };

    bot.send_message(
        chat_id,
        format!(
            "*✅ Chats linked to Pylon accounts*\n \
            {linked_table} \
            *❌ Chats not linked to Pylon accounts*\n \
            {not_linked_table} \
            *⚠️ Chats without the bot added*\n \
            {bot_not_member_table}"
        ),
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

async fn link_chat_to_account(bot: &Bot, chat_id: ChatId, setting: Settings) -> eyre::Result<()> {
    // Create inline keyboard with unlinked chats
    let mut keyboard = Vec::new();
    for (chat_id, pylon_account_id) in &setting.tg_chats_to_pylon_accounts {
        if pylon_account_id.trim().is_empty() {
            let chat = bot.get_chat(chat_id.clone()).await?;
            let chat_title = escape_markdown_v2(chat.title().unwrap_or_default());

            keyboard.push(vec![InlineKeyboardButton::callback(chat_title, chat_id)]);
        }
    }

    let inline_keyboard = teloxide::types::InlineKeyboardMarkup::new(keyboard);

    bot.send_message(chat_id, "Please select a chat to link:")
        .reply_markup(inline_keyboard)
        .await?;

    Ok(())
}

async fn is_bot_member(bot: &Bot, chat_id: ChatId) -> eyre::Result<bool> {
    let bot_user = bot.get_me().await?;
    let member = bot.get_chat_member(chat_id, bot_user.id).await?;

    Ok(matches!(
        member.status(),
        ChatMemberStatus::Member | ChatMemberStatus::Administrator | ChatMemberStatus::Owner
    ))
}

fn escape_markdown_v2(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|'
            | '{' | '}' | '.' | '!' => format!("\\{}", c),
            _ => c.to_string(),
        })
        .collect()
}

pub fn is_private_chat(msg: Message) -> bool {
    matches!(msg.chat.kind, ChatKind::Private(_))
}

pub fn is_public_chat(msg: Message) -> bool {
    matches!(msg.chat.kind, ChatKind::Public(_))
}
