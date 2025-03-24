# Process MIDI

Consists of three programmes:
1. `read_midi`
2. `translate_midi`
3. `command_midi`

## Read MIDI

Open a MIDI device and write to `stdout` all MIDI triples

One argument: The name of the MIDI input port.  This does not have to be the full name.  The first port where the passed name is a part of the port's name will be used.

## Translate MIDI

Read MIDI triples on `stdin`, transform the MIDI using a translation table, and write the transformed MIDI triples to `stdout`

## Command MIDI

Read MIDI triples on `stdin` and run commands in response.
