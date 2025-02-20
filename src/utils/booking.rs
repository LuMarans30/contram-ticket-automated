use color_eyre::eyre::Error;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use thirtyfour::{prelude::*, support::sleep};

use std::time::Duration;

use crate::user::User;

#[derive(Debug, Deserialize)]
struct Fermata {
    #[serde(rename = "nome")]
    name: String,
    #[serde(rename = "fermataID")]
    id: u32,
}

pub fn get_cities() -> Vec<(String, u32)> {
    let api_url = "https://marcheroma.contram.it/api/fermata/partenza";
    let client = Client::builder().build().unwrap();

    let response = client.get(api_url).send();

    let mut cities = match response.and_then(|r| r.json::<Vec<Fermata>>()) {
        Ok(json) => json.iter().map(|f| (f.name.clone(), f.id)).collect(),
        Err(e) => {
            println!("Error fetching cities: {}", e);
            println!("Using default cities");
            // Default cities with their IDs
            vec![
                (String::from("Camerino"), 24),
                (String::from("Ancona Piazza Cavour"), 38),
                (String::from("Ancona Stazione F.S."), 39),
                (String::from("Civitanova Marche Via Sonnino"), 42),
                (String::from("Porto San Giorgio"), 53),
            ]
        }
    };

    cities.sort_by_key(|&(_, id)| id);
    cities
}

pub fn get_city_by_id(cities: &[(String, u32)], target_id: u32) -> Option<&str> {
    cities
        .binary_search_by(|(_, id)| id.cmp(&target_id))
        .ok()
        .map(|idx| &cities[idx].0[..])
}

pub async fn find_and_wait_clickable(
    driver: &WebDriver,
    by: By,
    text: String,
) -> Result<WebElement, Error> {
    let element = driver
        .query(by)
        .with_text(text)
        .and_clickable()
        .first()
        .await?;
    Ok(element)
}

pub async fn fill_form_fields(driver: &WebDriver, user: &User) -> Result<(), Error> {
    let person_value = serde_json::to_value(user)?;
    if let Value::Object(map) = person_value {
        for (field, value) in map {
            if let Value::String(s) = value {
                fill_field(driver, &field, &s).await?;
            } else {
                return Err(Error::msg(format!("Field {} is not a string", field)));
            }
        }
    }
    Ok(())
}

pub async fn fill_field(driver: &WebDriver, field: &str, value: &str) -> Result<(), Error> {
    let element = driver.find(By::Name(field)).await?;
    element.send_keys(value).await?;
    Ok(())
}

pub async fn book_ticket(
    user: &User,
    from_id: u32,
    to_id: u32,
    date: String,
    is_headless: Option<bool>,
    wait_time: Option<u64>,
) -> Result<String, Error> {
    let duration_wait = Duration::from_secs(wait_time.unwrap_or(5));

    // Fill form from configuration file
    println!("Loaded User JSON configuration: \n{:?}", user);

    let cities = get_cities();

    // Validate cities
    let city_from = get_city_by_id(&cities, from_id).expect("Invalid departure city");
    let city_to = get_city_by_id(&cities, to_id).expect("Invalid arrival city");

    println!("Departing from {} to {} on {}", city_from, city_to, date);

    // Initialize WebDriver
    let mut caps = DesiredCapabilities::firefox();
    if is_headless.unwrap_or(true) {
        caps.set_headless()?;
        println!("Headless mode enabled");
    }

    let driver = WebDriver::new("http://localhost:4444", caps).await?;

    // Build and visit URL
    let url = format!(
        "https://marcheroma.contram.it/home/Ricerca?PartenzaID={}&DestinazioneID={}&DataPartenza={}&NumeroStudenti=1&NumeroAdulti=0",
        from_id, to_id, date
    );
    driver.goto(&url).await?;

    println!("Loaded URL: {}", url);

    // /html/body/div[2]/div[6]/div/div/table/tbody/tr/td[4]/form/button
    let btn_submit =
        find_and_wait_clickable(&driver, By::Tag("button"), "Prenota".to_string()).await?;
    btn_submit.click().await?;
    println!("Submitted booking form");
    sleep(duration_wait).await;

    // Go to cart
    driver
        .goto("https://marcheroma.contram.it/Home/RitornaCarrello?")
        .await?;

    println!("Navigated to cart");

    fill_form_fields(&driver, user).await?;

    sleep(duration_wait).await;

    // Final submission
    // /html/body/div[2]/form/div/button
    let btn_submit = find_and_wait_clickable(
        &driver,
        By::Tag("button"),
        "Procedi all'acquisto".to_string(),
    )
    .await?;
    btn_submit.scroll_into_view().await?;
    btn_submit.click().await?;

    sleep(duration_wait).await;

    let btn_submit =
        find_and_wait_clickable(&driver, By::Tag("button"), "Conferma acquisto".to_string())
            .await?;
    btn_submit.scroll_into_view().await?;
    btn_submit.click().await?;

    println!(
        "Submitted final booking form, an email will be sent to: {}",
        user.get_email()
    );

    driver.quit().await?;

    Ok(format!(
        "Ticket booked from {} to {} on {}\nAn email will be sent to: {}",
        city_from,
        city_to,
        date,
        user.get_email()
    ))
}
