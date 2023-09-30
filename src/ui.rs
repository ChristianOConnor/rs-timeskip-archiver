use diesel::SqliteConnection;
use futures::channel::mpsc::{self, Sender};
use futures::StreamExt;
use iced::widget::{
    text_input, button, Button, column, Column, Container, PickList, ProgressBar, Row, Rule, Scrollable, Space, text, Text,
};
use iced::Subscription;
use iced::{Alignment, Application, Command, Element, Length, Settings};
use rs_timeskip_archiver::models::{File, Profile};
use rs_timeskip_archiver::{get_files, get_profiles, AddFileParams};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use rayon::prelude::*;
mod download;
use futures::channel::mpsc::Sender as FuturesSender;



pub fn run_ui(connection: SqliteConnection) -> Result<(), iced::Error> {
    // Run the UI with the given database connection
    Archiver::run(Settings::with_flags(Arc::new(Mutex::new(connection))))
}

// Define the possible messages that can be sent to the UI
#[derive(Debug, Clone)]
pub enum Message {
    ProfileInputChanged(String),
    RunBlank,
    CreateProfile,
    LoadProfiles,
    ProfilesLoaded(Vec<Profile>),
    ProfileSelected(Profile),
    LoadFiles,
    FilesLoaded(Vec<File>),
    FileSelected(File),
    OpenFileDialog,
    FileChosen(Result<Vec<PathBuf>, String>),
    ProfileRefresh,
    Refresh,
    UpdateFileUploadProgress(Arc<Mutex<mpsc::Receiver<(usize, usize)>>>),
    Add,
    Download(usize),
    DownloadProgressed((usize, download::Progress)),
}

// Define the possible loading states for the UI
#[derive(Clone)]
pub enum LoadingState {
    Idle,
    Loading(String),
    Loaded,
}

// Define the main UI struct
pub struct Archiver<'a> {
    input_value: String,
    profiles: Vec<Profile>,
    selected_profile: Option<Profile>,
    connection: Arc<Mutex<SqliteConnection>>,
    scrollable_state_left: iced::widget::scrollable::State,
    scrollable_state_right: iced::widget::scrollable::State,
    files: Vec<File>,
    selected_file: Option<File>,
    loading_state: LoadingState,
    file_upload_progress: FileUploadProgress,
    downloads: Vec<Download<'a>>,
    last_id: usize,
    array_of_fileparams: Vec<AddFileParams<'a>>,
}

// Define the file upload progress struct
#[derive(Clone)]
struct FileUploadProgress {
    current: usize,
    total: usize,
}

impl FileUploadProgress {
    // Calculate the current progress ratio
    fn ratio(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.current as f32 / self.total as f32
        }
    }
}

impl<'a> Application for Archiver<'a> {
    // Define the application type, message type, flags type, and theme type
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = Arc<Mutex<SqliteConnection>>;
    type Theme = iced::theme::Theme;

