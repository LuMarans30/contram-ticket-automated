use bot::*;
use color_eyre::eyre::Error;
use user::*;

mod bot;
mod user;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Error> {
    color_eyre::install()?;

    bot_init().await;

    Ok(())
}
