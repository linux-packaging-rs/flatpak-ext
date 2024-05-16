export LIBRARY_PATH=$(flatpak-spawn --host sh -c "echo $LIBRARY_PATH")
export FLATPAK_USER_DIR=$HOME/.local/share/flatpak
$@