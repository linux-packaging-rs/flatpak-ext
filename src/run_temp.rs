use flatpak_ext::{
    run_temp::{run, Message},
    types::{Flatpak, FlatpakExtError, Repo},
};
use std::{env, fs::remove_dir_all};

use crate::utils::path_from_uri;

pub fn run_no_install(
    file: Option<String>,
    dep: Option<String>,
    app_id: Option<String>,
    remote: Option<String>,
    clean: bool,
) -> Result<(), FlatpakExtError> {
    if clean {
        let _ = remove_dir_all(env::temp_dir().join("flatrun"));
        log::trace!("Cleared directory: {:?}", env::temp_dir().join("flatrun"));
    }
    for e in env::vars().map(|(x, y)| format!("{}={}", x, y)) {
        log::trace!("{e}");
    }

    match file {
        Some(path) => {
            match run(
                Repo::temp(),
                Flatpak::Bundle(path_from_uri(path)),
                Some(Repo::default()),
                dep.map(|x| Flatpak::Bundle(path_from_uri(x))),
                remote,
                handle_message,
            ) {
                Ok(_) => Ok(()),
                Err(e) => {
                    log::error!("{:?}", e);
                    Err(e)
                }
            }
        }
        None => {
            if let Some(app_id) = app_id {
                match run(
                    Repo::temp_in(env::temp_dir().join("flatrun")),
                    Flatpak::Download(app_id),
                    Some(Repo::default()),
                    dep.map(|x| Flatpak::Bundle(path_from_uri(x))),
                    remote,
                    handle_message,
                ) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("{:?}", e);
                        Err(e)
                    }
                }
            } else {
                if !clean {
                    println!(
                        "'No args specified! {} run-temp --help' to see args",
                        env::current_exe().unwrap().to_string_lossy()
                    );
                }
                Ok(())
            }
        }
    }
}

fn handle_message(msg: Message) {
    match msg {
        Message::Install { r, progress, .. } => {
            println!("Installing {}... {}%", r, progress * 100.0);
        }
        Message::Running { n } => {
            println!("Running {}", n);
        }
        Message::Unknown => {
            println!("Unknown message!");
        }
    }
}
