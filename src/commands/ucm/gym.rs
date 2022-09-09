use log::error;
use crate::{CowContext, Error};
use scraper::{Html, Selector};

fn process_hours(data: &str) -> Vec<(String, String)> {
    let mut output: Vec<(String, String)> = Vec::new();

    let page = Html::parse_document(data);
    let text = Selector::parse(".content h3, .content p").unwrap();

    let mut temporary_name: Option<String> = None;
    let mut temporary_values: Vec<String> = Vec::new();
    for text in page.select(&text) {
        let text_data = text
            .text()
            .map(|o| o.trim())
            .filter(|o| !o.is_empty())
            .map(|o| o.to_string())
            .reduce(|a, b| format!("{}\n{}", a, b))
            .unwrap_or_default();

        if text.value().name() == "h3" {
            // New header, push values.
            extractor(&mut output, &temporary_name, &mut temporary_values);
            temporary_name = Some(text_data);
        } else if temporary_name != None {
            temporary_values.push(text_data);
        } else {
            // Can't read if there's no header.
            break;
        }
    }
    // Clear buffer if necessary.
    extractor(&mut output, &temporary_name, &mut temporary_values);

    output
}

fn extractor(output: &mut Vec<(String, String)>, temporary_name: &Option<String>, temporary_values: &mut Vec<String>) {
    if let Some(temp_name) = temporary_name {
        if !temporary_values.is_empty() {
            output.push((temp_name.clone(), temporary_values
                .iter()
                .map(|o| o.to_string())
                .reduce(|a, b| format!("{}\n{}", a, b))
                .unwrap()));

            temporary_values.clear();
        } else {
            output.push((temp_name.clone(), String::new()));
        }
    }
    // The "Some" condition should always be true in this case.
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the hours for recreation and atheletic facilities."),
    guild_only
)]
pub async fn gym(ctx: CowContext<'_>) -> Result<(), Error> {
    const TITLE: &str = "Recreation and Athletic Facility Hours";

    let sent_msg = ctx.send(|m| m.embed(|e| {
        e
            .title(TITLE)
            .description("Now loading, please wait warmly...")
    })).await?;

    const URL: &str = "https://recreation.ucmerced.edu/Facility-Hours";
    const EMPTY: &str = "\u{200b}";

    match reqwest::get(URL).await {
        Ok(response) => {
            match response.text().await {
                Ok(data) => {
                    let hours = process_hours(&*data);

                    if !hours.is_empty() {
                        sent_msg.edit(ctx, |m| {
                            m.embeds.clear();
                            m.embed(|e| {
                                e.title(TITLE).fields(hours.iter().map(|o| {
                                    let (name, value) = o;

                                    if value.is_empty() {
                                        (name.as_str(), EMPTY, false)
                                    } else {
                                        (name.as_str(), value.as_str(), false)
                                    }
                                }))
                            })
                        }).await?;
                    } else {
                        sent_msg.edit(ctx, |m| {
                            m.embeds.clear();
                            m.embed(|e| {
                                e.title(TITLE).description("Could not get any hours... Did the website change layout?")
                            })
                        }).await?;
                        error!("Unable to read athletics website");
                    }
                }
                Err(ex) => {
                    sent_msg.edit(ctx, |m| {
                        m.embeds.clear();
                        m.embed(|e| {
                            e.title(TITLE).description("UC Merced gave us weird data, try again later?")
                        })
                    }).await?;
                    error!("Failed to process hours: {}", ex);
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
            error!("Failed to get athletics hours: {}", ex);
        }
    }

    Ok(())
}