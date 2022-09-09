mod library;
mod libcal_models;
mod courses;
mod courses_old;
mod professors;
mod course_models;
mod pavilion;
mod pav_models;
pub mod reminders;
mod courses_db;
mod courses_db_models;
mod foodtrucks;
mod calendar;
mod gym;
mod store;

use library::*;
use courses::*;
use courses_old::*;
use pavilion::*;
use professors::*;
use foodtrucks::*;
use calendar::*;
use gym::*;
use store::*;
use reminders::*;
use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("library", "courses", "courses_old", "pavilion", "professors", "foodtrucks", "calendar", "gym", "store", "reminders"),
    description_localized("en", "Get information about UC Merced's services and facilities."),
    aliases("ucmerced")
)]
pub async fn ucm(_ctx: CowContext<'_>) -> Result<(), Error> {
    Ok(())
}