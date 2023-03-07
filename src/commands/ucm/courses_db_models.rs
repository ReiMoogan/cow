use std::fmt::{Display, Formatter};
use bitflags::bitflags;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub struct Reminder {
    pub user_id: u64,
    pub class_id: i32,
    pub min_trigger: i32,
    pub for_waitlist: bool,
    pub triggered: bool
}

pub struct Trigger {
    pub user_id: u64,
    pub course_reference_number: i32,
    pub term: i32,
    pub min_trigger: i32
}

pub struct Class {
    pub id: i32,
    pub term: i32,
    pub course_reference_number: i32,
    pub course_number: String,
    pub campus_description: Option<String>,
    pub course_title: Option<String>,
    pub credit_hours: u8,
    pub maximum_enrollment: i16,
    pub enrollment: i16,
    pub seats_available: i16,
    pub wait_capacity: i16,
    pub wait_available: i16
}

pub struct PartialClass {
    pub id: i32,
    pub course_reference_number: i32,
    pub course_number: String,
    pub course_title: Option<String>
}

bitflags! {
    pub struct Days: u8 {
        const BASE = 0;
        const SUNDAY = 1;
        const MONDAY = 2;
        const TUESDAY = 4;
        const WEDNESDAY = 8;
        const THURSDAY = 16;
        const FRIDAY = 32;
        const SATURDAY = 64;
    }
}

impl Display for Days {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let days_copy = *self;
        if days_copy == Days::BASE {
            return write!(f, "<no days assigned>");
        }

        let mut days = String::new();

        if days_copy.contains(Days::SUNDAY) { days.push_str("Sunday, "); }
        if days_copy.contains(Days::MONDAY) { days.push_str("Monday, "); }
        if days_copy.contains(Days::TUESDAY) { days.push_str("Tuesday, "); }
        if days_copy.contains(Days::WEDNESDAY) { days.push_str("Wednesday, "); }
        if days_copy.contains(Days::THURSDAY) { days.push_str("Thursday, "); }
        if days_copy.contains(Days::FRIDAY) { days.push_str("Friday, "); }
        if days_copy.contains(Days::SATURDAY) { days.push_str("Saturday, "); }

        write!(f, "{}", &days[0..days.len() - 2])
    }
}

#[repr(u8)]
#[derive(FromPrimitive)]
pub enum MeetingType {
    Lecture = 1,
    Discussion = 2,
    Lab = 3,
    Fieldwork = 4,
    Seminar = 5,
    IndividualStudy = 6,
    Tutorial = 7,
    Studio = 8,
    Practicum = 9,
    Exam = 10,
    Project = 11,
    Internship = 12
}

impl Display for MeetingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MeetingType::Lecture => write!(f, "Lecture"),
            MeetingType::Discussion => write!(f, "Discussion"),
            MeetingType::Lab => write!(f, "Lab"),
            MeetingType::Fieldwork => write!(f, "Fieldwork"),
            MeetingType::Seminar => write!(f, "Seminar"),
            MeetingType::IndividualStudy => write!(f, "Individual Study"),
            MeetingType::Tutorial => write!(f, "Tutorial"),
            MeetingType::Studio => write!(f, "Studio"),
            MeetingType::Practicum => write!(f, "Practicum"),
            MeetingType::Exam => write!(f, "Exam"),
            MeetingType::Project => write!(f, "Project"),
            MeetingType::Internship => write!(f, "Internship")
        }
    }
}

impl TryFrom<u8> for MeetingType {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        FromPrimitive::from_u8(v).ok_or(())
    }
}

pub struct Meeting {
    pub class_id: i32,
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
    pub begin_date: String,
    pub end_date: String,
    pub building: Option<String>,
    pub building_description: Option<String>,
    pub campus: Option<String>,
    pub campus_description: Option<String>,
    pub room: Option<String>,
    pub credit_hour_session: f32,
    pub hours_per_week: f32,
    pub in_session: Days,
    pub meeting_type: MeetingType
}

pub struct Professor {
    pub email: String,
    pub rmp_id: Option<String>,
    pub last_name: String,
    pub first_name: String,
    pub full_name: String,
    pub department: Option<String>,
    pub num_ratings: i32,
    pub rating: f32,
    pub difficulty: f32,
    pub would_take_again_percent: f32
}