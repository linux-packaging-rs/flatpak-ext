use std::path::PathBuf;

use async_std::task::spawn;
use iced::{
    executor,
    futures::SinkExt,
    subscription,
    widget::{button, column, row, text},
    window, Alignment, Application, Command, Element, Length, Subscription, Theme,
};

#[derive(Debug)]
pub struct ProgressInfo {
    repo: String,
    action: String,
    app_ref: String,
    message: String,
    progress: f32,
    temp_repo: PathBuf,
    deps_repo: PathBuf,
    app_state: AppState,
    process: Option<rustix::process::Pid>,
}

#[derive(Clone, Debug)]
pub enum AppState {
    LoadingFile(RunApp),
    Done,
}

#[derive(Clone, Debug)]
pub enum RunApp {
    Bundle(PathBuf),
    // Download(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    Progress((String, String, String, String, f32)),
    Finished(rustix::process::Pid),
    Close,
}

impl Application for ProgressInfo {
    type Executor = executor::Default;
    type Flags = (PathBuf, PathBuf, AppState);
    type Message = Message;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (ProgressInfo, Command<Self::Message>) {
        (
            ProgressInfo {
                repo: "".into(),
                action: "".into(),
                app_ref: "".into(),
                message: "".into(),
                progress: 0.0,
                temp_repo: flags.0,
                deps_repo: flags.1,
                app_state: flags.2,
                process: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Flatrun")
    }

    fn view(&self) -> Element<Message> {
        match self.app_state {
            AppState::LoadingFile(_) => {
                column![
                    text("Flatrun: Run flatpaks without installing")
                        .horizontal_alignment(iced::alignment::Horizontal::Center),
                    row![
                        text(&self.repo)
                            .horizontal_alignment(iced::alignment::Horizontal::Left)
                            .width(Length::Fill),
                        text(&self.action)
                            .horizontal_alignment(iced::alignment::Horizontal::Right)
                            .width(Length::Fill),
                    ]
                    .width(Length::Fill),
                    row![
                        text(&self.app_ref)
                            .horizontal_alignment(iced::alignment::Horizontal::Left)
                            .width(Length::Fill),
                        text(&self.message)
                            .horizontal_alignment(iced::alignment::Horizontal::Right)
                            .width(Length::Fill),
                    ]
                    .width(Length::Fill),
                    iced::widget::progress_bar(0.0..=1.0, self.progress).width(Length::Fill),
                ]
            }
            AppState::Done => {
                column![
                    text("Flatrun: Run flatpaks without installing")
                        .horizontal_alignment(iced::alignment::Horizontal::Center),
                    row![
                        text(&self.app_ref)
                            .horizontal_alignment(iced::alignment::Horizontal::Left)
                            .width(Length::Fill),
                        text("Running Application")
                            .horizontal_alignment(iced::alignment::Horizontal::Right)
                            .width(Length::Fill),
                    ]
                    .width(Length::Fill),
                    button(text(format!("Close {}", &self.app_ref))).on_press(Message::Close)
                ]
            }
        }
        .padding(32)
        .align_items(Alignment::Center)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::Progress((repo, action, app_ref, message, progress)) => {
                self.repo = repo;
                self.action = action;
                self.app_ref = app_ref;
                self.message = message;
                self.progress = progress;
                log::info!("UPDATE!");
                Command::none()
            }
            Message::Finished(pid) => {
                self.app_state = AppState::Done;
                self.process = Some(pid);
                Command::none()
            }
            Message::Close => {
                log::info!("CLOSE REQUESTED!");
                if let Some(pid) = self.process {
                    if let Err(e) = rustix::process::kill_process(pid, rustix::process::Signal::Int)
                    {
                        log::error!("Failed to kill process: {:?}", e);
                        Command::none()
                    } else {
                        let _ = std::fs::remove_dir(&self.temp_repo);
                        window::close::<Message>(window::Id::MAIN)
                    }
                } else {
                    Command::none()
                }
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if let AppState::LoadingFile(f) = self.app_state.clone() {
            let (temp_repo, deps_repo) = (self.temp_repo.clone(), self.deps_repo.clone());
            subscription::channel(
                std::any::TypeId::of::<Message>(),
                50,
                move |mut output| async move {
                    match f {
                        RunApp::Bundle(path) => {
                            spawn(async move {
                                crate::run_bundle_inner(
                                    &temp_repo,
                                    &deps_repo,
                                    &path,
                                    &mut Some(&mut output),
                                )
                                .await
                                .unwrap();
                                let _ = output.send(Message::Close).await;
                            });
                        } // RunApp::Download(appid) => {
                          //     // TODO
                          // }
                    }
                    loop {
                        async_std::future::pending::<i32>().await;
                    }
                },
            )
        } else {
            Subscription::none()
        }
    }
}
