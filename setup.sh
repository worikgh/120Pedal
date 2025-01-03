#!/bin/bash

set -e
source One20Pedal.sh

echo " * Test if archives available"
ARCHIVE=$(grep  '^deb ' /etc/apt/sources.list | cut -d ' ' -f 2 | head -n 1)
if [ ! -n "$ARCHIVE" ]; then
    echo " * What system is this?  There are no apt sources.  Must run on Debian"
    exit -1
fi

# PACKAGES=("libasound2-dev" "libjack-jackd2-dev" "pkg-config" "gcc-arm-linux-gnueabihf" "g++-arm-linux-gnueabihf" "gcc-aarch64-linux-gnu" "modep-mod-ui" "gxtuner" "podman" "curl" "libcurl4-openssl-dev" "liblilv-dev" "libreadline-dev")
PACKAGES=("libasound2-dev" "libjack-jackd2-dev" "pkg-config" "gcc-arm-linux-gnueabihf" "g++-arm-linux-gnueabihf" "gcc-aarch64-linux-gnu" "modep-mod-ui" "gxtuner" "curl" "libcurl4-openssl-dev" "liblilv-dev" "libreadline-dev")
echo " * Test if all required packages installed"
ALL_INSTALLED=1
for package in "${PACKAGES[@]}"; do
    if [[ ! $(dpkg -l | grep -qw "$package") ]]; then
	ALL_INSTALLED=0
	break
    fi
done

if [[ $ALL_INSTALLED -eq 0 ]] ; then
    echo " * Install missing packages"
    if curl  -I  ${ARCHIVE} 2>/dev/null | grep "200 OK" >/dev/null; then
	sudo apt update -y
	sudo apt install -y  ${PACKAGES}
    else
	echo " ** Some packages missing and cannot reach archive **"
	exit -1
    fi
fi

if curl  -I  ${ARCHIVE} >/dev/null | grep "200 OK" >/dev/null; then
    echo " * Check if any packages need to be updated"
    if [ $(apt list --upgradable 2>/dev/null|wc -l) != 0 ] ; then
	echo " * Upgrade packages"
	sudo apt update -y
	sudo apt full-upgrade -y
    fi
fi

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
cat <<EOF > $SYSTEMD_DIR/start_gxtuner
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
if [ ! -d mod-host ] ; then
    git clone https://github.com/worikgh/mod-host.git
    cd mod-host
else
    cd mod-host
    git pull
fi

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
MIDI_DRIVER_LINK=${One20PedalHome}/midi_driver.exe
MIDI_DRIVER=${One20PedalHome}/midi_driver/target/release/midi_driver
if [ ! -x ${MIDI_DRIVER} ] ; then
    echo " ** Failed to create executable: ${MIDI_DRIVER}"
    exit -1
fi

echo " * Create link to midi-driver"
if [ -e ${MIDI_DRIVER_LINK} ] ; then
    if [ ! -L ${MIDI_DRIVER_LINK} -o $(readlink -f ${MIDI_DRIVER_LINK}) != ${MIDI_DRIVER} ] ; then
	rm ${MIDI_DRIVER_LINK}
    fi
fi
if [ ! -e ${MIDI_DRIVER_LINK} ] ; then
    ln -s ${MIDI_DRIVER} ${MIDI_DRIVER_LINK}
fi


echo " * Finished"

