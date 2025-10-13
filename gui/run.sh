#!/bin/sh

set -eu
INVERT=${1-1} # Default to true
DIR=$(dirname "$(realpath "$0")")
cd ${DIR}

## Width and height passed (480x250) because of a bug.  If not
## supplied the max-width, max-height are supposed to be used.  But
## the window is too tall and the lower row of buttons is invisible
($DIR/target/release/qzn3t-gui $DIR/qzn3t_mod_ui.sh 480 250 2>&1) |systemd-cat -t qzn3t
