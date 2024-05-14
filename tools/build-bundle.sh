#!/bin/bash


usage() {
    echo "Usage: $0 [options]"
    echo "Options:"
    echo "  -h, --help                                        Display this help message"
    echo "  <PATH_TO_FLATPAKREF> <NAME> <BRANCH> <OUT_FILE>   Create a bundle from the flatpakref"
    # Add more options here as needed
}

# Check if the user has provided the help option
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    usage
    return 0
fi

# Check if no arguments were provided
if [ -z "$1" ]; then
    echo "No path provided."
    return 1
else
    echo "Argument provided: $1"
fi

PATH_TO_FLATPAKREF=$1

# Check if no arguments were provided
if [ -z "$2" ]; then
    echo "No app name provided."
    return 1
else
    echo "Argument provided: $2"
fi

APP_NAME=$2

# Check if no arguments were provided
if [ -z "$3" ]; then
    echo "No branch provided."
    return 1
else
    echo "Argument provided: $3"
fi

BRANCH=$3

# Check if no arguments were provided
if [ -z "$4" ]; then
    echo "No output file provided."
    return 1
else
    echo "Argument provided: $4"
fi

OUT_FILE=$4

# Check if flatpak command exists
if command -v flatpak &> /dev/null; then
    echo "Flatpak is installed."
else
    echo "Flatpak is not installed."
    return
fi


random_string=$(head /dev/urandom | tr -dc A-Za-z0-9 | head -c 10)

user_dir=$HOME/.cache/build-bundle-$random_string

env FLATPAK_USER_DIR=$user_dir flatpak install --user -y $PATH_TO_FLATPAKREF

env FLATPAK_USER_DIR=$user_dir flatpak build-bundle $user_dir/repo/ $OUT_FILE $APP_NAME $BRANCH

rm -r $user_dir