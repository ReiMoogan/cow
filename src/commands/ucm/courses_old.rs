use tracing::error;
use crate::{CowContext, Error};
use chrono::Datelike;
use crate::commands::ucm::course_models::{CourseList};

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the course list for a major.")
)]
pub async fn courses_old(
    ctx: CowContext<'_>,
    #[description = "The semester: Fall, Spring, or Summer"] selected_sem: String,
    #[description = "The major: ENGR, CSE, etc."] selected_major: String)
-> Result<(), Error> {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()?;

    let now = chrono::Utc::now();
    let sem_code = match selected_sem.to_lowercase().as_str() {
        "fall" => "10",
        "spring" => "20",
        "summer" => "30",
        _ => "00"
    };

    let term = format!("{}{}", now.year(), sem_code);


    // setting the session cookies
    let term_url = format!("https://reg-prod.ec.ucmerced.edu/StudentRegistrationSsb/ssb/term/search?\
        mode=courseSearch\
        &term={}\
        &studyPath=\
        &studyPathText=\
        &startDatepicker=\
        &endDatepicker=", term);
    let search_url = "https://reg-prod.ec.ucmerced.edu/StudentRegistrationSsb/ssb/courseSearch/courseSearch";

    client.get(term_url).send().await?;
    client.get(search_url).send().await?;

    let major = selected_major.to_uppercase();
    
    let url = format!("https://reg-prod.ec.ucmerced.edu/StudentRegistrationSsb/ssb/courseSearchResults/courseSearchResults?\
        txt_subject={}\
        &txt_term={}\
        &startDatepicker=\
        &endDatepicker=\
        &pageOffset=0\
        &pageMaxSize=10\
        &sortColumn=subjectDescription\
        &sortDirection=asc", major, term);

    match client.get(url).send().await {
        Ok(response) => {
            // TODO: add pagination for courses
            match response.json::<CourseList>().await {
                Ok(course_list) => {
                    ctx.send(|m| {
                        m.embeds.clear();
                        m.embed(|e| {
                            e
                                .title("Course List")
                                .description(format!("For major: {}", major));

                            for course in course_list.data {
                                let title = course.course_title.unwrap_or_else(|| "No Title".into());
                                e.field(format!("{} {}-{}", major, course.course_number.unwrap_or_else(|| "000".into()), title),
                                    course.course_description.unwrap_or_else(|| "No description".into())+"...", false);
                            }

                            e
                        })
                    }).await?;
                }
                Err(ex) => {
                    ctx.say("The course search gave us weird data, try again later?").await?;
                    error!("Failed to process course search: {}", ex);
                }
            }
        }
        Err(ex) => {
            ctx.say("Failed to connect to the course search API, try again later?").await?;
            error!("Failed to get course search: {}", ex);
        }
    }

    Ok(())
}