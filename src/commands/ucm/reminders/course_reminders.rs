use tracing::error;
use crate::{CowContext, cowdb, Error};

use crate::{db, Database};
use crate::commands::ucm::courses_db_models::Reminder;

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "List the reminders set."),
    discard_spare_arguments
)]
pub async fn list(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);

    match db.get_user_reminders(ctx.author().id).await {
        Ok(reminders) => {
            ctx.send(|m| {
                m.embeds.clear();
                m.embed(|e| {
                    e.title("Your Course Reminders");

                    if reminders.is_empty() {
                        e.description("You do not have any reminders set. Add some using `reminders add`.");
                    } else {
                        for reminder in reminders {
                            e.field(format!("CRN {}", reminder.course_reference_number),
                                    format!("Minimum Trigger: `{}`\nFor Waitlist: `{}`\nTriggered: `{}`", reminder.min_trigger, reminder.for_waitlist, reminder.triggered),
                                    false);
                        }
                    }

                    e
                })
            }).await?;
        }
        Err(ex) => {
            error!("Failed to get reminders for user: {}", ex);
            ctx.say("Failed to get your reminders... try again later?").await?;
        }
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Control reminders for class seats.")
)]
pub async fn add(
    ctx: CowContext<'_>,
    #[description = "The CRN of the class to get reminders for"] #[min = 10000] course_reference_number: i32,
    #[description = "The minimum amount of seats to trigger at, 1 minimum"]  #[min = 1] min_seats: Option<i32>,
    #[description = "If the reminder is for a waitlist spot"] for_waitlist: Option<bool>)
-> Result<(), Error> {

    let min_trigger = if let Some(seats) = min_seats {
        if seats < 1 {
            ctx.say("Your minimum trigger must be greater than or equal to 1 seat.").await?;
            return Ok(());
        }

        seats
    } else {
        1
    };
    
    let reminder = Reminder {
        user_id: ctx.author().id.0,
        course_reference_number,
        min_trigger,
        for_waitlist: for_waitlist.unwrap_or(false),
        triggered: false
    };

    let db = cowdb!(ctx);

    if let Ok(Some(class)) = db.get_class(course_reference_number).await {
        if let Err(ex) = db.add_reminder(&reminder).await {
            error!("Failed to add reminder: {}", ex);
            ctx.say("Error adding your reminder. Maybe you have a duplicate?").await?;
        } else {
            ctx.say(format!("Successfully added your reminder for {}: {}!",
                                                  class.course_number,
                                                  class.course_title.unwrap_or_else(|| "<unknown class name>".to_string())
            )).await?;
        }
    } else {
        ctx.say("Could not find this CRN... did you type it right?").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Remove reminders for class seats.")
)]
pub async fn remove(
    ctx: CowContext<'_>,
    #[description = "The CRN of the class to disable reminders for"] #[min = 10000] course_reference_number: i32)
-> Result<(), Error> {

    let db = cowdb!(ctx);
    match db.remove_reminder(ctx.author().id, course_reference_number).await {
        Ok(success) => {
            if success {
                ctx.say("Successfully removed your reminder.").await?;
            } else {
                ctx.say("You did not have a reminder with this CRN.").await?;
            }
        }
        Err(ex) => {
            error!("Failed to remove reminder: {}", ex);
            ctx.say("Failed to remove your reminder... try again later?").await?;
        }
    }

    Ok(())
}