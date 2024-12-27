#!/usr/bin/bash

set -e
source One20Pedal.sh
Home_DEV=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))

echo " * Get necessary packages"
apt update
apt install -y libjack-dev libasound2-dev libjack-jackd2-dev pkg-config  gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf libasound2-dev gcc-aarch64-linux-gnu  modep-mod-ui gxtuner podman curl

RUSTUP!!
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

echo " * Set up PiSound button "
cat <<EOF  > /usr/local/etc/pisound.conf
DOWN              /usr/local/pisound/scripts/pisound-btn/system/down.sh
UP                /usr/local/pisound/scripts/pisound-btn/system/up.sh

CLICK_1 /usr/local/pisound/scripts/pisound-btn/120Proof-start.sh
CLICK_2 /usr/local/pisound/scripts/pisound-btn/120Proof_stop.sh
CLICK_3 /usr/local/pisound/scripts/pisound-btn/toggle_wifi_hotspot.sh
CLICK_OTHER       /usr/local/pisound/scripts/pisound-btn/do_nothing.sh

CLICK_COUNT_LIMIT 8

HOLD_1S           /usr/local/pisound/scripts/pisound-btn/toggle_bt_discoverable.sh
HOLD_3S           /usr/local/pisound/scripts/pisound-btn/toggle_wifi_hotspot.sh
HOLD_5S           /usr/local/pisound/scripts/pisound-btn/shutdown.sh
HOLD_OTHER        /usr/local/pisound/scripts/pisound-btn/do_nothing.sh
EOF


cat <<EOF > /usr/local/pisound/scripts/pisound-btn/120Proof-start.sh
#!/usr/bin/sh
log "Start 120Proof"
. /home/patch/pedal_start.sh
EOF

chmod a+x /usr/local/pisound/scripts/pisound-btn/120Proof-start.sh

cat <<EOF > /usr/local/pisound/scripts/pisound-btn/120Proof-stop.sh
#!/usr/bin/sh
log "Stop 120Proof"
. /home/patch/pedal_stop.sh
EOF

chmod a+x /usr/local/pisound/scripts/pisound-btn/120Proof-stop.sh

"cat <<EOF4 > /home/patch/pedal_start.sh
#!/bin/sh
${One20PedalHome}/start
EOF4
"
chmod a+x /home/patch/pedal_start.sh

cat <<EOF > /home/patch/pedal_stop.sh
#!/bin/sh
${One20PedalHome}/stop
EOF

chmod a+x /home/patch/pedal_stop.sh

echo " * Set up gxtuner to start on patch login"
mkdir -p /home/patch/.config/systemd/user/
cat <<EOF > /home/patch/.config/systemd/user/gxtuner.service
[Unit]
Description=GxTuner Application
After=graphical.target

[Service]
ExecStartPre=/bin/sleep 10
ExecStart=/home/patch/.config/systemd/user/start_gxtuner
Environment="DISPLAY=:0"
Environment="XAUTHORITY=/home/patch/.Xauthority"
##User=patch
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF



echo " * Install local service for gxtuner"
systemctl --user daemon-reload
systemctl --user enable gxtuner.service
systemctl --user start gxtuner.service

cd ${One20PedalHome}
git clone https://github.com/worikgh/mod-host.git
cd mod-host
make -j `nproc`
cd ${One20PedalHome}

cd midi_driver
cargo build --release
