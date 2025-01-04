#!/bin/bash

set -e
DIR=$(dirname $0)
source "${DIR}/One20Pedal.sh"
perl "${DIR}/getPedalBoardCommands.pl"
