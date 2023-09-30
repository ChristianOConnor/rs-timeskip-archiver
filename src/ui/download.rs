use iced::subscription;

use std::hash::Hash;
use rs_timeskip_archiver::{AddFileParams, FileParams};

// Just a little utility function
pub fn file<I: 'static + Hash + Copy + Send + Sync>(
    id: I,
    file_bundle: Vec<AddFileParams>,
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
        State::Ready(mut param_file_array) => {
            let response: Result<Vec<_>, _> = param_file_array
                .iter_mut()
                .enumerate()
                .map(|(index, file_path)| rs_timeskip_archiver::add_file(&mut *file_path))
                .collect();

            match response {
                Ok(mut response) => {
                    if let Some(response) = Some(2) {
                        println!("add_file ran");
                        (
                            (id, Progress::Started),
                            State::Ready(param_file_array),
                        )
                    } else {
                        ((id, Progress::Errored), State::Finished)
                    }
                }
                Err(_) => ((id, Progress::Errored), State::Finished),
            }
        }
        _ => ((id, Progress::Errored), State::Finished),
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
    Finished,
}