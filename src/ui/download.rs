use iced::subscription;

use std::hash::Hash;
use rs_timeskip_archiver::{AddFileParams, FileParams};

// Just a little utility function
pub fn file<I: 'static + Hash + Copy + Send + Sync>(
    id: I,
    file_bundle: AddFileParams,
) -> iced::Subscription<(I, Progress)> {
    subscription::unfold(id, State::Ready(file_bundle), move |state| {
        download(id, state)
    })
}

#[derive(Debug, Clone)]
pub struct Download<'a, I> {
    id: I,
    param_file_array: AddFileParams<'a>,
}

async fn download<'a, I: Copy>(id: I, state: State<'a>) -> ((I, Progress), State<'a>) {
    match state {
        State::Ready(param_file_array) => {
            let response: Result<Vec<_>, _> = param_file_array
                .iter()
                .enumerate()
                .map(|(index, file_path)| rs_timeskip_archiver::add_file(*file_path))
                .collect();

            match response {
                Ok(response) => {
                    if let Some(total) = response.content_length() {
                        (
                            (id, Progress::Started),
                            State::Downloading {
                                response,
                                total,
                                downloaded: 0,
                            },
                        )
                    } else {
                        ((id, Progress::Errored), State::Finished)
                    }
                }
                Err(_) => ((id, Progress::Errored), State::Finished),
            }
        }
        State::Downloading {
            mut response,
            total,
            downloaded,
        } => match response.chunk().await {
            Ok(Some(chunk)) => {
                let downloaded = downloaded + chunk.len() as u64;

                let percentage = (downloaded as f32 / total as f32) * 100.0;

                (
                    (id, Progress::Advanced(percentage)),
                    State::Downloading {
                        response,
                        total,
                        downloaded,
                    },
                )
            }
            Ok(None) => ((id, Progress::Finished), State::Finished),
            Err(_) => ((id, Progress::Errored), State::Finished),
        },
        State::Finished => {
            // We do not let the stream die, as it would start a
            // new download repeatedly if the user is not careful
            // in case of errors.
            iced::futures::future::pending().await
        }
    }
}

#[derive(Debug, Clone)]
pub enum Progress {
    Started,
    Advanced(f32),
    Finished,
    Errored,
}

pub enum State<'a> {
    Ready(Vec<AddFileParams<'a>>),
    Downloading {
        response: reqwest::Response,
        total: u64,
        downloaded: u64,
    },
    Finished,
}