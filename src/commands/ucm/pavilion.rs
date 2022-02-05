use reqwest::{Url, Client};
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
use crate::commands::ucm::pav_models::*;
use log::error;

// Probably can be hard-coded to be 61bd7ecd8c760e0011ac0fac.
async fn fetch_pavilion_company_info(client: &Client) -> Result<Company, Box<dyn std::error::Error + Send + Sync>> {
    let response = client
        .get("https://widget.api.eagle.bigzpoon.com/company")
        .header("x-comp-id", "uc-merced-the-pavilion")
        .send()
        .await?
        .text()
        .await?;
    let result: PavResult<Company> = serde_json::from_str(&*response)?;

    Ok(result.data)
}

async fn fetch_pavilion_groups(client: &Client, company: &Company) -> Result<MenuGroups, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://widget.api.eagle.bigzpoon.com/locations/menugroups?locationId={}", company.location_info.id);

    let response = client
        .get(url)
        .header("x-comp-id", company.id.as_str())
        .send()
        .await?
        .text()
        .await?;
    let result: PavResult<MenuGroups> = serde_json::from_str(&*response)?;

    Ok(result.data)
}

async fn fetch_pavilion_menu(client: &Client, company: &Company, category: &str, group: &str) -> Result<MenuItems, Box<dyn std::error::Error + Send + Sync>> {
    // I still can't believe someone thought putting JSON in a GET query was a good idea.
    let url = Url::parse_with_params("https://widget.api.eagle.bigzpoon.com/menuitems",
    &[("categoryId", category), ("isPreview", "false"), ("locationId", company.location_info.id.as_str()), ("menuGroupId", group),
        ("userPreferences", r#"{"allergies":[],"lifestyleChoices":[],"medicalGoals":[],"preferenceApplyStatus":false}"#)])?;

    let response = client
        .get(url)
        .header("x-comp-id", company.id.as_str())
        .send()
        .await?
        .text()
        .await?;
    let result: PavResult<MenuItems> = serde_json::from_str(&*response)?;

    Ok(result.data)
}

#[command]
#[description = "Get the current menu at the UCM Pavilion."]
#[aliases("pav")]
pub async fn pavilion(ctx: &Context, msg: &Message) -> CommandResult {
    let date = chrono::offset::Local::now();
    let (day, meal) = PavilionTime::schedule(&date);
    let title = format!("{} at the Pavilion for {}", meal, day);
    let mut message = msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
        e
            .title(&title)
            .description("Loading data, please wait warmly...")
    })).await?;

    let description: String;
    let client = reqwest::Client::new();

    // I love nesting. /s
    match fetch_pavilion_company_info(&client).await {
        Ok(company_info) => {
            match fetch_pavilion_groups(&client, &company_info).await {
                Ok(groups) => {
                    if let Some(group) = groups.get_group(day) {
                        if let Some(category) = groups.get_category(meal) {
                            match fetch_pavilion_menu(&client, &company_info, &category, &group).await {
                                Ok(menu) => {
                                    description = menu.menu_items.into_iter()
                                        .map(|o| format!("**{}** - {}", o.name, o.description))
                                        .reduce(|a, b| format!("{}\n{}", a, b))
                                        .unwrap_or_else(|| "There is nothing on the menu?".to_string())
                                }
                                Err(ex) => {
                                    error!("Failed to get the menu: {}", ex);
                                    description = "Failed to get the menu from the website!".to_string();
                                }
                            }
                        } else {
                            let options = groups.menu_categories.into_iter()
                                .map(|o| format!("\"{}\"", o.name))
                                .reduce(|a, b| {format!("{}, {}", a, b)})
                                .unwrap_or_else(|| "None (?)".to_string());

                            description = format!("Could not find the given meal! Categories available: {}", options);
                        }
                    } else {
                        description = "Could not find a group for the given day!".to_string();
                    }
                }
                Err(ex) => {
                    description = "Failed to get groups and categories from the website!".to_string();
                    error!("Failed to get groups and categories: {}", ex);
                }
            }
        }
        Err(ex) => {
            description = "Failed to get company info!".to_string();
            error!("Failed to get company info: {}", ex);
        }
    }

    message.edit(&ctx.http, |m| m.embed(|e| {
        e
            .title(&title)
            .description(description)
    })).await?;

    Ok(())
}