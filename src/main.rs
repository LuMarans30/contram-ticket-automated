use clap::{Parser, command};
use color_eyre::{
    Result,
    eyre::{Context, Error},
};

use bot::*;
use user::*;
use utils::{booking::*, file_manager::FileManager};

mod bot;
mod user;
mod util;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about, color = clap::ColorChoice::Always, long_about = None)]
struct Args {
    /// Departure city (id, use --cities to print all available city ids)
    #[arg(short, long, required_unless_present("teloxide_token"))]
    from: Option<u32>,

    /// Destination city (id, use --cities to print all available city ids)
    #[arg(short, long, required_unless_present("teloxide_token"))]
    to: Option<u32>,

    /// Departure date (YYYY-MM-DD)
    #[arg(short, long, required_unless_present("teloxide_token"))]
    date: Option<String>,

    /// Path to JSON configuration file
    #[arg(short, long, default_value = "user.json")]
    config: String,

    /// Print all available cities
    #[arg(short, long, exclusive = true)]
    cities: bool,

    /// Headless mode
    #[arg(short = 'H', long)]
    headless: bool,

    /// Additional wait time in seconds
    #[arg(short, long, default_value_t = 5)]
    wait: u64,

    /// Optional Teloxide token (if set, overrides the need for from, to, and date)
    #[arg(long, env = "TELOXIDE_TOKEN", hide_env_values = true)]
    teloxide_token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    color_eyre::install()?;
    let args = Args::parse();

    if args.teloxide_token.is_none() {
        if args.from.is_none() || args.to.is_none() || args.date.is_none() {
            return Err(Error::msg("Missing required arguments"));
        } else {
            println!("Running in CLI mode");
        }
    } else {
        println!("Running in Telegram mode");
        bot_init().await;
        return Ok(());
    }

    let file_manager = FileManager::new(&args.config);

    if args.cities {
        println!("Available cities:");
        get_cities()
            .iter()
            .for_each(|c| println!("{} ({})", c.0, c.1));
        return Ok(());
    }

    let city_from = args.from.unwrap();
    let city_to = args.to.unwrap();
    let date = args.date.unwrap();

    let user = file_manager
        .get_user("CONTRAMCLIMODE".to_string())
        .wrap_err("Failed to read user configuration file")?;

    book_ticket(
        &user.user_data,
        city_from,
        city_to,
        date,
        Some(args.headless),
        Some(args.wait),
    )
    .await?;

    Ok(())
}
