#!/bin/bash

## Stop Wi-Fi client services
systemctl stop wpa_supplicant@wlan0
systemctl stop systemd-networkd

## Configure wlan0 as access point
ip link set wlan0 down
iw dev wlan0 set type ap
ip link set wlan0 up

## Set static IP for the hotspot
ip addr add 192.168.4.1/24 dev wlan0

## Configure and start dnsmasq
cat > /etc/dnsmasq.conf <<EOF
interface=wlan0
dhcp-range=192.168.4.2,192.168.4.20,255.255.255.0,24h
domain=local
address=/hotspot.local/192.168.4.1
EOF

systemctl restart dnsmasq

## Enable NAT (optional, only needed if you want internet sharing later)
# iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
# iptables -A FORWARD -i wlan0 -o eth0 -j ACCEPT

## Start hostapd (create config if needed)
if [ ! -f /etc/hostapd/hostapd.conf ]; then
    cat > /etc/hostapd/hostapd.conf <<EOF
interface=wlan0
driver=nl80211
ssid=RPiHotspot
hw_mode=g
channel=6
wmm_enabled=0
macaddr_acl=0
auth_algs=1
ignore_broadcast_ssid=0
wpa=2
wpa_passphrase=raspberry
wpa_key_mgmt=WPA-PSK
wpa_pairwise=TKIP
rsn_pairwise=CCMP
EOF
fi

systemctl unmask hostapd
systemctl enable hostapd
systemctl restart hostapd

echo "Hotspot enabled - SSID: RPiHotspot, Password: raspberry, IP: 192.168.4.1"
