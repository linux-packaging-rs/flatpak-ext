#[derive(Debug)]
pub struct ProgressInfo {
    repo: String,
    action: String,
    app_ref: String,
    message: String,
    progress: f32,
    app: RunApp,
    temp_repo: PathBuf,
    deps_repo: PathBuf,
}

#[derive(Clone, Debug)]
pub enum RunApp {
    Bundle(PathBuf),
    Download(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdateProgress((String, String, String, String, f32)),
    Hide,
    Done,
}

use std::path::PathBuf;

use async_std::task::spawn;
use iced::{
    command, executor,
    futures::SinkExt,
    subscription,
    widget::{column, text},
    window, Application, Command, Element, Theme,
};

impl Application for ProgressInfo {
    type Executor = executor::Default;
    type Flags = (RunApp, PathBuf, PathBuf);
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
                app: flags.0,
                temp_repo: flags.1,
                deps_repo: flags.2,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Flatrun")
    }

    fn view(&self) -> Element<Message> {
        // We use a column: a simple vertical layout
        column![
            text("Flatrun: Run flatpaks without installing"),
            text(&self.repo),
            text(&self.action),
            text(&self.app_ref),
            text(&self.message),
            iced::widget::progress_bar(0.0..=1.0, self.progress),
        ]
        .into()
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::UpdateProgress((repo, action, app_ref, message, progress)) => {
                self.repo = repo;
                self.action = action;
                self.app_ref = app_ref;
                self.message = message;
                self.progress = progress;
                log::info!("UPDATE!");
                Command::none()
            }
            Message::Hide => {
                log::info!("HIDE!");
                Command::batch([
                    window::minimize(window::Id::MAIN, true), // see: https://github.com/rust-windowing/winit/issues/2388#issuecomment-1416733516
                    window::change_mode::<Message>(window::Id::MAIN, window::Mode::Hidden),
                ])
            }
            Message::Done => {
                log::info!("CLOSE!");
                let _ = std::fs::remove_dir(&self.temp_repo);
                window::close::<Message>(window::Id::MAIN)
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let app = self.app.clone();
        let (temp_repo, deps_repo) = (self.temp_repo.clone(), self.deps_repo.clone());
        subscription::channel(
            std::any::TypeId::of::<Message>(),
            50,
            move |mut output| async move {
                match app {
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
                            output.send(Message::Done).await.unwrap();
                        });
                    }
                    RunApp::Download(appid) => {
                        // TODO
                    }
                }
                loop {
                    async_std::future::pending::<i32>().await;
                }
            },
        )
    }
}
