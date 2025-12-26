#!/bin/sh

set -eu
INVERT=${1-1} # Default to true
DIR=$(dirname "$(realpath "$0")")
cd ${DIR}

# Specify x/y as "Fullscreen" using the SDL2 GUI library disapears off the bottom of the screen
($DIR/target/release/qzn3t-gui -c $DIR/qzn3t_mod_ui.sh -x 1280 -y 640  2>&1) |systemd-cat -t qzn3t
