#!/bin/sh

set -eu

DIR=$(dirname "$(realpath "$0")")
$DIR/target/release/qzn3t-gui $DIR/qzn3t_mod_ui.sh 2>&1 > /tmp/gui.log

