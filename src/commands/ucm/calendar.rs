use chrono::{Datelike, Local};
use poise::CreateReply;
use tracing::error;
use crate::{CowContext, Error};
use scraper::{Html, Selector};
use serenity::all::CreateEmbed;

pub struct Semester {
    pub name: String,
    pub dates: Vec<(String, String)>
}

pub struct AcademicCalendar {
    pub name: String,
    pub semesters: Vec<Semester>
}

fn process_calendar(data: &str) -> Option<AcademicCalendar> {
    let page = Html::parse_document(data);

    let select_page_name = Selector::parse("h1").unwrap();
    let select_table_name = Selector::parse("h2").unwrap();
    let select_table = Selector::parse("table").unwrap();
    let select_row = Selector::parse("tr").unwrap();
    let select_column = Selector::parse("td").unwrap();

    let page_name = page.select(&select_page_name).next().map(|o| o.text().next().map(|o| o.to_string()));

    // Ensure this is a calendar page, not some other weird thing.
    if let Some(Some(ref name)) = page_name {
        if !name.to_lowercase().contains("calendar") {
            return None;
        }
    } else {
        return None;
    }

    let title_names = page
        .select(&select_table_name).flat_map(|o| o.text()
            .filter(|p| {
                let lowercase = p.to_lowercase();
                lowercase.contains("semester") || lowercase.contains("session")
            }))
        .map(|o| o.to_string());

    let tables = page
        .select(&select_table)
        .map(|table| table
            .select(&select_row)
            .map(|row| {
                let items = row
                    .select(&select_column)
                    .take(2)
                    .map(|col| col.text().next().map(|o| o.to_string()).unwrap_or_else(|| "<unknown>".to_string()))
                    .collect::<Vec<_>>();

                (items.first().map(|o| o.to_string()).unwrap_or_else(|| "<unknown>".to_string()),
                 items.get(1).map(|o| o.to_string()).unwrap_or_else(|| "<unknown>".to_string()))
            })
            .collect::<Vec<_>>()
        );

    let semesters = title_names.zip(tables)
        .map(|o| {
            let (name, dates) = o;
            Semester { name, dates }
        })
        .collect::<Vec<_>>();

    Some(AcademicCalendar { name: page_name.unwrap().unwrap(), semesters })
}

async fn print_schedule(ctx: &CowContext<'_>, schedule: &AcademicCalendar) -> Result<(), Error> {
    let mut embed = CreateEmbed::new().title(&schedule.name);

    for semester in &schedule.semesters {
        let output = semester.dates.iter()
            .map(|o| {
                let (l, r) = o;
                format!("{l} - {r}")
            })
            .reduce(|a, b| format!("{a}\n{b}"))
            .unwrap_or_else(|| "Nothing was written...".to_string());

        embed = embed.field(&semester.name, output, false);
    }

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the academic calendar for the year."),
    aliases("cal", "academiccalendar")
)]
pub async fn calendar(
    ctx: CowContext<'_>,
    #[description = "A year on or past 2005."] #[min = 2005] year: Option<i32>)
-> Result<(), Error> {
    let now = Local::now();
    let mut calendar_year = year.filter(|o| *o >= 2005).unwrap_or_else(|| now.year());

    if now.month() <= 7 { // Spring or summer semester are still on the previous year.
        calendar_year -= 1;
    }

    ctx.defer().await?;

    let url = format!("https://registrar.ucmerced.edu/schedules/academic-calendar/academic-calendar-{}-{}", calendar_year, calendar_year + 1);
    match reqwest::get(url).await {
        Ok(response) => {
            match response.text().await {
                Ok(data) => {
                    let schedules = process_calendar(&data);
                    if let Some(calendar) = schedules {
                        print_schedule(&ctx, &calendar).await?;
                    } else {
                        ctx.say("Either you inputted an invalid year, or the website did not give us reasonable data.").await?;
                    }
                }
                Err(ex) => {
                    ctx.say("UC Merced gave us weird data, try again later?").await?;
                    error!("Failed to process calendar: {}", ex);
                }
            }
        }
        Err(ex) => {
            ctx.say("Failed to connect to the UC Merced website, try again later?").await?;
            error!("Failed to get food truck schedule: {}", ex);
        }
    }

    Ok(())
}