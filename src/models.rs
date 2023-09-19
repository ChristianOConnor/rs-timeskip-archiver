use super::schema::{profiles, files};
use diesel::prelude::*;


#[derive(Queryable, Selectable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = profiles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Profile {
    pub id: i32,
    pub profile_name: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl ToString for Profile {
    fn to_string(&self) -> String {
        self.profile_name.clone()
    }
}

#[derive(Insertable)]
#[diesel(table_name = profiles)]
pub struct NewProfile<'a> {
    pub profile_name: &'a str
}

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = files)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct File {
    pub id: i32,
    pub file_name: String,
    pub sha256: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub profile_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = files)]
pub struct NewFile<'a> {
    pub file_name: &'a str,
    pub sha256: &'a str,
    pub profile_id: i32,
}
