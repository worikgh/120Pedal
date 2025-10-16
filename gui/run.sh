#!/bin/sh

set -eu
INVERT=${1-1} # Default to true
DIR=$(dirname "$(realpath "$0")")
cd ${DIR}

($DIR/target/release/qzn3t-gui $DIR/qzn3t_mod_ui.sh 2>&1) |systemd-cat -t qzn3t
