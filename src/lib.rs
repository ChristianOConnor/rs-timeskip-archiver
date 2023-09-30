pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender as FuturesSender;
use sha3::{Digest, Sha3_256};
use std::env;

use crate::models::{File, NewFile, NewProfile, Profile};



pub struct AddFileParams<'a> {
    pub connection: &'a mut diesel::SqliteConnection,
    pub file_path_str: &'a str,
    pub profile_id: &'a i32,
    pub tx_clone: &'a mut FuturesSender<(usize, usize)>,
    pub index: usize,
    pub total_files: usize,
}

pub trait FileParams {
    fn connection(&mut self) -> &mut diesel::SqliteConnection;
    fn file_path_str(&self) -> &str;
    fn profile_id(&self) -> &i32;
    fn tx_clone(&mut self) -> &mut FuturesSender<(usize, usize)>;
    fn index(&self) -> usize;
    fn total_files(&self) -> usize;
}

impl<'a> FileParams for AddFileParams<'a> {
    fn connection(&mut self) -> &mut diesel::SqliteConnection {
        self.connection
    }

    fn file_path_str(&self) -> &str {
        self.file_path_str
    }

    fn profile_id(&self) -> &i32 {
        self.profile_id
    }

    fn tx_clone(&mut self) -> &mut FuturesSender<(usize, usize)> {
        self.tx_clone
    }

    fn index(&self) -> usize {
        self.index
    }

    fn total_files(&self) -> usize {
        self.total_files
    }
}

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_profile(conn: &mut SqliteConnection, profile_name: &str) {
    use schema::profiles;

    let new_profile = NewProfile { profile_name };

    diesel::insert_into(profiles::table)
        .values(&new_profile)
        .execute(conn)
        .expect("Error saving new profile");
}

pub fn get_profiles(conn: &mut SqliteConnection) -> Vec<Profile> {
    use schema::profiles::dsl::*;

    profiles
        .load::<Profile>(conn)
        .expect("Error loading profiles")
}

pub fn add_file(file_param: &mut AddFileParams) -> Result<usize, diesel::result::Error> {
    println!("add_file started for {}", file_param.file_path_str);
    use schema::files;

    let file_path_buf = std::path::PathBuf::from(file_param.file_path_str);

    if file_path_buf.exists() {
        // Hash file
        let mut file_blob = std::fs::File::open(file_param.file_path_str).unwrap();
        let mut hasher = Sha3_256::new();
        std::io::copy(&mut file_blob, &mut hasher).unwrap();

        let file_out_hash = format!("{:x}", hasher.finalize());
        let new_file = NewFile {
            file_name: file_param.file_path_str,
            sha256: &file_out_hash,
            profile_id: *file_param.profile_id,
        };

        // Insert file into database
        diesel::insert_into(files::table)
            .values(&new_file)
            .execute(file_param.connection)?;

        // Send progress update
        if file_param.tx_clone.try_send((file_param.index, file_param.total_files)).is_err() {
            println!("Receiver has been dropped.");
        }
        println!("add_file ended for {}", file_param.file_path_str);

        Ok(2)
    } else {
        Err(diesel::result::Error::NotFound)
    }
}

pub fn get_files(conn: &mut SqliteConnection, pid: &i32) -> Vec<File> {
    use schema::files::dsl::*;

    files
        .filter(profile_id.eq(pid))
        .load::<File>(conn)
        .expect("Error loading files")
}
