#!/bin/sh

set -eu
INVERT=${1-0}
DIR=$(dirname "$(realpath "$0")")
cd ${DIR}
($DIR/target/release/qzn3t-gui $DIR/qzn3t_mod_ui.sh $INVERT 2>&1) > /tmp/gui.log

