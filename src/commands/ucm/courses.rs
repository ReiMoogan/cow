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

fn fix_time(time: &str) -> String {
    let hour_str = &time[..2];
    let minute_str = &time[2..];
    let hour = hour_str.parse::<u8>().unwrap();

    if hour == 0 {
        return format!("12:{} AM", minute_str);
    }
    if hour == 12 {
        return format!("12:{} PM", minute_str);
    }
    if hour < 12 {
        return format!("{}:{} AM", hour, minute_str);
    }
    format!("{}:{} PM", hour - 12, minute_str)
}

pub fn format_term(term: i32) -> String {
    let semester = match term % 100 {
        30 => "Fall",
        20 => "Summer",
        10 => "Summer",
        _ => "Unknown"
    };

    format!("{} {}", semester, term / 100)
}

pub fn semester_from_text(input: &str) -> Option<i32> {
    match input.to_lowercase().as_str() {
        "fall" => Some(30),
        "summer" => Some(20),
        "spring" => Some(10),
        _ => None
    }
}

async fn course_embed(ctx: &Context, msg: &Message, class: &Class) -> CommandResult {
    let db = db!(ctx);
    let professors = db.get_professors_for_class(class.id).await;
    let meetings = db.get_meetings_for_class(class.id).await;
    let stats = db.get_stats().await;

    msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
        e.title(format!("{}: {}", &class.course_number, class.course_title.clone().unwrap_or_else(|| "<unknown class name>".to_string())));
        e.description("Enrollment and Waitlist are in terms of seats available/seats taken/max seats.");
        e.field("CRN", class.course_reference_number, true);
        e.field("Credit Hours", class.credit_hours, true);
        e.field("Term", format_term(class.term), true);
        e.field("Enrollment", format!("{}/{}/{}", class.seats_available, class.enrollment, class.maximum_enrollment), true);
        e.field("Waitlist", format!("{}/{}/{}", class.wait_available, class.wait_capacity - class.wait_available, class.wait_capacity), true);

        if let Ok(professors) = professors {
            e.field("Professor(s)",
                    professors.iter()
                        .map(|o| format!("- {}", o.full_name.clone()))
                        .reduce(|a, b| format!("{}\n{}", a, b))
                        .unwrap_or_else(|| "No professors are assigned to this course.".to_string()),
                    false);
        }

        if let Ok(meetings) = meetings {
            e.field("Meeting(s)",
                    meetings.iter()
                        .map(|o| {
                            let output = format!("- {}: {} {}",
                                                 o.meeting_type, o.building_description.clone().unwrap_or_else(|| "<no building>".to_string()), o.room.clone().unwrap_or_else(|| "<no room>".to_string()));
                            if o.begin_time.is_some() && o.end_time.is_some() {
                                let begin_time = o.begin_time.clone().unwrap();
                                let end_time = o.end_time.clone().unwrap();
                                return format!("{} from {} to {}", output, fix_time(&begin_time), fix_time(&end_time));
                            }

                            output
                        })
                        .reduce(|a, b| format!("{}\n{}", a, b))
                        .unwrap_or_else(|| "No professors are assigned to this course.".to_string()),
                    false);
        }

        if let Ok(stats) = stats {
            if let Some(class_update) = stats.get("class") {
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
#[description = "Search for courses in a term."]
#[aliases("course")]
#[usage = "<CRN, Course Number, or Name> [Semester] [Year]"]
pub async fn courses(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let current_date = Local::now().date();
    let mut year = current_date.year();
    // You are required to specify if you want a summer class. Baka.
    let mut semester = if current_date.month() >= 3 && current_date.month() <= 10 { 30 } else { 10 };
    let mut search_query = String::new();

    while !args.is_empty() {
        if let Ok(numeric) = args.single::<i32>() {
            // Make sure it's not a year lol
            if numeric >= 10000 {
                let db = db!(ctx);
                match db.get_class(numeric).await {
                    Ok(option_class) => {
                        if let Some(class) = option_class {
                            course_embed(ctx, msg, &class).await?;
                        } else {
                            msg.channel_id.say(&ctx.http, format!("Could not find a class with the CRN `{}`.", numeric)).await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to get class: {}", ex);
                        msg.channel_id.say(&ctx.http, "Failed to query our database... try again later?").await?;
                    }
                }
                return Ok(())
            } else {
                year = numeric;
            }
        }

        let text = args.single::<String>().unwrap();
        if let Some(sem) = semester_from_text(&text) {
            semester = sem;
        } else {
            search_query.push(' ');
            search_query.push_str(&text);
        }
    }

    let term = year * 100 + semester;
    match search_course_by_number(ctx, msg, &search_query, term).await {
        Ok(any) => {
            if !any {
                match search_course_by_name(ctx, msg, &search_query, term).await {
                    Ok(any) => {
                        if !any {
                            msg.channel_id.say(&ctx.http, "Failed to find any classes with the given query. Did you mistype the input?").await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to search by name: {}", ex);
                        msg.channel_id.say(&ctx.http, "Failed to search for classes... try again later?").await?;
                    }
                }
            }
        }
        Err(ex) => {
            error!("Failed to search by name: {}", ex);
            msg.channel_id.say(&ctx.http, "Failed to search for classes... try again later?").await?;
        }
    }

    Ok(())
}

async fn search_course_by_number(ctx: &Context, msg: &Message, search_query: &str, term: i32) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let db = db!(ctx);
    let classes = db.search_class_by_number(search_query, term).await?;
    print_matches(ctx, msg, &classes).await?;

    Ok(!classes.is_empty())
}

async fn search_course_by_name(ctx: &Context, msg: &Message, search_query: &str, term: i32) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let db = db!(ctx);
    let classes = db.search_class_by_name(search_query, term).await?;
    print_matches(ctx, msg, &classes).await?;

    Ok(!classes.is_empty())
}

async fn print_matches(ctx: &Context, msg: &Message, classes: &[PartialClass]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if classes.is_empty() { return Ok(()); }

    if classes.len() == 1 {
        let db = db!(ctx);
        let class = db.get_class(classes[0].course_reference_number).await?.unwrap();
        course_embed(ctx, msg, &class).await?;
    } else {
        msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
            e.title("Class Search").description("Multiple results were found for your query. Search again using the CRN for a particular class.");
            e.field(format!("Classes Matched (totalling {})", classes.len()),
                    classes
                            .iter()
                            .take(10)
                            .map(|o| format!("`{}` - {}: {}", o.course_reference_number, o.course_number, o.course_title.clone().unwrap_or_else(|| "<unknown class name>".to_string())))
                            .reduce(|a, b| format!("{}\n{}", a, b))
                            .unwrap(),
                    false);
            e
        })).await?;
    }

    Ok(())
}