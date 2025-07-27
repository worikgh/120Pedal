# 120Pedal - MIDI Guitar Pedal Controller 🎸

![under construction](under-construction.png)

**THIS BARELY WORKS**

A system to control simulated guitar pedals using a MIDI foot controller. Designed primarily for Raspberry Pi 4/5, it allows real-time switching of audio effects via Jack audio connections.

![GUI Screenshot](GUI.png)

## Key Features
- Real-time audio routing (<100ms latency)
- Supports any  effects simulator with Jack audio I/O
  - Currently only mono effects are fully supported
- LV2 plugin integration via `mod-host` and `mod-ui`
- MIDI controller configuration
- GUI for setting audio levels of each effect
  - Includes  a mute switch

## System Requirements
- [Raspberry Pi](https://www.raspberrypi.com/) 4 or 5 (recommended)
  - This will probably run an any computer using Linux.  It was developed for and on a Raspberry Pi
- Debian 12 ([Patchbox OS](https://blokas.io/patchbox-os/))
  - Patchbox OS comes with the unessential but useful [Modep](https://blokas.io/modep/) software preinstalled.
- Compatible audio interface
- MIDI foot controller

## Installation

### Recommended OS: Patchbox OS
1. Download [Patchbox OS](https://blokas.io/patchbox-os/)
2. Install with these settings:
   - Select no additional modules during installation
   - Configure your audio interface settings
3. Post-installation:
```bash
sudo apt install modep-mod-ui git curl build-essential
sudo systemctl disable modep-mod-ui  # Prevent mod-ui from auto-starting
```

Optional: Remove Telemetry
---

Patchbox OS includes opt-out telemetry. To remove:
```bash
sudo apt purge blokas-telemetry
```
Optional: Remove Deskotop Panels
---

The LXDE desktop, default to PachOS, takes up screen room for no purpose and can be disabled.

1. Edit the file `/etc/xdg/lxsession/LXDE/desktop.conf `
  a. Remove the `launcher_manager/command=lxpanelctl` line from the [Session] section if it is there
  b. Remove both the `sNet/IconThemeName=PiXflat` and `sGtk/CursorThemeName=PiXflat` lines from the [GTK] section
2. Edit `~/.config/lxsession/LXDE/autostart`
  a. Remove `@lxpanel` line to disable default panel
  b. Remove `@pcmanfm`  line to disable desktop icons

### Other distributions

To run LV2 simulators an LV2 host is required.  The host [`mod-host`](https://github.com/mod-audio/mod-host) is recommended.

```
git clone https://github.com/mod-audio/mod-host
cd mod-host
make
```

It is possible to use Modep, and in particular, `mod-ui`, an Debian-12, but it is not trivial.  Use [this](https://github.com/worikgh/mod-ui/tree/raspberrypi-bookworm) and follow the instructions in the `README.md`.

`mod-ui` will not play nicely with [Patchstorage](https://patchstorage.com/)  and it will not display the nice PNG images of pedals like it will if you install from Patchbox OS, but it is still very useful

### Software Setup

1. Install required packages:

```bash
sudo apt install dnsmasq git hostapd iw jackd2 libasound2-dev \
libjack-jackd2-dev liblilv-dev libreadline-dev libsdl2-dev \
libsdl2-image-dev lv2-dev pkg-config python3.11-dev -y
```

2. Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Configuration

### 3.5" Screen Driver

```sh
git clone https://github.com/goodtft/LCD-show.git
cd LCD-show
sudo ./LCD35-show
```

### Auotlogin for `patch`

Ensure `patch` is in group `lightdm`: `sudo usermod -aG lightdm patch`
Ensure the `autologin-session` parameter in `/etc/lightdm/lightdm.conf` exists in `/usr/share/xsessions/` (E.g: `/usr/share/xsessions/LXDE.desktop` so `autologin-session=LXDE`)

`sudo systemctl restart lightdm`


### Auto-starting the GUI
```bash
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/120pedal.desktop <<EOF
[Desktop Entry]
Type=Application
Name=120Pedal
Exec=/bin/bash -c "sleep 5 && \${HOME}/120Pedal/gui/run.sh"
Comment=120Pedal Controller
X-GNOME-Autostart-enabled=true
X-GNOME-Autostart-Delay=10
EOF
chmod +x ~/.config/autostart/120pedal.desktop
```

### Jack Audio Setup

If not using Patchbox OS (that takes care of this):

1. Identify your audio interface:
```bash
aplay -l | grep "Your Interface Name"
```

2. Create `/etc/systemd/system/jackd.service`:
```ini
[Unit]
Description=JACK Audio Connection Kit
After=sound.target

[Service]
ExecStart=/bin/sh -c 'CARD=$(aplay -l | grep -m1 "Your Interface" | cut -d" " -f2 | tr -d ":"); exec /usr/bin/jackd -d alsa -d hw:$CARD -r 48000 -p 128 -n 2'
Restart=always
User=your_username
Group=audio
LimitMEMLOCK=8589934592
LimitRTPRIO=89
Environment="XDG_RUNTIME_DIR=/run/user/$(id -u your_username)"
Environment="JACK_NO_AUDIO_RESERVATION=1"

[Install]
WantedBy=multi-user.target
```

3. Enable Jack:
```bash
sudo systemctl start jackd
sudo systemctl enable jackd
```

### mod-host Setup

If not using Patchbox OS:

```bash
cd
git clone https://github.com/worikgh/mod-host.git
cd mod-host
make
./mod-host -n -p 5555
```

### mod-ui Installation

If not using Patchbox OS


```bash
cd
git clone https://github.com/worikgh/mod-ui.git
cd mod-ui
python3 -m venv myenv
source myenv/bin/activate
pip3 install -r requirements.txt

# Apply necessary patches
find myenv/lib/python* -name httputil.py | xargs sed -i 's/collections.MutableMapping/collections.abc.MutableMapping/'
make -C utils
export MOD_DEV_ENVIRONMENT=0
python3 ./server.py
```

Access the interface at `http://<your-pi-ip>:8888`  (**Not HTTPS**)

If using Pathbox OS access the interface at `http://<your-pi-ip>` (**Not HTTPS**)

## Setting Up Pedals

1. Clone the repository:
```bash
cd
git clone https://github.com/worikgh/120Pedal.git --recurse-submodules
cd 120Pedal/gui 
cargo build --release
cd ../midi_driver 
cargo build --release
```

2. Configure your pedal setups in the `PEDALS/` directory (see [PEDALS/README.md](PEDALS/README.md))

3. For LV2 simulators:
```bash
./getLV2  # Reads mod-ui pedal configurations
./setLV2  # Sets up LV2 simulators and Jack connections
```

**Error: `ERROR qzn3t_mixer:output_1 not a valid port`**
**Must mke some pdals**
**TODO: Making pedals documentation**

## MIDI Pedal Configuration

The MIDI pedal is driven with three components:

1. A reader: `read_midi` that connects to the device and outputs the MIDI data on its STDOUT
2. A translator: `translate_midi` that reads MIDI on its STDIN and writes (translated) MIDI on its STDOUT
3. An actor: `jack_midi` that reads MIDI on its STDIN and sets up Jack audio pipes

### Example: SINCO MIDI Pedal
![SINCO pedal](SINCO.png)

1. **Read MIDI Input**:
```bash
read_midi SINCO
```

2. **Translate MIDI Commands**:
```bash
translate_midi examples/sinco.cfg
```

3. **Control Jack Connections**:
```bash
jack_midi examples/midi_jack.cfg
```

### Pipeline Example
```bash
(export PATH=$PATH:$(pwd)/midi_driver/target/release
read_midi SINCO | translate_midi examples/sinco.cfg | jack_midi examples/sas_house.cfg)
```

## Pedal Configuration Files
Example pedal definition (`PEDALS/lost_world`):
```
system:capture_1 effect_14:in
effect_13:Out1 system:playback_1
```

## Troubleshooting
- Ensure Jack is running before starting mod-host
- Verify your audio interface is properly detected
- Check MIDI device permissions
