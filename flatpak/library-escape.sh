export LIBRARY_PATH=/run/host/$(flatpak-spawn --host sh -c "echo $LIBRARY_PATH")
export FLATPAK_USER_DIR=$HOME/.local/share/flatpak
$@