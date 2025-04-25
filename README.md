![under construction](under-construction.png)

**THIS BARELY WORKS**

# Guitar Pedal

### Set up the machine:

This has been built and tested primarily on Raspberry Pi 4 and 5 SBCs.

* Using Debian 12
*  Required packages:
  * jackd2
  * libjack-jackd2-dev
  * lv2-dev
  * libreadline-dev
  * liblilv-dev
  * git
  * libasound2-dev
  * pkg-config
  * python3.11-dev
* Install rust
  * `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### Configure Jack

Get the user name and the name of the sound card in use.  E.g: puppy using a Scarlett Solo:

```sh
aplay -l |grep  Scarl
card 3: Gen [Scarlett Solo 4th Gen], device 0: USB Audio [USB Audio]
```
Edit `/etc/systemd/system/jackd.service` to start Jack
```
[Unit]
Description=JACK Audio Connection Kit
After=sound.target

[Service]
ExecStart=/bin/sh -c 'CARD=$(aplay -l | grep -m1 "Scarlett Solo" | cut -d" " -f2 | tr -d ":"); exec /usr/bin/jackd -d alsa -d hw:$CARD -r 48000 -p 128 -n 2'
Restart=always
User=puppy
Group=audio
LimitMEMLOCK=8589934592
LimitRTPRIO=89
Environment="XDG_RUNTIME_DIR=/run/user/$(id -u puppy)"
Environment="JACK_NO_AUDIO_RESERVATION=1"

[Install]
WantedBy=multi-user.target
```
* `-d alsa` Alsa backend
* `-r 48000` Sample rate of 48kHz
* `-p 128` Buffer size

Run:
```sh
sudo systemctl start jackd
sudo systemctl enable jackd
```

### Setup Mod-host

[LV2](https:lv2plug.in) is a set of royalty-free open standards[2] for music production plug-ins and is very useful.  This pedal can be used without it, but using LV2 is a good idea.

* Clone `https://github.com/worikgh/mod-host.git`
  * `cd mod-host`
  * `make`
  * `./mod-host -n -p 5555 `

### Build Simulators

* This system switches audio jack signals in real time (sub 100ms)
  * A MIDI foot pedal can simulate guitar pedals

Any jack based audio processing can be used, so long as it can read and write audio from and to jack.  However a very convenient way is to use [mod-ui](https://github.com/worikgh/mod-ui.git).  It is important to use that repository, not the official one (as of 2025-04-19) as `mod-host` requires Jackd from Debian 12 and `mod-ui` from the official repository will not build with Python 3.10, which is the default on Debian-12.

The installation instructions in [this mod-ui](https://github.com/worikgh/mod-ui.git) include code to get around that.

* So clone https://github.com/worikgh/mod-ui.git
* `cd mod-ui`
* Follow the instructions in the `README.md` which are:
```
python3 -m venv myenv
source myenv/bin/activate
pip3 install -r requirements.txt
if [ -e myenv/lib/python3.10/site-packages/tornado/httputil.py ]; then
	echo "  * Update 3.10: "
	sed -i -e 's/collections.MutableMapping/collections.abc.MutableMapping/' myenv/lib/python3.10/site-packages/tornado/httputil.py
elif [ -e myenv/lib/python3.11/site-packages/tornado/httputil.py ]; then
	echo "  * Update 3.11: "
	sed -i -e 's/collections.MutableMapping/collections.abc.MutableMapping/' myenv/lib/python3.11/site-packages/tornado/httputil.py
elif [ -e myenv/lib/python3.12/site-packages/tornado/httputil.py ]; then
	echo "  * Update 3.12: "
	sed -i -e 's/collections.MutableMapping/collections.abc.MutableMapping/' myenv/lib/python3.12/site-packages/tornado/httputil.py
	sed -i -e 's/import ssl/import _NOT_ssl/' myenv/lib/python3.12/site-packages/tornado/netutil.py
fi
make -C utils
export MOD_DEV_ENVIRONMENT=0
python3 ./server.py
```

Then use a web browser to connect to port 8888 `http://<IP of PI>:8888` for the `mod-ui` interface.  It is possible to make use of LV2 simulators, with a nice user interface: [mod-ui_ss.png] `mod-ui` will look for LV2 plugins in `~/.lv2/`.

### Set up Pedals

* Clone the [120Pedal](https://github.com/worikgh/120Pedal.git) repository
* `cd 120Pedal`
* If using LV2 simulators and `mod-ui`
  * `./getLV2param`
    * This reads the pedals as set up by `mod-ui`
	* Alternatively if `mod-ui` run on a different computer, copy the LV2 definitions to `~/.lv2` and the `PEDALS/` directory to `120Pedal/PEDALS`
  * `./setLV2`
    * This sets up the LV2 simulators.  It connects them into pedal boards (named in the `PEDALS/` directory) and makes the Jack connections between them.
	* It makes no connections to the jack ports: `system_capture_*` and `system_playback_*`

### Pedal Driver

1. `read_midi` Passed a device name it reads MIDI fro that device and outputs it on its standard output
2. `translate_midi` Reads MIDI from the standard in, translates the MIDI according to instructions from a configuration file, outputs MIDI on its standard out
3. `jack_midi`  Reads MIDI from its standard in and creates and destroys Jack connections

Example SINCO MIDI Pedal
---

[SINCO.png]

* `read_midi SINCO` will open the pedal and send MIDI to its standard output.  The first MIDI device where the name is a super string of the argument ("SINCO" in this case) is chosen as the pedal device

* `translate_midi examples/sinco.cfg` The configuration file [sinco.cfg](midi_driver/examples/sinco.cfg) configures `translate_midi` so:
  * Programme change MIDI messages are on channel 0 because a channel is not specified and that is the default
  * Only programme change values output are:  0, 1, 2 or 3
  * The SINCO pedal has eight modes and can output 32 MIDI values.
    * Modes are changed by depressing two buttons together
	* The buttons are small and close together, it is easy to depress two together by mistake
    * The translation table ensure that button 'A' outputs 0, button 'B' outputs 1,  button 'C' outputs 2 and button 'D' outputs 3,  in all modes.

* `jack_midi examples/midi_jack.cfg`The configuration file:
```plaintext
j 0 PEADLS/Big_Muff
j 1 PEADLS/Chaos
j 2 PEADLS/GXepicvalve
j 3 PEADLS/lost_world
```
  * Button `a` will make the connections in `PEDALS/Big_Muff`
  * Button `b` will make the connections in `PEDALS/Chaos`
  * Button `c` will make the connections in `PEDALS/GXepicvalve`
  * Button `d` will make the connections in `PEDALS/lost_world`

The files have contents like:
```sh
$ cat PEDALS/lost_world 
system:capture_1 effect_14:in
effect_13:Out1 system:playback_1
```

Where `effect_14` and `effect_13` are LV2 simulators.  There will be Jack connections between them set up by `setLV2`

`120Pedal/midi_driver $ (export PATH=$PATH:$(pwd)/target/release; read_midi SINC | translate_midi examples/sinco.cfg | jack_midi examples/sas_house.cfg)`
	
