#!/bin/sh

set -eu

# It would be better not to start lxpanel
pkill lxpanel || true

DIR=$(dirname "$(realpath "$0")")
cd "${DIR}"

exec "$DIR/target/release/qzn3t-gui" -c "$DIR/qzn3t_mod_ui.sh" 2>&1 | systemd-cat -t qzn3t
