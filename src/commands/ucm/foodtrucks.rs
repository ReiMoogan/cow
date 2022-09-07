use log::error;
use crate::CowContext;
use serenity::{
    client::Context,
    model::{
        channel::Message
    },
    framework::standard::{
        CommandResult,
        macros::{
            command
        }
    }
};
use scraper::{Html, Selector};

fn process_schedules(data: &str) -> Option<String> {
    let page = Html::parse_document(data);
    let image = Selector::parse("p img").unwrap();

    page.select(&image)
        .next()
        .map(|o| o.value().attr("src").unwrap().to_string())
}

#[poise::command(prefix_command, slash_command)]
#[aliases(foodtruck)]
#[description = "Get the current food truck schedule."]
pub async fn foodtrucks(ctx: &CowContext<'_>) -> CommandResult {
    const TITLE: &str = "Food Truck Schedule";

    let mut sent_msg = msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
        e
            .title(TITLE)
            .description("Now loading, please wait warmly...")
    })).await?;

    const URL: &str = "https://dining.ucmerced.edu/food-trucks";
    match reqwest::get(URL).await {
        Ok(response) => {
            match response.text().await {
                Ok(data) => {
                    let image_url = process_schedules(&*data);

                    if let Some(schedule) = image_url {
                        sent_msg.edit(&ctx.http, |m| m.embed(|e| {
                            e.title(TITLE).image(schedule)
                        })).await?;
                    } else {
                        sent_msg.edit(&ctx.http, |m| m.embed(|e| {
                            e.title(TITLE).description("Could not get any valid schedules... Did the website change layout?")
                        })).await?;
                        error!("Unable to read food truck website");
                    }
                }
                Err(ex) => {
                    sent_msg.edit(&ctx.http, |m| m.embed(|e| {
                        e.title(TITLE).description("UC Merced gave us weird data, try again later?")
                    })).await?;
                    error!("Failed to process calendar: {}", ex);
                }
            }
        }
        Err(ex) => {
            sent_msg.edit(&ctx.http, |m| m.embed(|e| {
                e.title(TITLE).description("Failed to connect to the UC Merced website, try again later?")
            })).await?;
            error!("Failed to get food truck schedule: {}", ex);
        }
    }

    Ok(())
}