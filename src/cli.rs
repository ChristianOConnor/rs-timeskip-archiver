use dialoguer::{theme::ColorfulTheme, Select, Input};
use rs_timeskip_archiver_rewrite1::{create_profile, get_profiles, add_file, get_files};
use tabled::{builder::Builder, settings::Style};
use unicode_width::UnicodeWidthStr;
use std::fs::File;
use std::io::Write;
use diesel::prelude::*;
use futures::channel::mpsc;


pub fn run_cli(connection: &mut SqliteConnection) {
    loop {
        let mainmenu = &[
            "Create Profile",
            "Select Profile",
            "Exit",
        ];

        let selection_mainmenu = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Welcome, Please select from the following options:")
            .default(0)
            .items(&mainmenu[..])
            .interact()
            .unwrap();

        if selection_mainmenu == 0 {
            loop {
                let input: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Please type your profile name. (Type 'Back' to go back)")
                    .interact_text()
                    .unwrap();

                if input.to_lowercase() == "back" {
                    break;
                }

                let _ = create_profile(connection, &input);
                break;
            }
        } else if selection_mainmenu == 1 {
            loop {
                let profiles_response = get_profiles(connection);
                let mut profiles: Vec<String> = profiles_response.iter().map(|profile| profile.profile_name.clone()).collect();
                profiles.push("Back".to_string());

                if profiles.is_empty() {
                    println!("No profiles found. Please create a profile first.");
                    break;
                }

                let profiles_list = &profiles[..];
                let selection_profile = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Please select your profile.")
                    .default(0)
                    .items(&profiles_list[..])
                    .interact()
                    .unwrap();

                if profiles_list[selection_profile] == "Back" {
                    break;
                }

                let selected_profile = &profiles_response[selection_profile];

                loop {
                    let profile_menu = &[
                        "Display all files",
                        "Enter a new file path",
                        "Back",
                    ];

                    let selection_profile_menu = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Please select an option:")
                        .default(0)
                        .items(&profile_menu[..])
                        .interact()
                        .unwrap();

                    if selection_profile_menu == 0 {
                        let files_display_choice_menu = &[
                            "Display in terminal",
                            "Print to File",
                            "Back",
                        ];

                        let selection_files_display_choice_menu = Select::with_theme(&ColorfulTheme::default())
                            .with_prompt("Please select an option:")
                            .default(0)
                            .items(&files_display_choice_menu[..])
                            .interact()
                            .unwrap();
                        
                        if selection_files_display_choice_menu == 0 {
                            println!("Files in profile '{}':", selected_profile.profile_name);
                            let files_display = get_files(connection, &selected_profile.id);
                            let mut builder = Builder::new();

                            for file in &files_display {
                                let file_name = file.file_name.chars().take(20).collect::<String>(); // truncate to 20 characters
                                let sha256 = file.sha256.chars().take(20).collect::<String>(); // truncate to 20 characters
                                let created_at = file.created_at.to_string().chars().take(20).collect::<String>(); // truncate to 20 characters
                                let updated_at = file.updated_at.to_string().chars().take(20).collect::<String>(); // truncate to 20 characters

                                builder.push_record([file_name, sha256, created_at, updated_at]);
                            }
                            let table = builder.build()
                                .with(Style::ascii_rounded())
                                .to_string();
                            println!("{}", table);
                        } else if selection_files_display_choice_menu == 1 {
                            println!("Files in profile '{}':", selected_profile.profile_name);
                            let files_display = get_files(connection, &selected_profile.id);
                            let mut builder = Builder::new();
                            
                            for file in &files_display {
                                builder.push_record([file.file_name.to_string(), file.sha256.to_string(), file.created_at.to_string(), file.updated_at.to_string()]);
                            }
                            let table = builder.build()
                                .with(Style::ascii_rounded())
                                .to_string();

                            let mut f = File::create("files_in_profile_results.txt").unwrap();
                            f.write_all(table.as_bytes()).unwrap();
                        } else if selection_files_display_choice_menu == 2 {
                            break;
                        } else {
                            println!("Error");
                        }
                    } else if selection_profile_menu == 1 {
                        let filepath_input: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Type in a file path to add to the profile. (Type 'Back' to go back)")
                            .interact_text()
                            .unwrap();

                        if filepath_input.to_lowercase() == "back" {
                            break;
                        }

                        let (tx, _rx) = futures::channel::mpsc::channel::<(usize, usize)>(1);
                        let mut tx_clone = tx.clone();
                        match add_file(connection, &filepath_input, &selected_profile.id, &mut tx_clone, 1, 1) {
                            Ok(file_find_response) => println!("{}", file_find_response),
                            Err(e) => println!("Failed to add file: {}", e),
                        }
                    } else if selection_profile_menu == 2 {
                        break;
                    } else {
                        println!("Error");
                    }
                }
            }
        } else if selection_mainmenu == 2 {
            break;
        } else {
            println!("Error");
        }
    }
}