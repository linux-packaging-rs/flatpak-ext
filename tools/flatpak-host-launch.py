#!/usr/bin/env python
import configparser
import subprocess
import argparse
import shutil
import shlex
import sys
import os


class FlatpakInfoWrapper:
    def __init__(self) -> None:
        flatpak_info = configparser.ConfigParser()
        flatpak_info.read("/.flatpak-info")
        self.app_path = flatpak_info["Instance"]["app-path"]
        self.runtime_path = flatpak_info["Instance"]["runtime-path"]

    def to_host_path(self, path: str) -> str:
        if path.startswith("/app"):
            return os.path.join(self.app_path, path.removeprefix("/app/"))
        elif path.startswith("/usr"):
            return os.path.join(self.runtime_path, path.removeprefix("/usr/"))
        else:
            return path

    def get_ld_path(self) -> str:
        result = subprocess.run(["ldconfig", "-p"], capture_output=True, text=True)
        for line in result.stdout.splitlines():
            if line.strip().startswith("ld-linux"):
                return self.to_host_path(line.split(" => ")[1].strip())

    def get_all_lib_paths(self) -> list[str]:
        path_list: list[str] = []
        result = subprocess.run(["ldconfig", "-v"], capture_output=True, text=True)
        for line in result.stdout.splitlines():
            if not line.startswith("\t"):
                path_list.append(self.to_host_path(line.split(":")[0]))
        return path_list


def get_shebang(info_wrapper: FlatpakInfoWrapper, path: str) -> list[str]:
    try:
        with open(path, "r", encoding="utf-8") as f:
            first_line = f.readline()
    except Exception:
        return []

    if not first_line.startswith("#!"):
        return []

    shebang = shlex.split(first_line.removeprefix("#!"))

    shebang[0] = info_wrapper.to_host_path(shebang[0])

    return shebang


def check_permission() -> None:
    # Check if we have the needed permission
    # The Freedesktop Runtime don't include a dbus lib for python, so we just use gdbus here
    command = ["gdbus", "call", "--session", "--dest", "org.freedesktop.Flatpak", "--object-path", "/org/freedesktop/Flatpak/Development", "--method", "org.freedesktop.DBus.Peer.Ping"]
    result = subprocess.run(command, capture_output=True)

    if result.returncode != 0:
        print("The Flatpak is missing the --talk-name=org.freedesktop.Flatpak permission", file=sys.stderr)
        sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pkexec", action="store_true", help="Run as root using pkexec")
    parser.add_argument("--env", action='append', default=[], help="Set environment variable (can be used multiple times)")
    parser.add_argument("--working-directory", default="~", help="Set working directory")
    parser.add_argument("command", nargs="+", help="The command")
    args = parser.parse_args()

    if not os.path.isfile("/.flatpak-info"):
        print("flatpak-host-launch needs to be executed inside a Flatpak")
        sys.exit(1)

    check_permission()

    executable = shutil.which(args.command[0])
    if executable is None:
        print(args.command[0] + " was not found", file=sys.stderr)
        sys.exit(1)

    info_wrapper = FlatpakInfoWrapper()
    ld_path = info_wrapper.get_ld_path()
    lib_paths = info_wrapper.get_all_lib_paths()

    command = ["flatpak-spawn", "--host"]

    if args.pkexec:
        command += ["pkexec", "--disable-internal-agent"]

    command += ["env", "LD_LIBRARY_PATH=" + ":".join(lib_paths)]

    for env_name in ("XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"):
        command.append(f"{env_name}={os.getenv(env_name)}")

    # Set Python env vars
    command.append(f"PYTHONHOME={info_wrapper.runtime_path}")
    command.append("PYTHONPATH=" + info_wrapper.to_host_path(f"/app/lib/python{sys.version_info.major}.{sys.version_info.minor}/site-packages"))

    # Set custim env vars
    command += args.env

    command += [ld_path, "--library-path", ":".join(lib_paths)]

    command += get_shebang(info_wrapper, executable)

    command.append(info_wrapper.to_host_path(executable))
    command += args.command[1:]

    result = subprocess.run(command, cwd=os.path.expanduser(args.working_directory))

    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
