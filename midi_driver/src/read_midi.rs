use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
mod midi;

fn main() -> Result<(), Box<dyn Error>> {
    let name = env::args().nth(1).unwrap();

    let this_name = "120Pedal".to_string();
    let midi_in = midir::MidiInput::new(this_name.as_str())?;
    for (index, port) in midi_in.ports().iter().enumerate() {
        // Each available input port.
        match midi_in.port_name(port) {
            Err(_) => continue,
            Ok(port_name) => {
                eprintln!("DEBUGGING: port_name: {port_name}");
                if port_name.as_str().contains(name.as_str()) {
                    // Found the port (first port that `card_name`
                    // is a subset of)

                    let this_port = midi_in
                        .ports()
                        .get(index)
                        .ok_or("Invalid port number")
                        .unwrap()
                        .clone();

                    let connect = midi_in.connect(
                        &this_port,
                        format!("{}-in", this_name).as_str(),
                        move |_a, b, _| {
                            io::stdout()
                                .write_all(b)
                                .unwrap_or_else(|e| panic!("Cannot write to stdout: {}", e));
                            eprintln!("MIDI in {:?}", &b);
                        },
                        (),
                    );
                    match connect {
                        Ok(_) => {
                            println!("Created MIDI in");
                            loop {
                                thread::sleep(Duration::from_secs(1));
                            }
                        }
                        Err(err) => {
                            println!("Could not connect {:?}", err);
                        }
                    };
                    break;
                }
            }
        }
    }

    let mut coin = false;
    loop {
        if coin {
            eprint!("\rtick");
        } else {
            eprint!("\rtock");
        }
        coin = !coin;
        thread::sleep(Duration::from_secs(1));
    }
}
