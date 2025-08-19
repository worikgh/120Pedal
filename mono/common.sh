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
