use chrono::{NaiveDate, NaiveTime};
use reqwest::{Url, Client};
use serenity::{
    client::Context,
    model::{channel::Message},
    framework::standard::{
        CommandResult,
        macros::{
            command
        }
    },
    Error
};
use crate::commands::ucm::pav_models::*;
use log::error;
use serenity::framework::standard::Args;

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

async fn fetch_pavilion_restaurants(client: &Client, company: &Company) -> Result<Vec<Location>, Box<dyn std::error::Error + Send + Sync>> {
    let response = client
        .get("https://widget.api.eagle.bigzpoon.com/nearbyrestaurants")
        .header("x-comp-id", company.id.as_str())
        .send()
        .await?
        .text()
        .await?;
    let result: PavResult<Vec<Location>> = serde_json::from_str(&*response)?;

    Ok(result.data)
}

async fn fetch_pavilion_groups(client: &Client, location: &Location) -> Result<MenuGroups, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://widget.api.eagle.bigzpoon.com/locations/menugroups?locationId={}", location.id);

    let response = client
        .get(url)
        .header("x-comp-id", location.id.as_str())
        .send()
        .await?
        .text()
        .await?;
    let result: PavResult<MenuGroups> = serde_json::from_str(&*response)?;

    Ok(result.data)
}

async fn fetch_pavilion_menu(client: &Client, location: &Location, category: &str, group: &str) -> Result<MenuItems, Box<dyn std::error::Error + Send + Sync>> {
    // I still can't believe someone thought putting JSON in a GET query was a good idea.
    let url = Url::parse_with_params("https://widget.api.eagle.bigzpoon.com/menuitems",
    &[("categoryId", category), ("isPreview", "false"), ("locationId", location.id.as_str()), ("menuGroupId", group),
        ("userPreferences", r#"{"allergies":[],"lifestyleChoices":[],"medicalGoals":[],"preferenceApplyStatus":false}"#)])?;

    let response = client
        .get(url)
        .header("x-comp-id", location.id.as_str())
        .send()
        .await?
        .text()
        .await?;
    let result: PavResult<MenuItems> = serde_json::from_str(&*response)?;

    Ok(result.data)
}

async fn print_pavilion_times(ctx: &Context, msg: &Message) -> Result<(), Error> {
    msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| e
        .title("Pavilion/Yablokoff Times")
        .field("Weekdays", format!("Breakfast: {} - {}\nLunch: {} - {}\nDinner: {} - {}\nDinner (Yablokoff): {} - {}",
            PavilionTime::breakfast_weekday_start().format("%l:%M %p"), PavilionTime::breakfast_end().format("%l:%M %p"),
            PavilionTime::lunch_start().format("%l:%M %p"), PavilionTime::lunch_end().format("%l:%M %p"),
            PavilionTime::dinner_start().format("%l:%M %p"), PavilionTime::dinner_end().format("%l:%M %p"),
            YablokoffTime::dinner_start().format("%l:%M %p"), YablokoffTime::dinner_end().format("%l:%M %p")), false)
        .field("Weekends", format!("Breakfast: {} - {}\nLunch: {} - {}\nDinner: {} - {}",
            PavilionTime::breakfast_weekend_start().format("%l:%M %p"), PavilionTime::breakfast_end().format("%l:%M %p"),
            PavilionTime::lunch_start().format("%l:%M %p"), PavilionTime::lunch_end().format("%l:%M %p"),
            PavilionTime::dinner_start().format("%l:%M %p"), PavilionTime::dinner_end().format("%l:%M %p")), false)
    )).await?;

    Ok(())
}

#[command]
#[description = "Get the current menu at the UCM Pavilion and Yablokoff."]
#[aliases("pav", "yablokoff", "yab")]
pub async fn pavilion(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let date = chrono::offset::Local::now();
    let (mut day, mut meal) = PavilionTime::next_meal(&date);

    // Basically a string builder for custom meals.
    let mut custom_meal = String::new();

    // For next week searches (most likely unused)
    let mut next_week = false;

    if args.len() == 1 {
        // Peek at first element to check if it's asking for the hours.
        let input_lower = args.parse::<String>().unwrap().to_lowercase();
        if input_lower.contains("time") || input_lower.contains("hour") {
            print_pavilion_times(ctx, msg).await?;
            return Ok(())
        }
    }

    while !args.is_empty() {
        let input = args.single::<String>().unwrap();
        if input == "next".to_string() {
            next_week = true;
        }
        // If an input contains a day, set the day.
        else if let Ok(input_day) = Day::try_from(&input) {
            day = input_day;
        }
        // Otherwise, it's a custom meal option.
        else {
            if !custom_meal.is_empty() {
                custom_meal += " ";
            }
            custom_meal += &*input;
        }
    }

    let title: String;
    if !custom_meal.is_empty() {
        meal = Meal::from(&*custom_meal);
        if !matches!(meal, Meal::Other(_)) {
            title = format!("{} at the Pavilion for {}", meal, day);
        } else {
            // Do not let the bot print non-validated input.
            title = format!("Custom Category at the Pavilion for {}", day);
        }
    } else {
        title = format!("{} at the Pavilion for {}", meal, day);
    }

    let mut message = msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
        e
            .title(&title)
            .description("Loading data, please wait warmly...")
    })).await?;

    let description = process_bigzpoon_new(day, meal, next_week).await;

    message.edit(&ctx.http, |m| m.embed(|e| {
        e
            .title(&title)
            .description(description)
    })).await?;

    Ok(())
}

