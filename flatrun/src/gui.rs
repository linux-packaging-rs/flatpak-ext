#[derive(Debug)]
pub struct ProgressInfo {
    repo: String,
    action: String,
    app_ref: String,
    message: String,
    progress: f32,
    app: RunApp,
    temp_repo: TempDir,
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
    executor,
    futures::SinkExt,
    subscription,
    widget::{column, text},
    window, Application, Command, Element, Subscription, Theme,
};
use tempfile::TempDir;

impl Application for ProgressInfo {
    type Executor = executor::Default;
    type Flags = RunApp;
    type Message = Message;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (ProgressInfo, Command<Self::Message>) {
        let (temp_repo, deps_repo) = crate::get_repos().unwrap();
        log::info!(
            "temp_repo: {:?}, deps_repo: {:?}",
            temp_repo.path(),
            deps_repo
        );
        (
            ProgressInfo {
                repo: "".into(),
                action: "".into(),
                app_ref: "".into(),
                message: "".into(),
                progress: 0.0,
                app: flags,
                temp_repo,
                deps_repo,
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
                window::change_mode(window::Id::MAIN, window::Mode::Hidden)
            }
            Message::Done => {
                log::info!("CLOSE!");
                window::close::<Message>(window::Id::MAIN)
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let app = self.app.clone();
        let (temp_repo, deps_repo) = (
            self.temp_repo.path().to_path_buf().clone(),
            self.deps_repo.clone(),
        );
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