    // Initialize the UI with the given flags (database connection)
    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                input_value: String::new(),
                profiles: Vec::new(),
                selected_profile: None,
                connection: flags,
                scrollable_state_left: iced::widget::scrollable::State::new(),
                scrollable_state_right: iced::widget::scrollable::State::new(),
                files: Vec::new(),
                selected_file: None,
                loading_state: LoadingState::Idle,
                file_upload_progress: FileUploadProgress {
                    current: 0,
                    total: 0,
                },
                downloads: vec![Download::new(0)],
                last_id: 0,
                array_of_fileparams: Vec::new(),
            },
            // Load the profiles asynchronously and send a message when done
            Command::perform(async { Message::LoadProfiles }, |_| Message::LoadProfiles),
        )
    }

    // Set the title of the UI window
    fn title(&self) -> String {
        String::from("Archiver")
    }

    // Handle incoming messages and return a command to execute
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        println!("Received message: {:?}", message);
        match message {
            Message::LoadProfiles => {
                // Load the profiles from the database and send a message when done
                let connection = Arc::clone(&self.connection);
                let profiles = get_profiles(&mut *connection.lock().unwrap());
                Command::perform(async { profiles }, Message::ProfilesLoaded)
            }
            Message::ProfilesLoaded(profiles) => {
                // Update the UI with the loaded profiles
                self.profiles = profiles;
                Command::none()
            }
            Message::ProfileInputChanged(value) => {
                // Update the input value for creating a new profile
                self.input_value = value;
                Command::none()
            }
            Message::CreateProfile => {
                // Create a new profile with the given name
                let connection = Arc::clone(&self.connection);
                let _ = rs_timeskip_archiver::create_profile(
                    &mut *connection.lock().unwrap(),
                    &self.input_value,
                );
                self.input_value.clear();
                Command::perform(async { Message::ProfileRefresh }, |msg| msg)
            }
            Message::ProfileSelected(profile) => {
                // Select a profile and load its files
                self.selected_profile = Some(profile);
                Command::perform(async { Message::LoadFiles }, |msg| msg)
            }
            Message::LoadFiles => {
                // Load the files for the selected profile
                if let Some(profile) = &self.selected_profile {
                    let connection = Arc::clone(&self.connection);
                    let files = get_files(&mut *connection.lock().unwrap(), &profile.id);
                    Command::perform(async { files }, Message::FilesLoaded)
                } else {
                    Command::none()
                }
            }
            Message::RunBlank => {
                // Select a file
                println!("Running blank");
                println!(
                    "From RunBlank: Rendering progress bar with ratio: {}",
                    self.file_upload_progress.ratio()
                );
                println!(
                    "From RunBlank: Current Files: {}",
                    self.file_upload_progress.current
                );
                println!(
                    "From RunBlank: Total Files: {}",
                    self.file_upload_progress.total
                );
                Command::none()
            }
            Message::FilesLoaded(files) => {
                // Update the UI with the loaded files
                self.files = files;
                Command::none()
            }
            Message::FileSelected(file) => {
                // Select a file
                self.selected_file = Some(file);
                Command::none()
            }
            Message::OpenFileDialog => {
                // Open a file dialog to choose files to upload
                println!("Open file dialog called.");
                Command::perform(open_file_dialog(), Message::FileChosen)
            }
            Message::Add => {
                self.last_id += 1;

                self.downloads.push(Download::new(self.last_id));
                Command::none()
            }
            Message::Download(index) => {
                if let Some(download) = self.downloads.get_mut(index) {
                    download.start();
                }
                Command::none()
            }
            Message::DownloadProgressed((id, progress)) => {
                if let Some(download) =
                    self.downloads.iter_mut().find(|download| download.id == id)
                {
                    download.progress(progress);
                }
                Command::none()
            }
            Message::FileChosen(file_paths_result) => {
                // Upload the chosen files for the selected profile
                if let Some(profile) = &self.selected_profile {
                    if let Ok(file_paths) = file_paths_result.clone() {
                        // Clone necessary data for thread.
                        let connection = Arc::clone(&self.connection);
                        let profile_id = profile.id.clone();
                        let file_paths_for_length = file_paths.clone();
                        let file_paths_for_thread = file_paths.clone();

                        // Create a channel for communication.
                        let (tx, mut rx) = mpsc::channel::<(usize, usize)>(1);

                        let mut file_params_array: Vec<AddFileParams> = Vec::new();

                        for path in file_paths_for_thread {
                            let mut tx_clone = tx.clone(); // clone the sender and declare it as mutable
                            let file_path_str = path.to_str().unwrap_or("");
                            
                            let mut connection = connection.lock().unwrap();
                            let file_params = AddFileParams {
                                connection: &mut *connection,
                                file_path_str,
                                profile_id: &profile_id,
                                tx_clone: &mut tx_clone,
                                index: 0,
                                total_files: 0,
                            };
                            file_params_array.push(file_params);
                        }

                        self.array_of_fileparams = file_params_array;
                        let _ = Download::new(0, self.array_of_fileparams);

                        Command::none()
                    } else {
                        // Handle file dialog error here...
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }

            Message::UpdateFileUploadProgress(rx_clone) => {
                // Create a new clone for the async block
                let rx_for_async = Arc::clone(&rx_clone);
            
                // Create the Command using this new clone
                let cmd = Command::perform(
                    async move { Arc::clone(&rx_for_async) },
                    Message::UpdateFileUploadProgress
                );
                
                let mut rx = rx_clone.lock().unwrap();
            
                match rx.try_next() {
                    Ok(val) => match val {
                        Some(msg) => {
                            println!("Received progress: {} / {}", msg.0, msg.1);
                            self.file_upload_progress.current += 1;
                            if self.file_upload_progress.current >= self.file_upload_progress.total {
                                self.file_upload_progress.current = 0;
                                self.file_upload_progress.total = 0;
            
                                self.loading_state = LoadingState::Loaded;
                                return Command::perform(async { Message::Refresh }, |msg| msg);
                            }
            
                            cmd
                        }
                        None => Command::none(),
                    },
                    Err(e) => cmd,
                }
            }

            Message::ProfileRefresh => {
                // Refresh the profiles
                Command::perform(async { Message::LoadProfiles }, |_| Message::LoadProfiles)
            }
            Message::Refresh => {
                // Refresh the files for the selected profile
                if let Some(profile) = &self.selected_profile {
                    let connection = Arc::clone(&self.connection);
                    let files = get_files(&mut *connection.lock().unwrap(), &profile.id);
                    Command::perform(async { files }, Message::FilesLoaded)
                } else {
                    Command::none()
                }
            }
        }
    }

    // Subscribe to progress updates if there are any ongoing file uploads
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(self.downloads.iter().map(Download::subscription))
    }

    // Define the UI view
    fn view(&self) -> Element<Self::Message> {
        let downloads = Column::with_children(
            self.downloads.iter().map(Download::view).collect(),
        )
        .push(
            button("Add another download")
                .on_press(Message::Add)
                .padding(10),
        )
        .spacing(20)
        .align_items(Alignment::End);

        let pick_list = PickList::new(
            &self.profiles,
            self.selected_profile.clone(),
            Message::ProfileSelected,
        );

        let profile_text_input = text_input("New profile name here...", &self.input_value)
            .on_input(Message::ProfileInputChanged);

        let mut top_bar = Row::new()
            .padding(15)
            .spacing(20)
            .align_items(Alignment::Center)
            .push(Text::new("Select Profile:"))
            .push(pick_list.width(Length::FillPortion(1)))
            .push(profile_text_input)
            .push(Button::new(Text::new("Create Profile")).on_press(Message::CreateProfile));

        if self.selected_profile.is_some() {
            top_bar = top_bar
                .push(Button::new(Text::new("Upload File")).on_press(Message::OpenFileDialog));
        }

        let file_names_panel = self.files.iter().fold(Column::new(), |column, file| {
            let truncated_file_name = if file.file_name.len() > 20 {
                file.file_name[0..20].to_string() + "..."
            } else {
                file.file_name.clone()
            };
            column.push(
                Button::new(Text::new(truncated_file_name.clone()))
                    .on_press(Message::FileSelected(file.clone())),
            )
        });

        let file_properties_panel = if let Some(file) = &self.selected_file {
            Column::new()
                .push(Text::new(format!("Name: {}", &file.file_name)))
                .push(Text::new(format!("SHA256: {}", &file.sha256)))
                .push(Text::new(format!("Created At: {}", &file.created_at)))
                .push(Text::new(format!("Updated At: {}", &file.updated_at)))
        } else {
            Column::new()
        };

        let mut content = Column::new().spacing(10).padding(10).push(top_bar);

        if self.selected_profile.is_some() {
            content = content.push(Rule::horizontal(10)).push(
                Row::new()
                    .push(Scrollable::new(file_names_panel).width(Length::FillPortion(1)))
                    .push(Scrollable::new(file_properties_panel).width(Length::FillPortion(1))),
            );
        }

        // This is a filler container to push everything else down
        content = content.push(Container::new(Space::new(Length::Fill, Length::Shrink)));

        // The progress bar part
        println!(
            "Rendering progress bar with ratio: {}",
            self.file_upload_progress.ratio()
        );
        content = content.push(
            Column::new()
                .push(Space::new(Length::Fill, Length::Fill)) // Add this line
                .push(Text::new("File Upload Progress"))
                .push(downloads),
        );

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct Download<'a> {
    id: usize,
    state: State,
    parameter: AddFileParams<'a>
}