async fn process_bigzpoon(day: Day, meal: Meal) -> String {
    let description: String;
    let client = Client::new();

    // I love nesting. /s
    match fetch_pavilion_company_info(&client).await {
        Ok(company_info) => {
            match fetch_pavilion_groups(&client, &company_info.location_info).await {
                Ok(groups) => {
                    if let Some(group) = groups.get_group(day) {
                        if let Some(category) = groups.get_category(meal) {
                            match fetch_pavilion_menu(&client, &company_info.location_info, &category, &group).await {
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
                                .reduce(|a, b| { format!("{}, {}", a, b) })
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

    description
}

async fn process_bigzpoon_new(day: Day, meal: Meal, next_week: bool) -> String {
    let description: String;
    let client = Client::new();

    // Super nesting!
    match fetch_pavilion_company_info(&client).await {
        Ok(company_info) => {
            match fetch_pavilion_restaurants(&client, &company_info).await {
                Ok(restaurants) => {
                    // We need to calculate the week.
                    let today = chrono::offset::Local::now().naive_local();
                    let date = NaiveDate::from_ymd(2022, 8, 1).and_time(NaiveTime::from_hms(0, 0, 0));
                    let days = (today - date).num_days();
                    // Division then ceiling, then adding one if they're asking for next week.
                    let week_no = (days + 7 - 1) / 7 + (if next_week { 1 } else { 0 });

                    let location_match = restaurants
                        .iter()
                        .filter(|o| o.location_special_group_ids.is_some())
                        .filter(|o| o.location_special_group_ids.as_deref().unwrap().first().is_some())
                        // I have no idea if they're resetting the numbers for spring, so I won't future-proof this.
                        .find(|o| o.location_special_group_ids.as_deref().unwrap().first().unwrap().name == format!("PAV-FALL-W{}", week_no));

                    if let Some(location) = location_match {
                        match fetch_pavilion_groups(&client, location).await {
                            Ok(groups) => {
                                if let Some(group) = groups.get_group(day) {
                                    if let Some(category) = groups.get_category(meal) {
                                        match fetch_pavilion_menu(&client, location, &category, &group).await {
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
                                            .reduce(|a, b| { format!("{}, {}", a, b) })
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
                    else {
                        description = "Could not find an appropriate restaurant link for the week! Current algorithm might be outdated.".to_string();
                        error!("Failed to find restaurant for week {}: {}", week_no,
                            restaurants
                                .iter()
                                .filter(|o| o.location_special_group_ids.is_some())
                                .filter(|o| o.location_special_group_ids.as_deref().unwrap().first().is_some())
                                .map(|o| o.location_special_group_ids.as_deref().unwrap().first().unwrap().name.to_string())
                                .reduce(|a, b| format!("{}, {}", a, b))
                                .unwrap_or_else(|| "<none>".to_string())
                        );
                    }
                }
                Err(ex) => {
                    description = "Failed to load restaurant info!".to_string();
                    error!("Failed to read restaurant list from BigZpoon: {}", ex);
                }
            }
        }
        Err(ex) => {
            description = "Failed to get company info!".to_string();
            error!("Failed to get company info: {}", ex);
        }
    }

    description
}