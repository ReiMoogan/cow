mod course_reminders;

use std::sync::Arc;
use std::time::Duration;
use tracing::error;
use serenity::{
    CacheAndHttp,
    prelude::TypeMap
};
use tokio::sync::RwLock;
use tokio::time;
use crate::{CowContext, Database, Error};
use course_reminders::*;

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Set up reminders for class registration, based off seats or waitlist."),
    subcommands("add", "remove", "list"),
    discard_spare_arguments,
    aliases("remind", "reminder"),
    identifying_name = "Course Reminders"
)]
pub async fn reminders(ctx: CowContext<'_>) -> Result<(), Error> {
    list_code(ctx).await
}

pub async fn check_reminders(data: Arc<RwLock<TypeMap>>, ctx: Arc<CacheAndHttp>) {
    let mut interval_min = time::interval(Duration::from_secs(60));
    loop {
        interval_min.tick().await;
        let ctx_global = data.read().await;
        let db = ctx_global.get::<Database>().expect("Couldn't find database").clone();
        match db.trigger_reminders().await {
            Ok(triggers) => {
                for trigger in triggers {
                    if let Ok(user) = ctx.http.get_user(trigger.user_id).await {
                        if let Ok(Some(class)) = db.get_class(trigger.course_reference_number).await {
                            if let Err(ex) = user.direct_message(&ctx.http, |m| {
                                m.embed(|e| e
                                    .title("Reminder Triggered~")
                                    .description(class.course_title.unwrap_or_else(|| "<unknown class name>".to_string()))
                                    .field("Course Number", class.course_number, true)
                                    .field("Course Reference Number", class.course_reference_number, true)
                                    .field("Seats Available/Total", format!("{}/{}", class.seats_available, class.maximum_enrollment), true)
                                    .field("Waitlist Available/Total", format!("{}/{}", class.wait_available, class.wait_capacity), true)
                                )
                            }).await {
                                error!("Failed to send DM to user: {}", ex);
                            }
                        }
                    } else {
                        error!("Failed to get user");
                    }
                }
            },
            Err(ex) => {
                error!("Failed to query reminders: {}", ex);
            }
        }
    }
}