#!/usr/bin/bash

set -e

## Initialise the software that 120Pedal needs

source One20Pedal.sh

echo " * Get necessary packages"
sudo apt update
sudo apt install modep-mod-ui gxtuner -y

echo " * Install local service for gxtuner"
systemctl --user daemon-reload
systemctl --user enable gxtuner.service
systemctl --user start gxtuner.service

echo " * Set up the midi_driver for the pedal"
cd ${One20PedalHome}
git clone https://github.com/worikgh/mod-host.git
cd mod-host
make -j `nproc`
cd ${One20PedalHome}

cd midi_driver
cargo build --release


