use reqwest::Client;
use crate::{CowContext, Error};
use serde::Deserialize;
use log::error;
use std::error;

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct StoreConfig {
    pub store_hours: StoreHours
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct StoreHours {
    pub store_hours: Vec<StoreHoursWeek>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct StoreHoursWeek {
    pub note: Option<String>,
    pub description: String,
    pub sunday: String,
    pub monday: String,
    pub tuesday: String,
    pub wednesday: String,
    pub thursday: String,
    pub friday: String,
    pub saturday: String,
}

async fn fetch_hours(client: &Client) -> Result<StoreConfig, Box<dyn error::Error + Send + Sync>> {
    let response = client
        .get("https://svc.bkstr.com/store/config?storeName=ucmercedstore")
        .header("User-Agent", "Moogan/0.1.43")
        .send()
        .await?
        .text()
        .await?;

    let result: StoreConfig = serde_json::from_str(&*response)?;

    Ok(result)
}

fn read_hours(config: &StoreHours) -> Vec<(String, String)> {
    let mut output: Vec<(String, String)> = Vec::new();

    for week in config.store_hours.iter() {
        let week_str = if let Some(note) = week.note.as_ref() {
            format!("Note: {}\n\n", note)
        } else {
            String::new()
        };

        let schedule = format!("{}Sunday: {}\nMonday: {}\nTuesday: {}\nWednesday: {}\nThursday: {}\nFriday: {}\nSaturday: {}",
            week_str, week.sunday, week.monday, week.tuesday, week.wednesday, week.thursday, week.friday, week.saturday);

        output.push((week.description.clone(), schedule));
    }

    output
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en", "Get the times of the UC Merced store."),
    aliases("studentstore", "bookstore")
)]
pub async fn store(ctx: CowContext<'_>) -> Result<(), Error> {
    const TITLE: &str = "UC Merced University Store Hours";
    let loading_message = ctx.send(|m|
        m.embed(|e| e.title(TITLE).description("Now loading, please wait warmly..."))
    ).await?;

    let client = Client::new();
    match fetch_hours(&client).await {
        Ok(hours) => {
            let schedules = read_hours(&hours.store_hours);
            loading_message.edit(ctx, |m|
                {
                    m.embeds.clear();
                    m.embed(|e| e.title(TITLE).fields(schedules.iter().map(|o| {
                        let (description, hours) = o;
                        (description, hours, false)
                    })))
                }
            ).await?;
        }
        Err(ex) => {
            error!("Failed to load UCM store hours: {}", ex);
            loading_message.edit(ctx, |m|
                {
                    m.embeds.clear();
                    m.embed(|e| e.title(TITLE).description("Failed to load store hours. Try again later?"))
                }
            ).await?;
        }
    }

    Ok(())
}