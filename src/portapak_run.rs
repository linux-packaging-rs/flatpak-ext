use crate::{
    flatpak::{DependencyInstall, Flatpak, FlatpakRepo},
    PortapakError, RefType,
};

pub fn run(
    app: RefType,
    offline: bool,
    dependencies: DependencyInstall,
) -> Result<(), PortapakError> {
    log::info!("requested flatpak: {:?}", app);
    let repo = FlatpakRepo::new(offline)?;
    let flatpak = Flatpak::new(app, &repo, dependencies, offline)?;
    flatpak.run(&repo)?;
    Ok(())
}
