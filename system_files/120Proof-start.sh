#!/usr/bin/sh
sudo -u patch bash << EOF
echo "Now running as patch"
. /home/patch/pedal_start.sh
EOF
