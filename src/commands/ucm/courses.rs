use chrono::{Datelike, DateTime, Local, TimeZone, Utc};
use log::error;
use crate::{CowContext, cowdb, Error};
use std::error;
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
        10 => "Spring",
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

async fn course_embed(ctx: &CowContext<'_>, class: &Class) -> Result<(), Error> {
    let db = cowdb!(ctx);
    let professors = db.get_professors_for_class(class.id).await;
    let meetings = db.get_meetings_for_class(class.id).await;
    let stats = db.get_stats().await;

    ctx.send(|m| {
        m.embeds.clear();
        m.embed(|e| {
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
                                    return format!("{} ({} - {}) from {} to {} on {}", output, o.begin_date, o.end_date, fix_time(&begin_time), fix_time(&end_time), o.in_session);
                                }

                                output
                            })
                            .reduce(|a, b| format!("{}\n{}", a, b))
                            .unwrap_or_else(|| "No meetings are assigned to this course.".to_string()),
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
        })
    }).await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Search for courses in a term."),
    aliases("course", "class", "classes")
)]
pub async fn courses(
    ctx: CowContext<'_>,
    #[description = "CRN, course number, or name of class"] #[rest] query: Option<String>
) -> Result<(), Error> {
    let query = query.unwrap_or_default();

    if query.is_empty() {
        ctx.say("Type the CRN, course number, or name of the class to look it up.").await?;
        return Ok(());
    }

    let args = query.split(' ');

    let current_date = Local::now().date();
    let mut year = current_date.year();
    // You are required to specify if you want a summer class. Baka.
    let mut semester = if current_date.month() >= 3 && current_date.month() <= 10 { 30 } else { 10 };
    let mut search_query = String::new();

    for arg in args {
        if let Ok(numeric) = arg.parse::<i32>() {
            // Make sure it's not a year lol
            if numeric >= 10000 {
                let db = cowdb!(ctx);
                match db.get_class(numeric).await {
                    Ok(option_class) => {
                        if let Some(class) = option_class {
                            course_embed(&ctx, &class).await?;
                        } else {
                            ctx.say(format!("Could not find a class with the CRN `{}`.", numeric)).await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to get class: {}", ex);
                        ctx.say("Failed to query our database... try again later?").await?;
                    }
                }
                return Ok(())
            } else if numeric >= 2005 {
                year = numeric;
                continue;
            }
        }

        if let Some(sem) = semester_from_text(arg) {
            semester = sem;
        } else {
            search_query.push(' ');
            search_query.push_str(arg);
        }
    }

    let term = year * 100 + semester;
    match search_course_by_number(&ctx, &search_query, term).await {
        Ok(any) => {
            if !any {
                match search_course_by_name(&ctx, &search_query, term).await {
                    Ok(any) => {
                        if !any {
                            ctx.say("Failed to find any classes with the given query. Did you mistype the input?").await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to search by name: {}", ex);
                        ctx.say("Failed to search for classes... try again later?").await?;
                    }
                }
            }
        }
        Err(ex) => {
            error!("Failed to search by name: {}", ex);
            ctx.say("Failed to search for classes... try again later?").await?;
        }
    }

    Ok(())
}

async fn search_course_by_number(ctx: &CowContext<'_>, search_query: &str, term: i32) -> Result<bool, Box<dyn error::Error + Send + Sync>> {
    let db = cowdb!(ctx);
    let classes = db.search_class_by_number(search_query, term).await?;
    print_matches(ctx, &classes).await?;

    Ok(!classes.is_empty())
}

async fn search_course_by_name(ctx: &CowContext<'_>, search_query: &str, term: i32) -> Result<bool, Box<dyn error::Error + Send + Sync>> {
    let db = cowdb!(ctx);
    let classes = db.search_class_by_name(search_query, term).await?;
    print_matches(ctx, &classes).await?;

    Ok(!classes.is_empty())
}

async fn print_matches(ctx: &CowContext<'_>, classes: &[PartialClass]) -> Result<(), Box<dyn error::Error + Send + Sync>> {
    if classes.is_empty() { return Ok(()); }

    if classes.len() == 1 {
        let db = cowdb!(ctx);
        let class = db.get_class(classes[0].course_reference_number).await?.unwrap();
        course_embed(ctx, &class).await?;
    } else {
        ctx.send(|m| {
            m.embeds.clear();
            m.embed(|e| {
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
            })
        }).await?;
    }

    Ok(())
}