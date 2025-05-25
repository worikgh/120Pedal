#!/bin/sh

## Utility to switch between `mod-ui` and `qzn3t`.  Used by the GUI
## front end

# -e: Exit immediately if any command fails
# -u: Treat unset variables as an error
set -eu

COMMAND="$1"

RUN_QZN3T="/home/patch/120Pedal/examples/qzn3t_gui"
RUN_MODUI="/home/patch/120Pedal/examples/mod-ui"

# Validate arguments
if [ $# -ne 1 ]; then
    echo "Usage: $0 [true|false]" >&2
    exit 1
fi

case "$COMMAND" in
    true)
        if [ ! -x "$RUN_QZN3T" ]; then
            echo "Error: $RUN_QZN3T not found or not executable" >&2
            exit 1
        fi
        if ! "$RUN_QZN3T"; then
            echo "Error: Failed to run ${RUN_QZN3T}" >&2
            exit 1
        fi
        ;;
    false)
        if [ ! -x "$RUN_MODUI" ]; then
            echo "Error: $RUN_MODUI not found or not executable" >&2
            exit 1
        fi
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
