use chrono::Datelike;
use tracing::error;
use crate::{CowContext, Error};
use crate::commands::ucm::libcal_models::Calendar;

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the hours for the Kolligian Library."),
    aliases("lib"),
    discard_spare_arguments
)]
pub async fn library(ctx: CowContext<'_>) -> Result<(), Error> {
    let date = chrono::offset::Local::now();
    let url = format!("https://api3.libcal.com/api_hours_grid.php?iid=4052&lid=0&format=json&date={}-{:0>2}-{:0>2}", date.year(), date.month(), date.day());
    match reqwest::get(url).await {
        Ok(response) => {
            match response.json::<Calendar>().await {
                Ok(data) => {
                    ctx.send(|m| {
                        let library = &data.locations[0].weeks[0];
                        let start_date = chrono::NaiveDate::parse_from_str(&library.sunday.date, "%Y-%m-%d").unwrap();
                        m.embeds.clear();
                        m.embed(|e| {
                            e
                                .title("Kolligian Library Hours")
                                .description(format!("For the week of {}", start_date.format("%B %d, %Y")))
                                .field("Sunday", &library.sunday.rendered, false)
                                .field("Monday", &library.monday.rendered, false)
                                .field("Tuesday", &library.tuesday.rendered, false)
                                .field("Wednesday", &library.wednesday.rendered, false)
                                .field("Thursday", &library.thursday.rendered, false)
                                .field("Friday", &library.friday.rendered, false)
                                .field("Saturday", &library.saturday.rendered, false)
                        })
                    }).await?;
                }
                Err(ex) => {
                    ctx.say("The library gave us weird data, try again later?").await?;
                    error!("Failed to process calendar: {}", ex);
                }
            }
        }
        Err(ex) => {
            ctx.say("Failed to connect to the library API, try again later?").await?;
            error!("Failed to get calendar: {}", ex);
        }
    }

    Ok(())
}