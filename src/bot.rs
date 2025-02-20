use std::{io::ErrorKind, thread::sleep};

use chrono::{Days, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Europe::Rome;
use color_eyre::eyre::Error;
use teloxide::{
    dispatching::dialogue::{Dialogue, InMemStorage},
    prelude::*,
    utils::command::BotCommands,
};

use crate::{
    User,
    util::{get_stickers, send_cached_sticker},
    utils::file_manager::TelegramUser,
};

use crate::utils::booking::*;
use crate::utils::file_manager::FileManager;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveFirstName,
    ReceiveLastName {
        first_name: String,
    },
    ReceivePhoneNumber {
        first_name: String,
        last_name: String,
    },
    ReceivePersonalEmail {
        first_name: String,
        last_name: String,
        phone_number: String,
    },
    ReceiveInstitutionalEmail {
        first_name: String,
        last_name: String,
        phone_number: String,
        personal_email: String,
    },
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Supported commands:")]
enum Command {
    #[command(description = "Start interaction")]
    Start,
    #[command(description = "Begin user registration")]
    Createuser,
    #[command(description = "Get my user information")]
    Getuser,
    #[command(description = "Delete my user information")]
    Deleteuser,
    #[command(description = "Get available cities")]
    Getcities,
    #[command(description = "Book a ticket")]
    Bookticket(String),
    #[command(description = "Show help menu")]
    Help,
    #[command(description = "Cancel current operation")]
    Cancel,
}

pub async fn bot_init() {
    let bot = Bot::from_env();
    bot.set_my_commands(Command::bot_commands())
        .await
        .expect("Failed to set commands");

    let handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(
            dptree::entry()
                .filter(|msg: Message| msg.text().map(|t| t.starts_with('/')).unwrap_or(false))
                .endpoint(handle_invalid_command),
        )
        .branch(dptree::case![State::Start].endpoint(handle_unexpected_messages))
        .branch(dptree::case![State::ReceiveFirstName].endpoint(receive_first_name))
        .branch(dptree::case![State::ReceiveLastName { first_name }].endpoint(receive_last_name))
        .branch(
            dptree::case![State::ReceivePhoneNumber {
                first_name,
                last_name
            }]
            .endpoint(receive_phone_number),
        )
        .branch(
            dptree::case![State::ReceivePersonalEmail {
                first_name,
                last_name,
                phone_number
            }]
            .endpoint(receive_personal_email),
        )
        .branch(
            dptree::case![State::ReceiveInstitutionalEmail {
                first_name,
                last_name,
                phone_number,
                personal_email
            }]
            .endpoint(receive_institutional_email),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn get_username(msg: Message) -> Result<String, Error> {
    msg.from
        .expect("Could not identify user")
        .username
        .ok_or(Error::msg("Could not identify username"))
}

async fn handle_start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    dialogue.exit().await?;
    bot.send_message(msg.chat.id, "👋 Welcome!\nUse /help for commands")
        .await?;
    Ok(())
}

async fn handle_createuser(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Let's start! What's your first name?")
        .await?;
    dialogue.update(State::ReceiveFirstName).await?;
    Ok(())
}

async fn handle_getuser(bot: Bot, msg: Message) -> HandlerResult {
    let file_manager = FileManager::new("users.json");
    let username = get_username(msg.clone()).await?;

    match file_manager.get_user(username) {
        Ok(user) => {
            bot.send_message(msg.chat.id, user.user_data.to_string())
                .await?;
        }
        Err(e) => {
            let error_message = match e.kind() {
                ErrorKind::NotFound => "❌ No user registered yet!\nUse /createuser to register.",
                _ => "❌ Failed to access user data. Please re-register with /createuser",
            };
            bot.send_message(msg.chat.id, error_message).await?;
        }
    }
    Ok(())
}

async fn handle_deleteuser(bot: Bot, msg: Message) -> HandlerResult {
    let mut file_manager = FileManager::new("users.json");
    let username = get_username(msg.clone()).await?;

    match file_manager.delete_user(username) {
        Ok(()) => {
            send_cached_sticker(
                bot.clone(),
                msg.clone(),
                get_stickers().get("bye").unwrap().to_string(),
            )
            .await;
            bot.send_message(
                msg.chat.id,
                "✅ User data deleted successfully!\nUse /createuser to register again.",
            )
            .await?;
        }
        Err(e) => {
            let error_message = match e.kind() {
                std::io::ErrorKind::NotFound => "❌ No user registered to delete.",
                _ => "❌ Failed to delete user data. Please try again.",
            };
            bot.send_message(msg.chat.id, error_message).await?;
        }
    }
    Ok(())
}

async fn handle_getcities(bot: Bot, msg: Message) -> HandlerResult {
    let cities = get_cities();
    let cities_list = cities
        .iter()
        .map(|(name, id)| format!("{}. {}", id, name))
        .collect::<Vec<_>>()
        .join("\n");

    bot.send_message(
        msg.chat.id,
        format!("Available cities (ID. name):\n{}", cities_list),
    )
    .await?;
    Ok(())
}

async fn handle_bookticket(bot: Bot, msg: Message, args: String) -> HandlerResult {
    // Argument parsing and validation
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() != 3 {
        send_cached_sticker(
            bot.clone(),
            msg.clone(),
            get_stickers()
                .get("error_cat_invalid_syntax")
                .unwrap()
                .to_string(),
        )
        .await;
        bot.send_message(
            msg.chat.id,
            "❌ Invalid command syntax.\nUsage: /bookticket <from> <to> <date> (YYYY-MM-DD)",
        )
        .await?;
    }

    let from = parts[0].parse::<u32>();
    let to = parts[1].parse::<u32>();
    let date = parts[2].to_string();
    let cities = get_cities();

    let file_manager = FileManager::new("users.json");

    let user = match file_manager.get_user(get_username(msg.clone()).await?) {
        Ok(user) => {
            bot.send_message(msg.chat.id, user.user_data.to_string())
                .await?;
            user
        }
        Err(e) => {
            let error_message = match e.kind() {
                // Handle file not found specifically
                ErrorKind::NotFound => "❌ No user registered yet!\nUse /createuser to register.",
                _ => "❌ Failed to access user data. Please try again later.",
            };

            bot.send_message(msg.chat.id, error_message).await?;
            return Ok(());
        }
    };

    if from.is_err() || to.is_err() {
        send_message(
            bot.clone(),
            msg.clone(),
            "❌ Arrival city ID not found.".to_string(),
            Some("error_cat_invalid_syntax"),
        )
        .await;
        send_cached_sticker(
            bot.clone(),
            msg.clone(),
            get_stickers()
                .get("error_cat_invalid_syntax")
                .unwrap()
                .to_string(),
        )
        .await;
        bot.send_message(msg.chat.id, "❌ Invalid city ID.").await?;
        return Ok(());
    }

    let id_from = from.unwrap();
    let id_to = to.unwrap();

    let city_from = match get_city_by_id(&cities, id_from) {
        Some(city) => city,
        None => {
            send_message(
                bot.clone(),
                msg.clone(),
                "❌ Departure city ID not found.".to_string(),
                Some("error_cat_invalid_syntax"),
            )
            .await;
            return Ok(());
        }
    };

    let city_to = match get_city_by_id(&cities, id_to) {
        Some(city) => city,
        None => {
            send_message(
                bot.clone(),
                msg.clone(),
                "❌ Arrival city ID not found.".to_string(),
                Some("error_cat_invalid_syntax"),
            )
            .await;
            return Ok(());
        }
    };

    bot.send_message(
        msg.chat.id,
        format!(
            "🚌 Booking ticket from {} to {} on {}.\nPlease wait... ⏳",
            city_from, city_to, date
        ),
    )
    .await?;

    let parsed_date = match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            send_message(
                bot.clone(),
                msg.clone(),
                "❌ Invalid date format (use YYYY-MM-DD)".to_string(),
                Some("error_cat_invalid_syntax"),
            )
            .await;
            return Ok(());
        }
    };

    // Create DateTime in Rome timezone at midnight
    let parsed_datetime = Rome
        .from_local_datetime(&parsed_date.and_hms_opt(0, 0, 0).unwrap())
        .unwrap();

    // Calculate booking open time (7 days before at midnight)
    let booking_open_date = parsed_date.checked_sub_days(Days::new(7)).unwrap();
    let booking_open_datetime = Rome
        .from_local_datetime(&booking_open_date.and_hms_opt(0, 0, 0).unwrap())
        .unwrap();

    // Get current time in Rome
    let now = Utc::now().with_timezone(&Rome);

    if parsed_datetime <= now + Days::new(1) {
        send_message(
            bot.clone(),
            msg.clone(),
            "❌ Invalid date".to_string(),
            Some("error_cat_invalid_syntax"),
        )
        .await;
        return Ok(());
    }

    // Wait until booking opens every minute
    if now < booking_open_datetime {
        send_message(
            bot.clone(),
            msg.clone(),
            format!(
                "Waiting until {} for booking to open...",
                booking_open_datetime.date_naive()
            ),
            Some("hourglass"),
        )
        .await;
        while Utc::now().with_timezone(&Rome) < booking_open_datetime {
            println!("...");
            sleep(Duration::seconds(60).to_std().unwrap());
        }
        sleep(Duration::seconds(1).to_std().unwrap()); // Buffer for precision issues
    }

    let response = match book_ticket(&user.user_data, id_from, id_to, date, Some(false), None).await
    {
        Ok(r) => r,
        Err(e) => {
            send_message(
                bot.clone(),
                msg.clone(),
                format!("❌ Booking failed: {}", e),
                Some("error_cat"),
            )
            .await;
            return Ok(());
        }
    };

    println!("Response from book_ticket: {}", response);

    send_cached_sticker(
        bot.clone(),
        msg.clone(),
        get_stickers().get("success_cat").unwrap().to_string(),
    )
    .await;
    bot.send_message(msg.chat.id, response).await?;
    Ok(())
}

