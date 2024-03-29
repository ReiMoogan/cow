use chrono::{Datelike, Duration, Local};
use tracing::error;
use crate::{CowContext, Error};
use scraper::{Html, Selector};
use regex::Regex;

fn process_schedules(data: &str) -> Option<String> {
    let now = Local::now();

    let monday = if now.weekday() == chrono::Weekday::Sun {
        now + Duration::days(1) // day after sunday
    } else {
        now - Duration::days(now.weekday().num_days_from_monday() as i64) // days before to monday
    };

    let monday_date = format!("{}.{}", monday.month(), monday.day());

    let page = Html::parse_document(data);
    let image = Selector::parse("img").unwrap();

    let links = page.select(&image)
        .map(|o| o.value().attr("src").unwrap().to_string())
        // filter out the logo, food truck image, translate icon, and svg images
        .filter(|o| !o.contains("svg") && !o.contains("logo") && !o.contains("translate") && !o.contains("food_trucks_20211006-4"))
        .collect::<Vec<String>>();

    let day = links.iter().find(|o| o.contains(&monday_date));

    // Returning back to the old way lol
    if day.is_some() {
        // error!("Found exact");
        return day.map(|o| o.to_string());
    }

    if day.is_some() {
        day.map(|o| o.to_string())
    } else {
        None
    }
}

/// Get the latest food truck schedule posted on the UC Merced website.
#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the current food truck schedule."),
    aliases("foodtruck"),
    discard_spare_arguments
)]
pub async fn foodtrucks(ctx: CowContext<'_>) -> Result<(), Error> {
    const TITLE: &str = "Food Truck Schedule";

    let sent_msg = ctx.send(|m| {
        m.embeds.clear();
        m.embed(|e| {
            e
                .title(TITLE)
                .description("Now loading, please wait warmly...")
        })
    }).await?;

    const URL: &str = "https://dining.ucmerced.edu/food-trucks";
    match reqwest::get(URL).await {
        Ok(response) => {
            match response.text().await {
                Ok(data) => {
                    let image_url = process_schedules(&data);

                    if let Some(schedule) = image_url {
                        sent_msg.edit(ctx, |m| {
                            m.embeds.clear();
                            m.embed(|e| {
                                e.title(TITLE).image(schedule)
                            })
                        }).await?;
                    } else {
                        sent_msg.edit(ctx, |m| {
                            m.embeds.clear();
                            m.embed(|e| {
                                e.title(TITLE).description("Could not get any valid schedules... either the school didn't update their website, or they changed their layout. If you see a valid schedule on https://dining.ucmerced.edu/food-trucks, please ping DoggySazHi!")
                            })
                        }).await?;
                        error!("Unable to read food truck website");
                    }
                }
                Err(ex) => {
                    sent_msg.edit(ctx, |m| {
                        m.embeds.clear();
                        m.embed(|e| {
                            e.title(TITLE).description("UC Merced gave us weird data, try again later?")
                        })
                    }).await?;
                    error!("Failed to process calendar: {}", ex);
                }
            }
        }
        Err(ex) => {
            sent_msg.edit(ctx, |m| {
                m.embeds.clear();
                m.embed(|e| {
                    e.title(TITLE).description("Failed to connect to the UC Merced website, try again later?")
                })
            }).await?;
            error!("Failed to get food truck schedule: {}", ex);
        }
    }

    Ok(())
}