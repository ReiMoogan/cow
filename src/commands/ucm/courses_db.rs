use std::collections::HashMap;
use chrono::NaiveDateTime;
use num_traits::ToPrimitive;
use serenity::{
    model::id::{
        UserId
    }
};
use rust_decimal::{
    Decimal,
    prelude::FromPrimitive
};

use crate::Database;
use crate::commands::ucm::courses_db_models::*;
use crate::commands::ucm::courses::{to_term, to_crn};

impl Database {
    pub async fn get_user_reminders(&self, user_id: UserId) -> Result<Vec<Reminder>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let user_decimal = Decimal::from_u64(user_id.0).unwrap();
        let res = conn.query(
            "SELECT class_id, min_trigger, for_waitlist, triggered FROM [UniScraper].[UCM].[reminder] WHERE user_id = @P1",
            &[&user_decimal])
            .await?
            .into_first_result()
            .await?;

        let mut out: Vec<Reminder> = Vec::new();

        for reminder in res {
            out.push(Reminder {
                user_id: user_id.0,
                class_id: reminder.get(0).unwrap(),
                min_trigger: reminder.get(1).unwrap(),
                for_waitlist: reminder.get(2).unwrap(),
                triggered: reminder.get(3).unwrap()
            });
        }

        Ok(out)
    }

    pub async fn add_reminder(&self, reminder: &Reminder) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let user_decimal = Decimal::from_u64(reminder.user_id).unwrap();

        // Will panic if there is a duplicate, since I have uniqueness set.
        conn.execute(
            "INSERT INTO [UniScraper].[UCM].[reminder] (user_id, class_id, min_trigger, for_waitlist, triggered) VALUES (@P1, @P2, @P3, @P4, @P5)",
            &[&user_decimal, &reminder.class_id, &reminder.min_trigger, &reminder.for_waitlist, &reminder.triggered])
            .await?;

        Ok(())
    }

    pub async fn remove_reminder(&self, user_id: UserId, course_reference_number: i32) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let user_decimal = Decimal::from_u64(user_id.0).unwrap();

        let total = conn.execute(
            "DELETE FROM [UniScraper].[UCM].[reminder] WHERE user_id = @P1 AND course_reference_number = @P2",
            &[&user_decimal, &course_reference_number])
            .await?.total();

        Ok(total > 0)
    }

    pub async fn trigger_reminders(&self) -> Result<Vec<Trigger>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;

        let res = conn.simple_query(
            "EXEC [UniScraper].[UCM].[TriggerReminders]")
            .await?
            .into_first_result()
            .await?;

        let mut out: Vec<Trigger> = Vec::new();

        for reminder in res {
            let user_id: Decimal = reminder.get(0).unwrap();
            let course_id: i32 = reminder.get(1).unwrap();
            out.push(Trigger {
                user_id: user_id.to_u64().unwrap(),
                course_reference_number: to_crn(course_id),
                term: to_term(course_id),
                min_trigger: reminder.get(2).unwrap()
            });
        }

        Ok(out)
    }

    pub async fn get_class(&self, course_reference_number: i32, term: i32) -> Result<Option<Class>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let res = conn.query(
            "SELECT id, term, course_number, campus_description, course_title, credit_hours, maximum_enrollment, enrollment, seats_available, wait_capacity, wait_available FROM [UniScraper].[UCM].[class] WHERE course_reference_number = @P1 AND term = @P2",
            &[&course_reference_number, &term])
            .await?
            .into_row()
            .await?;

        let mut out: Option<Class> = None;

        if let Some(class) = res {
            let course_number: &str = class.get(2).unwrap();
            let campus_description: Option<&str> = class.get(3);
            let course_title: Option<&str> = class.get(4);
            out = Some(Class {
                id: class.get(0).unwrap(),
                term,
                course_reference_number,
                course_number: course_number.to_string(),
                campus_description: campus_description.map(|o| o.to_string()),
                course_title: course_title.map(|o| o.to_string()),
                credit_hours: class.get(5).unwrap(),
                maximum_enrollment: class.get(6).unwrap(),
                enrollment: class.get(7).unwrap(),
                seats_available: class.get(8).unwrap(),
                wait_capacity: class.get(9).unwrap(),
                wait_available: class.get(10).unwrap()
            });
        }

        Ok(out)
    }

    // Note: class_id is referring to an ID stored in the database, not the CRN. Fetch this through get_class.
    pub async fn get_professors_for_class(&self, class_id: i32) -> Result<Vec<Professor>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let res = conn.query(
            "SELECT rmp_id, last_name, first_name, email, department, num_ratings, rating, difficulty, would_take_again_percent, full_name FROM [UniScraper].[UCM].[professor] INNER JOIN [UniScraper].[UCM].[faculty] ON professor.email = faculty.professor_email WHERE class_id = @P1;",
            &[&class_id])
            .await?
            .into_first_result()
            .await?;

        let mut out: Vec<Professor> = Vec::new();

        for professor in res {
            let rmp_id: Option<&str> = professor.get(0);
            let last_name: &str = professor.get(1).unwrap();
            let first_name: &str = professor.get(2).unwrap();
            let email: &str = professor.get(3).unwrap();
            let department: Option<&str> = professor.get(4);
            let full_name: &str = professor.get(9).unwrap();
            out.push(Professor {
                rmp_id: rmp_id.map(|o| o.to_string()),
                last_name: last_name.to_string(),
                first_name: first_name.to_string(),
                email: email.to_string(),
                department: department.map(|o| o.to_string()),
                num_ratings: professor.get(5).unwrap(),
                rating: professor.get(6).unwrap(),
                difficulty: professor.get(7).unwrap(),
                would_take_again_percent: professor.get(8).unwrap(),
                full_name: full_name.to_string()
            });
        }

        Ok(out)
    }

    // Note: class_id is referring to an ID stored in the database, not the CRN. Fetch this through get_class.
    pub async fn get_meetings_for_class(&self, class_id: i32) -> Result<Vec<Meeting>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let res = conn.query(
            "SELECT begin_time, end_time, begin_date, end_date, building, building_description, campus, campus_description, room, credit_hour_session, hours_per_week, in_session, meeting_type FROM [UniScraper].[UCM].[meeting] WHERE class_id = @P1;",
            &[&class_id])
            .await?
            .into_first_result()
            .await?;

        let mut out: Vec<Meeting> = Vec::new();

        for meeting in res {
            let begin_time: Option<&str> = meeting.get(0);
            let end_time: Option<&str> = meeting.get(1);
            let begin_date: &str = meeting.get(2).unwrap();
            let end_date: &str = meeting.get(3).unwrap();
            let building: Option<&str> = meeting.get(4);
            let building_description: Option<&str> = meeting.get(5);
            let campus: Option<&str> = meeting.get(6);
            let campus_description: Option<&str> = meeting.get(7);
            let room: Option<&str> = meeting.get(8);
            let meeting_type: u8 = meeting.get(12).unwrap();
            out.push(Meeting {
                class_id,
                begin_time: begin_time.map(|o| o.to_string()),
                end_time: end_time.map(|o| o.to_string()),
                begin_date: begin_date.to_string(),
                end_date: end_date.to_string(),
                building: building.map(|o| o.to_string()),
                building_description: building_description.map(|o| o.to_string()),
                campus: campus.map(|o| o.to_string()),
                campus_description: campus_description.map(|o| o.to_string()),
                room: room.map(|o| o.to_string()),
                credit_hour_session: meeting.get(9).unwrap(),
                hours_per_week: meeting.get(10).unwrap(),
                in_session: Days::from_bits(meeting.get(11).unwrap()).unwrap(),
                meeting_type: MeetingType::try_from(meeting_type).unwrap()
            });
        }

        Ok(out)
    }

    pub async fn get_description_for_course(&self, course_number: &str) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let res = conn.query(
            "SELECT course_description FROM [UniScraper].[UCM].[description] WHERE @P1 LIKE course_number + '%';",
            &[&course_number])
            .await?
            .into_row()
            .await?;

        if let Some(row) = res {
            let description: Option<&str> = row.get(0);
            return Ok(description.map(|o| o.to_string()));
        }

        Ok(None)
    }

    // Course number is like CSE-031.
    pub async fn search_class_by_number(&self, course_number: &str, term: i32) -> Result<Vec<PartialClass>, Box<dyn std::error::Error + Send + Sync>> {
        self.general_class_search(course_number, term,
                                  "SELECT id, course_reference_number, course_number, course_title \
                                  FROM UniScraper.UCM.class \
                                  WHERE term = @P1 AND CONTAINS(course_number, @P2);").await
    }

    // Course name is like Computer Organization and Assembly.
    pub async fn search_class_by_name(&self, course_name: &str, term: i32) -> Result<Vec<PartialClass>, Box<dyn std::error::Error + Send + Sync>> {
        self.general_class_search(course_name, term,
          "SELECT id, course_reference_number, course_number, course_title FROM \
                    (SELECT id, course_reference_number, course_number, course_title, term, ROW_NUMBER() \
                        OVER (PARTITION BY course_title ORDER BY course_reference_number) AS RowNumber \
                        FROM UniScraper.UCM.class WHERE term = @P1 AND CONTAINS(course_title, @P2)) AS mukyu \
                    WHERE mukyu.RowNumber = 1;").await
    }

    fn create_full_text_query(&self, search_query: &str) -> String {
        search_query
            .trim()
            .split(' ')
            .map(|o| o.replace(['(', ')', '\"', '\''], "")) // *unqueries your query*
            .map(|o| format!("\"*{o}*\"")) // Wildcards
            .reduce(|a, b| format!("{a} AND {b}"))
            .unwrap()
    }

    async fn general_class_search(&self, search_query: &str, term: i32, sql: &str) -> Result<Vec<PartialClass>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;

        let input = self.create_full_text_query(search_query);

        let res = conn.query(sql, &[&term, &input])
            .await?
            .into_first_result()
            .await?;

        let mut out: Vec<PartialClass> = Vec::new();

        for class in res {
            let course_number: &str = class.get(2).unwrap();
            let course_title: Option<&str> = class.get(3);

            let item = PartialClass {
                id: class.get(0).unwrap(),
                course_reference_number: class.get(1).unwrap(),
                course_number: course_number.to_string(),
                course_title: course_title.map(|o| o.to_string())
            };

            if search_query == course_number || course_title.map(|o| o == search_query).unwrap_or(false) {
                // Return early with one item
                return Ok(vec![item]);
            } else {
                out.push(item);
            }
        }

        Ok(out)
    }

    pub async fn search_professor(&self, search_query: &str) -> Result<Vec<Professor>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;

        let input = self.create_full_text_query(search_query);

        let res = conn.query("SELECT rmp_id, last_name, first_name, email, department, num_ratings, rating, difficulty, would_take_again_percent, full_name FROM [UniScraper].[UCM].[professor] WHERE CONTAINS(full_name, @P1);", &[&input])
            .await?
            .into_first_result()
            .await?;

        let mut out: Vec<Professor> = Vec::new();

        for professor in res {
            let rmp_id: Option<&str> = professor.get(0);
            let last_name: &str = professor.get(1).unwrap();
            let first_name: &str = professor.get(2).unwrap();
            let email: &str = professor.get(3).unwrap();
            let department: Option<&str> = professor.get(4);
            let full_name: &str = professor.get(9).unwrap();
            out.push(Professor {
                rmp_id: rmp_id.map(|o| o.to_string()),
                last_name: last_name.to_string(),
                first_name: first_name.to_string(),
                email: email.to_string(),
                department: department.map(|o| o.to_string()),
                num_ratings: professor.get(5).unwrap(),
                rating: professor.get(6).unwrap(),
                difficulty: professor.get(7).unwrap(),
                would_take_again_percent: professor.get(8).unwrap(),
                full_name: full_name.to_string()
            });
        }

        Ok(out)
    }

    pub async fn get_classes_for_professor(&self, professor_email: &str, term: i32) -> Result<Vec<PartialClass>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;

        let res = conn.query("SELECT class.id, class.course_reference_number, class.course_number, class.course_title FROM [UniScraper].[UCM].[professor] \
            INNER JOIN [UniScraper].[UCM].[faculty] ON professor.email = faculty.professor_email \
            INNER JOIN [UniScraper].[UCM].[class] ON class.id = faculty.class_id \
            WHERE class.term = @P1 AND professor.email = @P2", &[&term, &professor_email])
            .await?
            .into_first_result()
            .await?;

        let mut out: Vec<PartialClass> = Vec::new();

        for class in res {
            let course_number: &str = class.get(2).unwrap();
            let course_title: Option<&str> = class.get(3);

            let item = PartialClass {
                id: class.get(0).unwrap(),
                course_reference_number: class.get(1).unwrap(),
                course_number: course_number.to_string(),
                course_title: course_title.map(|o| o.to_string())
            };

            out.push(item);
        }

        Ok(out)
    }

    pub async fn get_stats(&self) -> Result<HashMap<String, NaiveDateTime>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let res = conn.simple_query(
            "SELECT table_name, last_update FROM [UniScraper].[UCM].[stats];")
            .await?
            .into_first_result()
            .await?;

        let mut out = HashMap::new();

        for meeting in res {
            let table_name: &str = meeting.get(0).unwrap();
            let last_update: NaiveDateTime = meeting.get(1).unwrap();
            out.insert(table_name.to_string(), last_update);
        }

        Ok(out)
    }
}