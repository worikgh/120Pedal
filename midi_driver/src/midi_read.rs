use midir::MidiIO;
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
//use midir::MidiInput;
use midir::MidiInputPort;
mod midi;

const THIS_MIDI_NAME: &str = "120Pedal";

fn main() -> Result<(), Box<dyn Error>> {
    // The name of the MIDI port.  The first port found that contains
    // this string will be used
    let name = env::args().nth(1).unwrap();

    // Create the port for MIDI input
    let this_name = THIS_MIDI_NAME.to_string();
    let midi_in = midir::MidiInput::new(THIS_MIDI_NAME)?;

    let this_port = get_midi_port(&name, &midi_in)?;

    let connect = midi_in.connect(
        &this_port,
        format!("{}-in", this_name).as_str(),
        move |_a, b, _| {
            // The meat of this programme.  Simply write all data from
            // MIDI to stdout
            io::stdout()
                .write_all(b)
                .unwrap_or_else(|e| panic!("Cannot write to stdout: {}", e));
            io::stdout().flush().expect("Failed to flush stdout");
        },
        (),
    );
    match connect {
        Ok(_) => {
            eprintln!("Created MIDI in");
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
        Err(err) => Err(format!("Could not connect: {:?}", err).into()),
    }
}

/// Get the first MDII port that has `name` as part of its name
fn get_midi_port<T>(name: &str, midi_in: &T) -> Result<MidiInputPort, Box<dyn Error>>
where
    T: MidiIO<Port = MidiInputPort>,
{
    midi_in
        .ports()
        .iter()
        .find(|&port| {
            midi_in
                .port_name(port)
                .map(|port_name| port_name.contains(name))
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("No MIDI port found containing '{}'", name).into())
        .cloned()
}
