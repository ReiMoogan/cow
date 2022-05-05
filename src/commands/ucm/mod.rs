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

use serenity::framework::standard::macros::group;

use crate::commands::ucm::reminders::REMINDERS_GROUP;

use library::*;
use courses::*;
use courses_old::*;
use pavilion::*;
use professors::*;

#[group]
#[prefixes("ucm", "ucmerced")]
#[description = "Get information about UC Merced's services and facilities."]
#[summary = "UC Merced info"]
#[commands(library, courses, courses_old, pavilion, professors)]
#[sub_groups(reminders)]
struct UCM;