#[derive(Debug)]
enum State {
    Idle,
    Downloading { progress: f32 },
    Finished,
    Errored,
}

impl<'a> Download<'a> {
    pub fn new(id: usize, parameter: AddFileParams) -> Self {
        Download {
            id,
            state: State::Idle,
            parameter,
        }
    }

    pub fn start(&mut self) {
        match self.state {
            State::Idle { .. }
            | State::Finished { .. }
            | State::Errored { .. } => {
                self.state = State::Downloading { progress: 0.0 };
            }
            State::Downloading { .. } => {}
        }
    }

    pub fn progress(&mut self, new_progress: download::Progress) {
        if let State::Downloading { progress } = &mut self.state {
            match new_progress {
                download::Progress::Started => {
                    *progress = 0.0;
                }
                download::Progress::Advanced(percentage) => {
                    *progress = percentage;
                }
                download::Progress::Finished => {
                    self.state = State::Finished;
                }
                download::Progress::Errored => {
                    self.state = State::Errored;
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.state {
            State::Downloading { .. } => {
                download::file(self.id, self.parameter)
                    .map(Message::DownloadProgressed)
            }
            _ => Subscription::none(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let current_progress = match &self.state {
            State::Idle { .. } => 0.0,
            State::Downloading { progress } => *progress,
            State::Finished { .. } => 100.0,
            State::Errored { .. } => 0.0,
        };

        //let progress_bar = progress_bar(0.0..=100.0, current_progress);

        let control: Element<_> = match &self.state {
            State::Idle => button("Start the download!")
                .on_press(Message::Download(self.id))
                .into(),
            State::Finished => {
                column!["Download finished!", button("Start again")]
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .into()
            }
            State::Downloading { .. } => {
                text(format!("Downloading... {current_progress:.2}%")).into()
            }
            State::Errored => column![
                "Something went wrong :(",
                button("Try again").on_press(Message::Download(self.id)),
            ]
            .spacing(10)
            .align_items(Alignment::Center)
            .into(),
        };

        Column::new()
            .spacing(10)
            .padding(10)
            .align_items(Alignment::Center)
            .push(control)
            .into()
    }
}

// Open a file dialog to choose files to upload
async fn open_file_dialog() -> Result<Vec<PathBuf>, String> {
    if let Some(paths) = rfd::FileDialog::new().pick_files() {
        Ok(paths.into_iter().map(|path| path.into()).collect()) // Convert into Vec<PathBuf>
    } else {
        Err("No file was selected or an error occurred".into())
    }
}
