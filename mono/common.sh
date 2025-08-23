#!/bin/sh

# Function to set NAME and JACKNAME
get_jackname() {
  # Get the name of the script
  NAME=$(basename "$0")

  # Return the name
  echo "${NAME}_mono"
}

# Export the function so it can be used in other scripts
export -f get_jackname

# Utilise the fact that this script lives in a directory immediately
# below 120Pedal
get_120Home() {
  # Get the directory of the script
  SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

  # Get the parent directory (120Pedal directory)
  PARENT_DIR=$(dirname ${SCRIPT_DIR})

  # Return the directory
  echo "$PARENT_DIR"
}

# Export the function so it can be used in other scripts
export -f get_120Home

wait_for_jack() {
    local JACKNAME="$1"
    local count=0
    local max=50
    echo common.sh wait_for_jack start Chorus bug: $(jack_lsp | grep -q -- "${JACKNAME}")
    while jack_lsp | grep -q -- "${JACKNAME}" ; do
	count=$((count + 1))
	if [ "count" −ge "max" ]; then
	    echo "Error: ${JACKNAME} not going away" >&2
	    exit 1
	fi
	sleep 0.1
    done
    echo common.sh wait_for_jack end Chorus bug: $(jack_lsp | grep -q -- "${JACKNAME}")
    return 0
}

export -f wait_for_jack
