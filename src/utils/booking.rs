use crate::user::User;
use color_eyre::eyre::Error;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use thirtyfour::prelude::*;

#[derive(Debug, Deserialize)]
struct Fermata {
    #[serde(rename = "nome")]
    name: String,
    #[serde(rename = "fermataID")]
    id: u32,
}

pub async fn get_cities() -> Result<Vec<(String, u32)>, Error> {
    let api_url = "https://marcheroma.contram.it/api/fermata/partenza";
    let client = Client::new();
    let response = client.get(api_url).send().await?;

    let cities = match response.json::<Vec<Fermata>>().await {
        Ok(json) => json.into_iter().map(|f| (f.name, f.id)).collect(),
        Err(_) => {
            println!("Error fetching cities, using default cities");
            vec![
                ("Camerino".to_string(), 24),
                ("Ancona Piazza Cavour".to_string(), 38),
                ("Ancona Stazione F.S.".to_string(), 39),
                ("Civitanova Marche Via Sonnino".to_string(), 42),
                ("Porto San Giorgio".to_string(), 53),
            ]
        }
    };

    let mut sorted_cities = cities;
    sorted_cities.sort_by_key(|&(_, id)| id);
    Ok(sorted_cities)
}

pub fn validate_city_id(cities: &[(String, u32)], target_id: u32) -> Result<&str, Error> {
    cities
        .binary_search_by(|(_, id)| id.cmp(&target_id))
        .map(|idx| &cities[idx].0[..])
        .map_err(|_| Error::msg(format!("Invalid city ID: {}", target_id)))
}

pub async fn find_and_wait(driver: &WebDriver, by: By, text: String) -> Result<WebElement, Error> {
    let element = driver
        .query(by)
        .with_text(text)
        .and_clickable()
        .first()
        .await?;
    element.scroll_into_view().await?;
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
) -> Result<String, Error> {
    // Fetch cities and validate IDs
    let cities = get_cities()
        .await
        .expect("❌ Failed to fetch cities. Please try again later.");
    let city_from = validate_city_id(&cities, from_id).expect("❌ Departure city not found");
    let city_to = validate_city_id(&cities, to_id).expect("❌ Arrival city not found");
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

    // Wait for the booking button to be clickable and click it
    let btn_submit = find_and_wait(&driver, By::Tag("button"), "Prenota".to_string()).await?;
    btn_submit.click().await?;
    println!("Submitted booking form");

    // Explicit wait for navigation to cart
    driver
        .goto("https://marcheroma.contram.it/Home/RitornaCarrello?")
        .await?;
    println!("Navigated to cart");

    // Fill form fields
    fill_form_fields(&driver, user).await?;

    // Final submission
    let btn_submit = find_and_wait(
        &driver,
        By::Tag("button"),
        "Procedi all'acquisto".to_string(),
    )
    .await?;
    btn_submit.click().await?;

    // Confirm purchase
    let btn_confirm =
        find_and_wait(&driver, By::Tag("button"), "Conferma acquisto".to_string()).await?;
    btn_confirm.click().await?;
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
