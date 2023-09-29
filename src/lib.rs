pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender as FuturesSender;
use sha3::{Digest, Sha3_256};
use std::env;

use crate::models::{File, NewFile, NewProfile, Profile};

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

pub fn add_file(
    conn: &mut SqliteConnection,
    file_path: &str,
    pid: &i32,
    tx: &mut FuturesSender<(usize, usize)>,
    current_file_index: usize,
    total_files: usize,
) -> Result<usize, diesel::result::Error> {
    println!("add_file started for {}", file_path);
    use schema::files;

    let file_path_buf = std::path::PathBuf::from(file_path);

    if file_path_buf.exists() {
        // Hash file
        let mut file_blob = std::fs::File::open(file_path).unwrap();
        let mut hasher = Sha3_256::new();
        std::io::copy(&mut file_blob, &mut hasher).unwrap();

        let file_out_hash = format!("{:x}", hasher.finalize());
        let new_file = NewFile {
            file_name: file_path,
            sha256: &file_out_hash,
            profile_id: *pid,
        };

        // Insert file into database
        diesel::insert_into(files::table)
            .values(&new_file)
            .execute(conn)?;

        // Send progress update
        if tx.try_send((current_file_index, total_files)).is_err() {
            println!("Receiver has been dropped.");
        }
        println!("add_file ended for {}", file_path);

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
