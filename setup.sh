#!/usr/bin/bash

set -e
source One20Pedal.sh
Home_DEV=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))

echo Create a 120Pedal boot image for Raspberry Pi

echo " * Load packages needed to build the software"
# cargo install cross
# sudo dpkg --add-architecture armhf 
# sudo apt update
# sudo apt install -y libjack-dev libasound2-dev libjack-jackd2-dev:arm64 pkg-config  gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf libasound2-dev:armhf gcc-aarch64-linux-gnu podman

## This should be passed on the command line
IMG_FN=$1
if [ -z ${IMG_FN} ] ; then
    echo Pass Patchbox OS image path as argument
    exit 1
fi

if [ ! -r $IMG_FN ] ; then
    echo "${IMG_FN} is not readable"
    exit 1
fi

## Files that are necessary to run the Pedal
ONE20_FILES=(
    start
    stop
    One20Pedal.sh
    setPedalBoardCommon.pl
    getPedalBoardCommands.pl
    setPedalBoardCommon.sh
    getPedalBoardCommands.sh
    initialise_pi.sh
)

echo " * Mount ${IMG_FN} on mnt"
mkdir -p $Home_DEV/mnt
sudo losetup -fP $IMG_FN
DEVICE=$(losetup -a |grep $IMG_FN|cut -d ' ' -f 1|cut -d ':' -f 1)
echo " * mount: $DEVICE"
sudo mount ${DEVICE}p2 mnt
echo ${IMG_FN} on $DEVICE

## Prepare source for pedal `midi_driver`
rsync -a --exclude target/  --exclude '.git/' --exclude '*.rs.bk' midi_driver mnt/home/patch mnt${One20PedalHome}

echo " * Copy the files"
for FILE in "${ONE20_FILES[@]}";do
    if [ ! -r ${Home_DEV}/${FILE} ] ; then
	echo ${FILE} does not exist
	exit -1
    fi
    cp -v ${Home_DEV}/${FILE}  mnt${One20PedalHome}
done

echo " * Set up PiSound button "
sudo bash -c 'cat <<EOF  > mnt/usr/local/etc/pisound.conf
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
'

sudo bash -c 'cat <<EOF > mnt/usr/local/pisound/scripts/pisound-btn/120Proof-start.sh
#!/usr/bin/sh
log "Start 120Proof"
. /home/patch/pedal_start.sh
EOF
'
sudo chmod a+x mnt/usr/local/pisound/scripts/pisound-btn/120Proof-start.sh

sudo bash -c 'cat <<EOF > mnt/usr/local/pisound/scripts/pisound-btn/120Proof-stop.sh
#!/usr/bin/sh
log "Stop 120Proof"
. /home/patch/pedal_stop.sh
EOF
'
sudo chmod a+x mnt/usr/local/pisound/scripts/pisound-btn/120Proof-stop.sh

sudo bash -c "cat <<EOF4 > mnt/home/patch/pedal_start.sh
#!/bin/sh
${One20PedalHome}/start
EOF4
"
sudo chmod a+x mnt/home/patch/pedal_start.sh

sudo bash -c 'cat <<EOF > mnt/home/patch/pedal_stop.sh
#!/bin/sh
${One20PedalHome}/stop
EOF
'
sudo chmod a+x mnt/home/patch/pedal_stop.sh

echo " * Set up gxtuner to start on patch login"
mkdir -p mnt/home/patch/.config/systemd/user/
sudo bash -c 'cat <<EOF > mnt/home/patch/.config/systemd/user/gxtuner.service
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
'

echo " * Prepared image: Unmounting"
sudo umount mnt
sudo losetup -d $DEVICE

echo " * Finished. Image in ${IMG_FN}"