async fn handle_help(bot: Bot, msg: Message) -> HandlerResult {
    let help_text = Command::descriptions().to_string();
    bot.send_message(msg.chat.id, help_text).await?;
    Ok(())
}

async fn handle_cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    handle_stop(bot, dialogue, msg).await
}

async fn send_message(bot: Bot, msg: Message, response: String, sticker_id: Option<&str>) {
    bot.send_message(msg.chat.id, response)
        .await
        .log_on_error()
        .await;
    if let Some(sticker_id) = sticker_id {
        send_cached_sticker(
            bot,
            msg,
            get_stickers().get(sticker_id).unwrap().to_string(),
        )
        .await;
    }
}

// Main command handler
async fn handle_command(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
) -> HandlerResult {
    match cmd {
        Command::Start => handle_start(bot, dialogue, msg).await,
        Command::Createuser => handle_createuser(bot, dialogue, msg).await,
        Command::Getuser => handle_getuser(bot, msg).await,
        Command::Deleteuser => handle_deleteuser(bot, msg).await,
        Command::Getcities => handle_getcities(bot, msg).await,
        Command::Bookticket(args) => handle_bookticket(bot, msg, args).await,
        Command::Help => handle_help(bot, msg).await,
        Command::Cancel => handle_cancel(bot, dialogue, msg).await,
    }
}

