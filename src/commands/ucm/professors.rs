use chrono::{Datelike, DateTime, Local, TimeZone, Utc};
use log::error;
use serenity::{
    client::Context,
    model::{
        channel::Message
    },
    framework::standard::{
        CommandResult,
        macros::{
            command
        }, Args
    }
};
use crate::commands::ucm::courses_db_models::*;
use crate::{Database, db};

async fn professor_embed(ctx: &Context, msg: &Message, professor: &Professor) -> CommandResult {
    let db = db!(ctx);

    let current_date = Local::now().date();
    let year = current_date.year();
    let semester = if current_date.month() >= 3 && current_date.month() <= 10 { 30 } else { 10 };
    let term = year * 100 + semester;

    let classes = db.get_classes_for_professor(professor.id, term).await;
    let stats = db.get_stats().await;
    msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
        e.title(&professor.full_name);
        e.description("Note: this uses Rate My Professor, which may be off at times~");
        e.field("Rating Score", professor.rating, true);
        e.field("Number of Ratings", professor.num_ratings, true);
        e.field("Email", professor.email.clone().unwrap(), true);


        if let Ok(classes) = classes {
            e.field(format!("Classes for {} (totalling {})", crate::commands::ucm::format_term(term), classes.len()),
                    classes.iter()
                        .map(|o| format!("- {} (`{}`): {}", &o.course_number, o.course_reference_number, o.course_title.clone().unwrap_or_else(|| "<unknown class name>".to_string())))
                        .reduce(|a, b| if a.len() < 1000 { format!("{}\n{}", a, b) } else {a})
                        .unwrap_or_else(|| "This person is not teaching any classes for this term.".to_string()),
                    false);
        }

        if let Ok(stats) = stats {
            if let Some(class_update) = stats.get("professor") {
                let local_time: DateTime<Local> = Local.from_local_datetime(class_update).unwrap();
                let utc_time: DateTime<Utc> = DateTime::from(local_time);
                e.footer(|f| f.text("Last updated at"));
                e.timestamp(utc_time);
            }
        }

        e
    })).await?;

    Ok(())
}

#[command]
#[description = "Search for a professor."]
#[aliases("professor")]
#[usage = "<Professor's Name>"]
pub async fn professors(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let search_query = args.message();

    let db = db!(ctx);
    match db.search_professor(search_query).await {
        Ok(professors) => {
            print_matches(ctx, msg, &professors).await?;
        }
        Err(ex) => {
            error!("Failed to search by name: {}", ex);
            msg.channel_id.say(&ctx.http, "Failed to search for professors... try again later?").await?;
        }
    }

    Ok(())
}

async fn print_matches(ctx: &Context, msg: &Message, professors: &[Professor]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if professors.is_empty() {
        msg.channel_id.say(&ctx.http, "No matches were found. Check your query for typos or generalize it. Or, we may not have the person logged.").await?;
    } else if professors.len() == 1 {
        professor_embed(ctx, msg, professors.get(0).unwrap()).await?;
    } else {
        msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
            e.title("Professor Search").description("Multiple results were found for your query. Try refining your input.");
            e.field(format!("Professors Matched (totalling {})", professors.len()),
                    professors
                        .iter()
                        .take(10)
                        .map(|o| format!("`{}` - {}", o.full_name, o.department.clone().unwrap_or_else(|| "<unknown department>".to_string())))
                        .reduce(|a, b| format!("{}\n{}", a, b))
                        .unwrap(),
                    false);
            e
        })).await?;
    }

    Ok(())
}