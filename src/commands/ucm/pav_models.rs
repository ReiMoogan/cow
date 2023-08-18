use serde::Deserialize;
use std::convert::{TryFrom, From};
use std::fmt::{Display, Formatter};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use chrono::{Datelike, DateTime, Local, NaiveTime, Weekday};

#[derive(FromPrimitive)]
pub enum Day {
    Sunday = 0,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday
}

pub enum Meal {
    Breakfast,
    Lunch,
    Dinner,
    Other(String) // Force a search.
}

impl TryFrom<u32> for Day {
    type Error = ();
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        FromPrimitive::from_u32(v).ok_or(())
    }
}

impl From<Weekday> for Day {
    fn from(v: Weekday) -> Self {
        match v {
            Weekday::Mon => Day::Monday,
            Weekday::Tue => Day::Tuesday,
            Weekday::Wed => Day::Wednesday,
            Weekday::Thu => Day::Thursday,
            Weekday::Fri => Day::Friday,
            Weekday::Sat => Day::Saturday,
            Weekday::Sun => Day::Sunday
        }
    }
}

impl TryFrom<&str> for Day {
    type Error = ();
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        if v.len() < 2 {
            // Can't predict the date off one or zero chars.
            return Err(());
        }

        match &v.to_lowercase()[..2] {
            "su" => Ok(Day::Sunday),
            "mo" => Ok(Day::Monday),
            "tu" => Ok(Day::Tuesday),
            "we" => Ok(Day::Wednesday),
            "th" => Ok(Day::Thursday),
            "fr" => Ok(Day::Friday),
            "sa" => Ok(Day::Saturday),
            &_ => Err(())
        }
    }
}

impl From<Day> for String {
    fn from(val: Day) -> Self {
        match val {
            Day::Sunday => "su".to_string(),
            Day::Monday => "mo".to_string(),
            Day::Tuesday => "tu".to_string(),
            Day::Wednesday => "we".to_string(),
            Day::Thursday => "th".to_string(),
            Day::Friday => "fr".to_string(),
            Day::Saturday => "sa".to_string()
        }
    }
}

impl Display for Day {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Day::Sunday => write!(f, "Sunday"),
            Day::Monday => write!(f, "Monday"),
            Day::Tuesday => write!(f, "Tuesday"),
            Day::Wednesday => write!(f, "Wednesday"),
            Day::Thursday => write!(f, "Thursday"),
            Day::Friday => write!(f, "Friday"),
            Day::Saturday => write!(f, "Saturday")
        }
    }
}

impl From<&str> for Meal {
    fn from(v: &str) -> Self {
        match v.to_lowercase().as_str() {
            "breakfast" => Meal::Breakfast,
            "lunch" => Meal::Lunch,
            "dinner" => Meal::Dinner,
            other => Meal::Other(other.to_string())
        }
    }
}

impl From<Meal> for String {
    fn from(val: Meal) -> Self {
        match val {
            Meal::Breakfast => "breakfast".to_string(),
            Meal::Lunch => "lunch".to_string(),
            Meal::Dinner => "dinner".to_string(),
            Meal::Other(x) => x
        }
    }
}

impl Display for Meal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Meal::Breakfast => write!(f, "Breakfast"),
            Meal::Lunch => write!(f, "Lunch"),
            Meal::Dinner => write!(f, "Dinner"),
            Meal::Other(x) => write!(f, "{x}")
        }
    }
}

// Shrinking some models down since they're pretty large.

pub trait PavData {}

#[derive(Debug, Deserialize)]
pub struct PavResult<T> {
    pub code: u16,
    pub message: String,
    pub data: T
}

// Pavilion Info

#[derive(Debug, Deserialize)]
pub struct Location {
    // WHY DOES THIS HAVE BOTH _id AND id IN THE JSON???
    // Turns out they send _id more than id >:[
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "locationSpecialGroupIds")]
    pub location_special_group_ids: Option<Vec<LocationSpecialGroupIds>>
}

#[derive(Debug, Deserialize)]
pub struct LocationSpecialGroupIds {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String
}

#[derive(Debug, Deserialize)]
pub struct Company {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "locationInfo")]
    pub location_info: Location
}

// Pavilion Groups

