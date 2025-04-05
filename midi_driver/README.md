# Process MIDI

Process MIDI using three types of object

1. Producers.  Here `read_midi`
2. Translators. Here `translate_midi`
3. Consumers. Here `command_midi`


## Producer: Read MIDI

src: `src/read.rs`
bin: `read_midi`

Producers open a MIDI device and write MIDI to `stdout`

Arguments:
	* `--list` List the MIDI ports that can be connected to then exit
	* The name of the MIDI input port.  This does not have to be the full name.  The first port where the passed name is a part of the port's name will be used.

## Translator: Translate MIDI
src: `src/translate.rs`
bin: `translate_midi`

Translators read MIDI on `stdin` and write MIDI on `stdout`.

For noteon and noteoff messages the note is translated using a lookup table, and the transformed noteon or noteoff message is written to  `stdout`

All other MIDI messages are passed through.

`translate_midi` takes one argument: The name of a configuration file.

### Configuration File

The configuration file consists of lines of the form: "t n m"
* `t` the character 't'
* `n` A MIDI note to translate, an integer in 0..127
* `m` A MIDI note to output, if `n` received, in 0..127

Any line starting with "t "" must be of this form otherwise `translate_midi` will panic.

All other lines are ignored

## Consumer: Command MIDI

Consumers read on `stdin` and affect the world.  `command_midi` runs  commands in response to noteon messages. A future plan is to kill the commands in response to noteoff messages

`command_midi` takes one argument: A configuration file name

### Configuration File

The configuration file consists of two sorts of lines:

1. lines of the form: "x n s"
  * `x` the character 'x'
  * `n` A MIDI note to translate, an integer in 0..127
  * `s` A command to run if the noteon message containing `n` is received.
    * `s` must be a executable command
    * No arguments are provided for
2. one or more lines of the form: "c n"
  * `c` the character 'c'
  * `n` the channel to monitor
    * If defined more than once the last definition is used
	* If missing defaults to 0
