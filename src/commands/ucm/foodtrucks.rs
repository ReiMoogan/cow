use chrono::{Datelike, Duration, Local};
use poise::CreateReply;
use tracing::error;
use crate::{CowContext, Error};
use scraper::{Html, Selector};
use regex::Regex;
use serenity::all::CreateEmbed;

fn process_schedules(data: &str) -> Option<String> {
    let now = Local::now();

    let monday = if now.weekday() == chrono::Weekday::Sun {
        now + Duration::days(1) // day after sunday
    } else {
        now - Duration::days(now.weekday().num_days_from_monday() as i64) // days before to monday
    };

    let monday_date = format!("{}-{}", monday.month(), monday.day());

    let page = Html::parse_document(data);
    let image = Selector::parse("img").unwrap();

    let links = page.select(&image)
        .map(|o| o.value().attr("src").unwrap().to_string())
        // filter out the logo, food truck image, translate icon, and svg images
        .filter(|o| !o.contains("svg") && !o.contains("logo") && !o.contains("translate") && !o.contains("food_trucks_20211006-4"))
        .collect::<Vec<String>>();

    // let day = links.iter().find(|o| o.contains(&monday_date));
    //
    // if day.is_some() {
    //     error!("Found exact");
    //     return day.map(|o| o.to_string());
    // }

    // ok so they probably changed their naming scheme :reimudizzy:
    // regex to match numbers from the link https://dining.ucmerced.edu/sites/dining.ucmerced.edu/files/page/images/llh-ucm_9-18-10-13_002_page_1.png
    let re = Regex::new(r"(\d+).*?(\d+).*(\d+)").unwrap();
    // first two numbers are a date, last number is the page number
    // Also absolutely poor programming with unwraps everywhere
    error!("Checking dates");
    let day = links.iter().find(|o| {
        if let Some(captures) = re.captures(o) {
            let date = format!("{}-{}-{}", now.year(), captures.get(1).unwrap().as_str(), captures.get(2).unwrap().as_str());
            let page = captures.get(3).unwrap().as_str();
            let chrono_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").unwrap();
            let chrono_date_with_page = chrono_date + Duration::days((page.parse::<i64>().unwrap() - 1) * 7);
            let formatted_final_date = format!("{}-{}", chrono_date_with_page.month(), chrono_date_with_page.day());
            error!("Checking dates {} {} {}", date, page, formatted_final_date);
            formatted_final_date == monday_date
        } else {
            false
        }
    });

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

    let sent_msg = ctx.send(CreateReply::default().embed(CreateEmbed::new().title(TITLE).description("Now loading, please wait warmly..."))).await?;

    const URL: &str = "https://dining.ucmerced.edu/food-trucks";
    match reqwest::get(URL).await {
        Ok(response) => {
            match response.text().await {
                Ok(data) => {
                    let image_url = process_schedules(&data);

                    if let Some(schedule) = image_url {
                        sent_msg.edit(ctx, CreateReply::default().embed(CreateEmbed::new().title(TITLE).image(schedule))).await?;
                    } else {
                        sent_msg.edit(ctx, CreateReply::default().embed(CreateEmbed::new().title(TITLE).description("Could not get any valid schedules... either the school didn't update their website, or they changed their layout. If you see a valid schedule on https://dining.ucmerced.edu/food-trucks, please ping DoggySazHi!"))).await?;
                        error!("Unable to read food truck website");
                    }
                }
                Err(ex) => {

                    sent_msg.edit(ctx, CreateReply::default().embed(CreateEmbed::new().title(TITLE).description("UC Merced gave us weird data, try again later?"))).await?;
                    error!("Failed to process calendar: {}", ex);
                }
            }
        }
        Err(ex) => {
            sent_msg.edit(ctx, CreateReply::default().embed(CreateEmbed::new().title(TITLE).description("Failed to connect to the UC Merced website, try again later?"))).await?;
            error!("Failed to get food truck schedule: {}", ex);
        }
    }

    Ok(())
}