async fn handle_invalid_command(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "⚠️ Invalid command syntax. Use /help for command usage.",
    )
    .await?;
    Ok(())
}

async fn handle_unexpected_messages(bot: Bot, _: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "⚠️ Please use commands to interact with me!\n\nAvailable commands:\n/help - Show all commands",
    ).await?;
    Ok(())
}

async fn handle_stop(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match dialogue.get().await? {
        Some(State::Start) => {
            send_cached_sticker(
                bot.clone(),
                msg.clone(),
                get_stickers().get("sleepy_cat").unwrap().to_string(),
            )
            .await;
            bot.send_message(msg.chat.id, "ℹ️ No active operation to cancel.")
                .await?;
        }
        _ => {
            dialogue.exit().await?;
            bot.send_message(
                msg.chat.id,
                "✅ Operation cancelled. All progress has been reset.",
            )
            .await?;
        }
    }
    Ok(())
}

async fn receive_first_name(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    if let Some(first_name) = msg.text() {
        bot.send_message(msg.chat.id, "What's your last name?")
            .await?;
        dialogue
            .update(State::ReceiveLastName {
                first_name: first_name.into(),
            })
            .await?;
    }
    Ok(())
}

async fn receive_last_name(
    bot: Bot,
    dialogue: MyDialogue,
    first_name: String,
    msg: Message,
) -> HandlerResult {
    if let Some(last_name) = msg.text() {
        bot.send_message(msg.chat.id, "What's your phone number?")
            .await?;
        dialogue
            .update(State::ReceivePhoneNumber {
                first_name,
                last_name: last_name.into(),
            })
            .await?;
    }
    Ok(())
}

async fn receive_institutional_email(
    bot: Bot,
    dialogue: MyDialogue,
    first_name: String,
    last_name: String,
    phone_number: String,
    personal_email: String,
    msg: Message,
) -> HandlerResult {
    if let Some(institutional_email) = msg.text() {
        let user = User::new(
            personal_email,
            first_name,
            last_name,
            institutional_email.into(),
            phone_number,
        );

        println!("{}", user);

        let mut file_manager = FileManager::new("users.json");
        let telegram_user = TelegramUser {
            username: get_username(msg.clone()).await?,
            user_data: user,
        };

        if file_manager.add_user(telegram_user).is_ok() {
            bot.send_message(msg.chat.id, "✅ User registered successfully!")
                .await?;
        } else {
            bot.send_message(msg.chat.id, "❌ Failed to save user data.")
                .await?;
        }
        dialogue.exit().await?;
    }
    Ok(())
}

async fn receive_phone_number(
    bot: Bot,
    dialogue: MyDialogue,
    first_name: String,
    last_name: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(phone_number) => {
            bot.send_message(msg.chat.id, "What's your personal email?")
                .await?;
            dialogue
                .update(State::ReceivePersonalEmail {
                    first_name,
                    last_name,
                    phone_number: phone_number.into(),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }
    Ok(())
}

async fn receive_personal_email(
    bot: Bot,
    dialogue: MyDialogue,
    first_name: String,
    last_name: String,
    phone_number: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(personal_email) => {
            bot.send_message(msg.chat.id, "What's your institutional email?")
                .await?;
            dialogue
                .update(State::ReceiveInstitutionalEmail {
                    first_name,
                    last_name,
                    phone_number,
                    personal_email: personal_email.into(),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }
    Ok(())
}
