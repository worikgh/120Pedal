#!/bin/sh

## Utility to switch between `mod-ui` and `qzn3t`.  Used by the GUI
## front end

# -e: Exit immediately if any command fails
# -u: Treat unset variables as an error
set -eu


# Validate arguments
if [ $# -ne 1 ]; then
    echo "Usage: $0 [true|false]" >&2
    exit 1
fi

COMMAND="$1"

# Get the root directory from the pedal from this script's absolute
# path
SCRIPT_PATH=$(realpath "$0")
SCRIPT_DIR=$(dirname "$SCRIPT_PATH")
One20PedalHome=$(dirname "$SCRIPT_DIR")
echo DBG qzn3t_mod_ui.sh: One20PedalHome: ${One20PedalHome}

RUN_QZN3T="${One20PedalHome}/examples/qzn3t_gui"
RUN_MODUI="${One20PedalHome}/examples/mod-ui"
if [ ! -x "$RUN_QZN3T" ]; then
    echo "Error: $RUN_QZN3T not found or not executable" >&2
    exit 1
fi
if [ ! -x "$RUN_MODUI" ]; then
    echo "Error: $RUN_MODUI not found or not executable" >&2
    exit 1
fi

case "$COMMAND" in
    true)
        if ! "$RUN_QZN3T"; then
            echo "Error: Failed to run ${RUN_QZN3T}" >&2
            exit 1
        fi
        ;;
    false)
        if ! "$RUN_MODUI"; then
            echo "Error: Failed to run ${RUN_MODUI}" >&2
            exit 1
        fi
        ;;
    *)
        echo "Usage: $0 [true|false]" >&2
        exit 1
        ;;
esac

exit 0
