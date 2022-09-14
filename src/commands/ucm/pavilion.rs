use chrono::{NaiveDate, NaiveTime};
use reqwest::{Url, Client};
use crate::{CowContext, Error};
use crate::commands::ucm::pav_models::*;
use tracing::error;
use std::error;

// Probably can be hard-coded to be 61bd7ecd8c760e0011ac0fac.
async fn fetch_pavilion_company_info(client: &Client) -> Result<Company, Box<dyn error::Error + Send + Sync>> {
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

async fn fetch_pavilion_restaurants(client: &Client, company: &Company) -> Result<Vec<Location>, Box<dyn error::Error + Send + Sync>> {
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

async fn fetch_pavilion_groups(client: &Client, company: &Company, location: &Location) -> Result<MenuGroups, Box<dyn error::Error + Send + Sync>> {
    let url = format!("https://widget.api.eagle.bigzpoon.com/locations/menugroups?locationId={}", location.id);

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

async fn fetch_pavilion_menu(client: &Client, company: &Company, location: &Location, category: &str, group: &str) -> Result<MenuItems, Box<dyn error::Error + Send + Sync>> {
    // I still can't believe someone thought putting JSON in a GET query was a good idea.
    let url = Url::parse_with_params("https://widget.api.eagle.bigzpoon.com/menuitems",
    &[("categoryId", category), ("isPreview", "false"), ("locationId", location.id.as_str()), ("menuGroupId", group),
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

async fn fetch_pavilion_raw_materials(client: &Client, company: &Company, location: &Location, item: &Item) -> Result<Vec<RawMaterial>, Box<dyn error::Error + Send + Sync>> {
    const BODY: &str = r#"{ "menuId": "M_ID", "fdaRounding": true, "allergyIds": [], "lifestyleChoiceIds": [], "nutritionGoals": [], "preferenceApplyStatus": false, "skipCommonIngredients": [], "locationId": "L_ID" }"#;
    let response = client
        .post("https://widget.api.eagle.bigzpoon.com/raw-materials")
        .header("x-comp-id", company.id.as_str())
        .header("Content-Type", "application/json")
        .body(BODY.replace("M_ID", item.id.as_ref()).replace("L_ID", location.id.as_ref()))
        .send()
        .await?
        .text()
        .await?;
    let result: PavResult<Vec<RawMaterial>> = serde_json::from_str(&*response)?;

    Ok(result.data)
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the current hours for dining services."),
    aliases("snack", "snacks", "snackshop", "cafe", "lantern", "lanterncafe")
)]
pub async fn dining(ctx: CowContext<'_>) -> Result<(), Error> {
    print_pavilion_times(ctx).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the current menu at the UCM Pavilion and Yablokoff."),
    aliases("pav", "yablokoff", "yab")
)]
pub async fn pavilion(
    ctx: CowContext<'_>,
    #[description = "\"hours\" for hours, day of the week, and/or \"breakfast\"/\"lunch\"/\"dinner\""] #[rest] options: Option<String>)
-> Result<(), Error> {
    let date = chrono::offset::Local::now();
    let (mut day, mut meal) = PavilionTime::next_meal(&date);

    // Basically a string builder for custom meals.
    let mut custom_meal = String::new();

    // For next week searches (most likely unused)
    let mut next_week = false;

    let input = options.unwrap_or_default();
    let args = input.split(' ').collect::<Vec<_>>();
    if args.len() == 1 {
        // Peek at first element to check if it's asking for the hours.
        let input_lower = args[0].to_lowercase();
        if input_lower.contains("time") || input_lower.contains("hour") {
            print_pavilion_times(ctx).await?;
            return Ok(())
        } else if input_lower.contains("announce") {
            print_announcements(ctx).await?;
            return Ok(())
        }
    }

    for arg in args {
        if arg == "next" {
            next_week = true;
        }
        // If an input contains a day, set the day.
        else if let Ok(input_day) = Day::try_from(arg) {
            day = input_day;
        }
        // Otherwise, it's a custom meal option.
        else {
            if !custom_meal.is_empty() {
                custom_meal += " ";
            }
            custom_meal += arg;
        }
    }

    let title: String;
    if !custom_meal.is_empty() {
        meal = Meal::from(&*custom_meal);
        if !matches!(meal, Meal::Other(_)) {
            title = format!("{} at the Pavilion/Yablokoff for {}", meal, day);
        } else {
            // Do not let the bot print non-validated input.
            title = format!("Custom Category at the Pavilion/Yablokoff for {}", day);
        }
    } else {
        title = format!("{} at the Pavilion/Yablokoff for {}", meal, day);
    }

    let message = ctx.send(|m| m.embed(|e| {
        e
            .title(&title)
            .description("Loading data, please wait warmly...")
    })).await?;

    let menus = process_bigzpoon(day, meal, next_week).await;

    message.edit(ctx, |m| {
        m.embeds.clear();
        m.embed(|e| {
            e.title(&title);
            e.description("Note (9/10/2022): Item names may be incorrect, please check descriptions for the correct menu items. Regardless, items shown may not be available at the time of your visit. (I have no idea what the Pavilion is doing with their menu.)");
            for group in menus.iter().take(4) { // Max four filled fields...
                let (group_name, menu) = group;
                let menu_truncated = menu.chars().take(1024).collect::<String>();
                e.field(group_name, menu_truncated, false);
            }

            e
        })
    }).await?;

    Ok(())
}

async fn print_pavilion_times(ctx: CowContext<'_>) -> Result<(), Error> {
    ctx.send(|m| m.embed(|e| e
        .title("Dining Services Hours")
        .field("Pavilion on Weekdays", format!("Breakfast: {} - {}\nLunch: {} - {}\nDinner: {} - {}",
            PavilionTime::breakfast_weekday_start().format("%l:%M %p"), PavilionTime::breakfast_end().format("%l:%M %p"),
            PavilionTime::lunch_start().format("%l:%M %p"), PavilionTime::lunch_end().format("%l:%M %p"),
            PavilionTime::dinner_start().format("%l:%M %p"), PavilionTime::dinner_end().format("%l:%M %p")), false)
        .field("Pavilion on Weekends", format!("Breakfast: {} - {}\nLunch: {} - {}\nDinner: {} - {}",
            PavilionTime::breakfast_weekend_start().format("%l:%M %p"), PavilionTime::breakfast_end().format("%l:%M %p"),
            PavilionTime::lunch_start().format("%l:%M %p"), PavilionTime::lunch_end().format("%l:%M %p"),
            PavilionTime::dinner_start().format("%l:%M %p"), PavilionTime::dinner_end().format("%l:%M %p")), false)
        .field("Yablokoff on Weekdays", format!("Dinner: {} - {}",
            YablokoffTime::dinner_start().format("%l:%M %p"), YablokoffTime::dinner_end().format("%l:%M %p")), false)
        .field("Lantern Cafe", "Monday to Friday: 7:00 AM - 5:00 PM", false)
        .field("Bobcat Snack Shop", "Monday to Friday: 8:00 AM - 6:00 PM", false)
    )).await?;

    Ok(())
}

async fn print_announcements(ctx: CowContext<'_>) -> Result<(), Box<dyn error::Error + Send + Sync>> {
    const TITLE: &str = "Pavilion/Yablokoff Announcements";
    let message = ctx.send(|m| m.embed(|e| {
        e
            .title(TITLE)
            .description("Loading data, please wait warmly...")
    })).await?;

    let pav_announcement = process_announcement("ANNOUNCEMENT-PAV").await;
    let wydc_announcement = process_announcement("ANNOUNCEMENT-WYDC").await;

    message.edit(ctx, |m| {
        m.embeds.clear();
        m.embed(|e| {
            e
                .title(TITLE)
                .field("Pavilion Announcements", pav_announcement, false)
                .field("Yablokoff Announcements", wydc_announcement, false)
        })
    }).await?;

    Ok(())
}

async fn process_announcement(name: &str) -> String {
    let description: String;
    let client = Client::new();

    match fetch_pavilion_company_info(&client).await {
        Ok(company_info) => {
            match fetch_pavilion_restaurants(&client, &company_info).await {
                Ok(restaurants) => {
                    let announcements_location = restaurants
                        .iter()
                        .filter(|o| o.location_special_group_ids.is_some())
                        .filter(|o| o.location_special_group_ids.as_deref().unwrap().first().is_some())
                        .find(|o| o.location_special_group_ids.as_deref().unwrap().first().unwrap().name == name);

                    if let Some(location) = announcements_location {
                        match fetch_pavilion_groups(&client, &company_info, location).await {
                            Ok(groups) => {
                                if let Some(group) = groups.menu_groups.get(0) {
                                    if let Some(category) = groups.menu_categories.iter().find(|o| o.name.to_lowercase().contains("announce")) {
                                        match fetch_pavilion_menu(&client, &company_info, location, &category.id, &group.id).await {
                                            Ok(menu) => {
                                                let item = menu.menu_items.first();
                                                if let Some(announcement) = item {
                                                    match fetch_pavilion_raw_materials(&client, &company_info, location, announcement).await {
                                                        Ok(materials) => {
                                                            description = materials
                                                                .iter()
                                                                .map(|o| o.name.clone())
                                                                .reduce(|a, b| format!("{}\n{}", a, b))
                                                                .unwrap_or_else(|| "The announcement is empty?".to_string());
                                                        }
                                                        Err(ex) => {
                                                            description = "Failed to get announcement data.".to_string();
                                                            error!("Failed to get announcement data: {}", ex);
                                                        }
                                                    }
                                                } else {
                                                    description = "No announcement could be found.".to_string();
                                                }
                                            }
                                            Err(ex) => {
                                                description = "Failed to get the menu from the website.".to_string();
                                                error!("Failed to get the menu: {}", ex);
                                            }
                                        }
                                    } else {
                                        description = "Failed to find a category for announcements.".to_string();
                                        error!("Failed to find a group for announcements");
                                    }
                                } else {
                                    description = "Failed to find a group for info.".to_string();
                                    error!("Failed to find a group for info");
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
                        error!("Failed to find restaurant for data: {}",
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

async fn process_bigzpoon(day: Day, meal: Meal, next_week: bool) -> Vec<(String, String)> {
    let mut output: Vec<(String, String)> = Vec::new();
    let client = Client::new();

    // Super nesting!
    match fetch_pavilion_company_info(&client).await {
        Ok(company_info) => {
            match fetch_pavilion_restaurants(&client, &company_info).await {
                Ok(restaurants) => {
                    // We need to calculate the week.
                    let today = chrono::offset::Local::now().naive_local();
                    // This is the start of the new schedule system (7/31/2022 reset at W1 as well)
                    let date = NaiveDate::from_ymd(2022, 8, 21).and_time(NaiveTime::from_hms(0, 0, 0));
                    let days = (today - date).num_days();
                    // Division then ceiling, add one for 1-based indexing, then adding one more if they're asking for next week.
                    let week_no = days / 7 + 1 + (if next_week { 1 } else { 0 });

                    let location_match = restaurants
                        .iter()
                        .filter(|o| o.location_special_group_ids.is_some())
                        .filter(|o| o.location_special_group_ids.as_deref().unwrap().first().is_some())
                        .collect::<Vec<_>>();
                    // I have no idea if they're resetting the numbers for spring, so I won't future-proof this.
                    let pav_location = location_match.iter().find(|o| o.location_special_group_ids.as_deref().unwrap().first().unwrap().name == format!("PAV-FALL-W{}", week_no));
                    let ywdc_location = location_match.iter().find(|o| o.location_special_group_ids.as_deref().unwrap().first().unwrap().name == "YWDC-FALL");

                    get_menu_items(&day, &meal, &mut output, &client, &company_info, &restaurants, pav_location).await;

                    // YWDC does not have next-week options. Also, it must be a weekday.
                    if !next_week && YablokoffTime::is_dinner(&day) {
                        // Prepend the name, because "Dinner" exists verbatim in both categories (can be confused by a user)
                        let mut ywdc_output: Vec<(String, String)> = Vec::new();
                        get_menu_items(&day, &meal, &mut ywdc_output, &client, &company_info, &restaurants, ywdc_location).await;
                        for ywdc_menu in ywdc_output {
                            let (category, menu) = ywdc_menu;
                            output.push((format!("Yablokoff {}", category), menu));
                        }
                    }
                }
                Err(ex) => {
                    output.push(("Error~".to_string(), "Failed to load restaurant info!".to_string()));
                    error!("Failed to read restaurant list from BigZpoon: {}", ex);
                }
            }
        }
        Err(ex) => {
            output.push(("Error~".to_string(), "Failed to get company info!".to_string()));
            error!("Failed to get company info: {}", ex);
        }
    }

    output
}

async fn get_menu_items(day: &Day, meal: &Meal, output: &mut Vec<(String, String)>, client: &Client, company_info: &Company, restaurants: &[Location], pav_location: Option<&&Location>) {
    if let Some(location) = pav_location {
        match fetch_pavilion_groups(client, company_info, location).await {
            Ok(groups) => {
                if let Some(group) = groups.get_group(day) {
                    for category in groups.get_categories(meal)
                    {
                        let description = match fetch_pavilion_menu(client, company_info, location, category.id.as_ref(), &group).await {
                            Ok(menu) => {
                                menu.menu_items.into_iter()
                                    .map(|o| format!("**{}** - {}", o.name, o.description))
                                    .reduce(|a, b| format!("{}\n{}", a, b))
                                    .unwrap_or_else(|| "There is nothing on the menu?".to_string())
                            }
                            Err(ex) => {
                                error!("Failed to get the menu: {}", ex);
                                "Failed to get the menu from the website!".to_string()
                            }
                        };

                        output.push((category.name.clone(), description));
                    }
                } else {
                    output.push(("Error~".to_string(), "Could not find a group for the given day!".to_string()));
                }
            }
            Err(ex) => {
                output.push(("Error~".to_string(), "Failed to get groups and categories from the website!".to_string()));
                error!("Failed to get groups and categories: {}", ex);
            }
        }
    } else {
        output.push(("Error~".to_string(), "Could not find an appropriate restaurant link for the week! Current algorithm might be outdated.".to_string()));
        error!("Failed to find restaurant: {}",
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
