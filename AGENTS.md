# Autofishbot Agent Documentation

## Architecture Overview

The `autofishbot_rs` is a Rust-based Discord self-bot designed to automate interactions with the "Virtual Fisher" game. It uses `tokio` for async runtime, `reqwest` for HTTP interactions (commands), and `tungstenite` for Gateway (WebSocket) events.

### Key Modules

- **`discord`**: Handles Discord API interactions.
  - `client.rs`: HTTP client for sending commands and interactions.
  - `gateway.rs`: WebSocket client for receiving events.
  - `types.rs`: Struct definitions for Discord objects (Messages, Embeds, Commands).

- **`engine`**: Core logic of the bot.
  - `bot.rs`: The main state machine. It switches between states like `Fishing`, `Captcha`, and `Exploration`.
  - `explorer.rs`: **New Module**. Responsible for discovering commands, executing them, and parsing the results to gather game data (Shops, Items, Buffs). Includes fallback logic for hardcoded commands if discovery fails due to rate limits.
  - `parser.rs`: Contains Regex patterns and logic to parse Discord Embeds into structured data (`ShopItem`, `CatchEvent`).
  - `database.rs`: **Updated Module**. Uses SQLite (`sqlx`) to persist data.
    - Stores: `fish`, `catch_history`, `player_snapshots`, `shop_items`, `game_entities`, `command_registry`.
    - Uses `DELETE` journal mode to ensure compatibility in restrictive environments.

- **`tui`**: Terminal User Interface using `ratatui`.

## Data Gathering & Exploration

The bot now features an `Exploration` mode (managed by `explorer.rs`).

1.  **Discovery**: It fetches all available slash commands from the guild. If this fails (e.g. 429 Rate Limit), it falls back to a hardcoded list of known commands.
2.  **Execution**: It iterates through a priority list of commands (e.g., `/shop`, `/fishdex`, `/buffs`).
    - Handles command options and subcommands by auto-selecting the first available subcommand if none is specified but required.
3.  **Parsing**: When a message is received in response to a command, `parser.rs` analyzes the Embeds.
    - **Shops**: Parses item names, prices, and descriptions.
    - **Lists**: Parses generic lists of buffs or quests.
4.  **Storage**: Discovered items are upserted into the `shop_items` and `game_entities` tables in SQLite.

## Database Schema

- **`shop_items`**: `name`, `shop_type`, `price`, `currency`, `description`, `stock`.
- **`game_entities`**: Generic storage for `Buffs`, `Quests`, etc. (`entity_type`, `name`, `details`).
- **`command_registry`**: Tracks which commands exist and when they were last executed.

## Development Tips

- **Adding Parsers**: Update `engine/parser.rs` with new Regex patterns for unseen menus.
- **Extending Exploration**: Add new command names to the `target_commands` list in `engine/explorer.rs`.
- **Headless Mode**: Use `cargo run --bin headless` to run without the TUI (useful for debugging/logging).

## Known Issues & Future Improvements

1.  **Rate Limiting**: The bot hits Discord rate limits (`429`) frequently during startup when Discovery, Scheduler, and Fishing logic trigger simultaneously.
    - *Workaround*: Added startup delays and staggered task execution.
    - *Improvement Needed*: Implement a global rate-limit bucket manager in `DiscordClient` that queues requests instead of firing them blindly.
2.  **Command Structure Discovery**: If discovery fails, the bot uses a hardcoded command structure. If the game updates its command arguments (e.g., adding a required option to `/shop`), the fallback will fail with `400 Bad Request`.
    - *Improvement Needed*: A way to persist discovered command structures to disk so the bot remembers them across restarts even if rate-limited.
3.  **Shop Logic**: The fallback logic assumes `/shop` has a `view` subcommand or works without args. This might need adjustment based on live game changes.
4.  **Scheduler**: The scheduler currently just fires tasks. It should ideally check if the bot is in a "busy" state (like Exploration) to avoid conflict.

## Warnings

- The bot uses a user token (Self-Bot). This is against Discord TOS. Use at your own risk.
- Rate limits are handled naively. `DiscordClient` logs 429s but does not automatically back off perfectly.
