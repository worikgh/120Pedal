#!/bin/bash

## Stop hotspot
systemctl stop dnsmasq

# Reset wlan0 to managed mode

systemctl stop dnsmasq
ip link set wlan0 down
iw dev wlan0 set type managed
ip link set wlan0 up
systemctl restart wpa_supplicant@wlan0
systemctl restart systemd-networkd
systemctl restart systemd-networkd
echo "Wi-Fi client mode enabled (Connecting to Pauline)"
