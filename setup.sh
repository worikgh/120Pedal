#!/usr/bin/bash

set -e
source One20Pedal.sh
Home_DEV=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))

echo " * Get necessary packages"
sudo apt update
echo " * Upgrade all packages"
sudo apt full-upgrade -y
sudo apt install -y  libasound2-dev libjack-jackd2-dev pkg-config  gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf libasound2-dev gcc-aarch64-linux-gnu  modep-mod-ui gxtuner podman curl libcurl4-openssl-dev liblilv-dev libreadline-dev
# sudo apt install -y libjack-dev

echo " * Set up PiSound button "
sudo cp system_files/pisound.conf /usr/local/etc/pisound.conf
sudo cp system_files/120Proof-start.sh /usr/local/pisound/scripts/pisound-btn/120Proof-start.sh
sudo chmod a+x /usr/local/pisound/scripts/pisound-btn/120Proof-start.sh
sudo cp system_files/120Proof-stop.sh /usr/local/pisound/scripts/pisound-btn/120Proof-stop.sh
sudo chmod a+x /usr/local/pisound/scripts/pisound-btn/120Proof-stop.sh
cat <<EOF4 > /home/patch/pedal_start.sh
#!/bin/sh
${One20PedalHome}/start
EOF4
chmod a+x /home/patch/pedal_start.sh
cat <<EOF > /home/patch/pedal_stop.sh
#!/bin/sh
${One20PedalHome}/stop
EOF
chmod a+x /home/patch/pedal_stop.sh

echo " * Set up gxtuner to start on patch login"
SYSTEMD_DIR=/home/patch/.config/systemd/user
mkdir -p $SYSTEMD_DIR
if [ -e $SYSTEMD_DIR/gxtuner.service ] ; then
    rm $SYSTEMD_DIR/gxtuner.service
fi
cat <<EOF > $SYSTEMD_DIR/gxtuner.service
[Unit]
Description=GxTuner Application
After=graphical.target

[Service]
ExecStartPre=/bin/sleep 10
ExecStart=$SYSTEMD_DIR/start_gxtuner
Environment="DISPLAY=:0"
Environment="XAUTHORITY=/home/patch/.Xauthority"
##User=patch
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF

chmod a+x $SYSTEMD_DIR/gxtuner.service

echo " * Start script gxturner"
echo <<EOF > $SYSTEMD_DIR/start_gxtuner
#!/bin/bash
/usr/bin/gxtuner
EOF
chmod a+x $SYSTEMD_DIR/start_gxtuner

echo " * Install local service for gxtuner"
systemctl --user daemon-reload
systemctl --user enable gxtuner.service
systemctl --user start gxtuner.service

echo " * Fetch mod-host source"

cd ${One20PedalHome}
git clone https://github.com/worikgh/mod-host.git
cd mod-host
echo " * Make mod-host"
make -j `nproc`
cd ${One20PedalHome}

#RUSTUP!!
echo " * Update Rust"
curl https://sh.rustup.rs -sSf | sh -s -- -y
source $HOME/.profile
cd ${One20PedalHome}
cd midi_driver

echo " * Build midi-driver"
cargo build --release

echo " * Finished"

