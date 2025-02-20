use std::collections::HashMap;

use teloxide::{
    Bot,
    prelude::{OnError, Request, Requester},
    types::{InputFile, Message},
};

pub async fn send_cached_sticker(bot: Bot, msg: Message, sticker_id: String) {
    bot.send_sticker(msg.chat.id, InputFile::file_id(sticker_id))
        .send()
        .await
        .log_on_error()
        .await;
}

pub fn get_stickers() -> HashMap<String, String> {
    HashMap::from([
        (
            String::from("hourglass"),
            String::from(
                "CAACAgIAAxkBAAIBZV9sZQzDh8UkI9c4E1j0m7aB6tV5QAAJ9kA8jQqDZ7l3K7Gm0dS0t8A8oF9gEAA",
            ),
        ),
        (
            String::from("error_cat"),
            String::from(
                "CAACAgIAAxkBAAExajpnoTwhNF3vQnogLnDZSqgFMINMAgACNQEAAhZ8aAN0t5Pt54TmvDYE",
            ),
        ),
        (
            String::from("error_cat_invalid_syntax"),
            String::from("CAACAgQAAxkBAAExajZnoTwBaB1ps8dz6iLpqsFPjfIjZAACgQADWG21LihYUUl5XCWvNgQ"),
        ),
        (
            String::from("success_cat"),
            String::from(
                "CAACAgIAAxkBAAExaipnoTuG7tQy2s1y501C9r49-WAzMgACQAEAAhZ8aAPOt9pjb9XRXTYE",
            ),
        ),
        (
            String::from("sleepy_cat"),
            String::from(
                "CAACAgIAAxkBAAExakpnoT7L46_ogEccuszOh0k221KiUwACQw8AAgPLqUoG1QXSYyp97jYE",
            ),
        ),
        (
            String::from("bye"),
            String::from("CAACAgIAAxkBAAExamZnoUaX6MQ0DtOa8nMWCZWQx_m8swACUgADQbVWDAIQ4mRpfw9yNgQ"),
        ),
    ])
}