#[derive(Debug, Deserialize)]
pub struct Group {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub order: Option<i32>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MenuGroups {
    pub menu_groups: Vec<Group>,
    pub menu_categories: Vec<Group>
}

impl MenuGroups {
    #[allow(dead_code)]
    fn search_all<'a>(array: &'a [Group], query: &str) -> Vec<&'a Group> {
        let query_lower = query.to_lowercase();
        array.iter().filter(|x| x.name.to_lowercase().contains(&query_lower)).collect::<Vec<_>>()
    }

    #[allow(dead_code)]
    fn search(array: &[Group], query: &str) -> Option<String> {
        Self::search_all(array, query).iter().min_by(|a, b| a.name.len().cmp(&b.name.len()) ).map(|o| o.id.clone())
    }

    #[allow(dead_code)]
    pub fn get_group(&self, day: &Day) -> Option<String> {
        Self::search(&self.menu_groups, &day.to_string())
    }

    #[allow(dead_code)]
    pub fn get_groups(&self, day: &Day) -> Vec<&Group> {
        Self::search_all(&self.menu_groups, &day.to_string())
    }

    #[allow(dead_code)]
    pub fn get_category(&self, meal: &Meal) -> Option<String> {
        MenuGroups::search(&self.menu_categories, &meal.to_string())
    }

    #[allow(dead_code)]
    pub fn get_categories(&self, meal: &Meal) -> Vec<&Group> {
        MenuGroups::search_all(&self.menu_categories, &meal.to_string())
    }
}

// Pavilion Items

#[derive(Debug, Deserialize)]
pub struct Item {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub description: String
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MenuItems {
    pub menu_items: Vec<Item>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RawMaterial {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String
}

// Pavilion Times (hard-coded, IDK if there's an API for them)
pub struct PavilionTime;

impl PavilionTime {

    // Turns out from_hms_opt is not a constant function, so... this monstrosity has to occur.
    // At least inlining is a thing.

    #[inline(always)]
    pub fn breakfast_weekday_start() -> NaiveTime { NaiveTime::from_hms_opt(7, 0, 0).unwrap() }
    #[inline(always)]
    pub fn breakfast_weekend_start() -> NaiveTime { NaiveTime::from_hms_opt(9, 0, 0).unwrap() }
    #[inline(always)]
    pub fn breakfast_end() -> NaiveTime { NaiveTime::from_hms_opt(10, 30, 0).unwrap() }
    #[inline(always)]
    pub fn lunch_start() -> NaiveTime { NaiveTime::from_hms_opt(11, 0, 0).unwrap() }
    #[inline(always)]
    pub fn lunch_end() -> NaiveTime { NaiveTime::from_hms_opt(15, 0, 0).unwrap() }
    #[inline(always)]
    pub fn dinner_start() -> NaiveTime { NaiveTime::from_hms_opt(16, 0, 0).unwrap() }
    #[inline(always)]
    pub fn dinner_end() -> NaiveTime { NaiveTime::from_hms_opt(21, 0, 0).unwrap() }


    pub fn next_meal(datetime: &DateTime<Local>) -> (Day, Meal) {
        let day = Day::from(datetime.weekday());
        let time = datetime.time();

        if time < PavilionTime::breakfast_end() {
            return (day, Meal::Breakfast);
        } else if time < PavilionTime::lunch_end() {
            return (day, Meal::Lunch);
        } else if time < PavilionTime::dinner_end() {
            return (day, Meal::Dinner);
        }

        // Give them the breakfast from the day after.
        (Day::from(datetime.weekday().succ()), Meal::Breakfast)
    }
}

// Yablokoff Wallace Dining Center Times (also hard-coded)
pub struct YablokoffTime;

impl YablokoffTime {
    #[inline(always)]
    pub fn dinner_start() -> NaiveTime { NaiveTime::from_hms_opt(15, 0, 0).unwrap() }
    #[inline(always)]
    pub fn dinner_end() -> NaiveTime { NaiveTime::from_hms_opt(21, 0, 0).unwrap() }
    #[inline(always)]
    pub fn late_night_start() -> NaiveTime { NaiveTime::from_hms_opt(21, 0, 0).unwrap() }
    #[inline(always)]
    pub fn late_night_end() -> NaiveTime { NaiveTime::from_hms_opt(0, 0, 0).unwrap() }

    #[allow(dead_code)]
    pub fn is_dinner(day_of_week: &Day) -> bool {
        // Ensure it's not a weekend, since it's closed then.
        !(matches!(day_of_week, Day::Saturday) || matches!(day_of_week, Day::Sunday))
    }
}