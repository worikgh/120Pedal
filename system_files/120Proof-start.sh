#!/bin/bash
sudo -u patch bash << EOF
echo "120Proof-start.sh Now running as: $(whoami)"
. /home/patch/pedal_start.sh
EOF
