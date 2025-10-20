## Telegram to Pylon sync bot

### Overview

A Telegram bot that syncs with Pylon to create issues from Telegram messages.

### Setup

#### Creating a Telegram Bot

1. Message [@BotFather](https://t.me/botfather) on Telegram
2. Send `/newbot` and follow the prompts
3. Save the bot token provided

#### Configuration

Create a `settings.toml` file:

```toml
bot_admins = ["your_telegram_username"]

[tg_chats_to_pylon_accounts]
# Will be populated automatically when bot is added to chats
```

Set environment variables:

```bash
TELOXIDE_TOKEN=your_bot_token_from_botfather
PYLON_API_TOKEN=your_pylon_api_token
```

#### Running the Bot

```bash
cargo run
```

Optional flags:
- `--settings-path <PATH>` - Path to settings file (default: `./settings.toml`)
- `--logs-path <PATH>` - Directory for log files

### Usage

#### Add the bot to a chat

1. Add the bot to a Telegram group
2. [Link the chat to a Pylon account](#linking-a-chat-to-pylon)

#### Create an issue

1. Reply to any message with `/issue <title>`
2. The bot will create a Pylon issue with the replied message content

Example:
```
/issue Bug in login flow
```

#### Admin Commands

Admin commands work only in private chats with authorized users (configured in `bot_admins`).

- `/help` - Show available commands
- `/active` - List all active chats linked to Pylon accounts
- `/unlinked` - List chats not yet linked to a Pylon account
- `/orphans` - List configured chats where the bot is no longer a member
- `/link` - Link a Telegram chat to a Pylon account (interactive)

##### Linking a Chat to Pylon

1. Send `/link` to the bot in a private chat
2. Select the chat from the inline keyboard
3. Enter the Pylon account ID when prompted
