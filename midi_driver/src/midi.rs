//!  Handle the MIDI connections
use std::error;
// use std::fmt;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
#[derive(Default)]
#[allow(dead_code)]
pub struct MidiData {
    pub connection_cache: Vec<(String, String)>,
    pub last: u8,
}
// #[derive(Debug, Clone)]
// struct MidiError {
//     what: String,
// }

// impl fmt::Display for MidiError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "120Proof MIDI Error: {}", self.what)
//     }
// }
// impl error::Error for MidiError {}
#[allow(dead_code)]
pub struct Midi {
    pub name: String,
    translate_table: HashMap<u8, u8>,
}

#[allow(dead_code)]
impl Midi {
    pub fn new(
        name: String,
        translate_table: HashMap<u8, u8>,
    ) -> Result<Self, Box<dyn error::Error>> {
        Ok(Midi {
            name,
            translate_table,
        })
    }

    pub fn run(
        &self,
        mut f: impl FnMut(&[u8], &mut MidiData) + Send + 'static,
    ) -> Result<(), Box<dyn error::Error>> {

        // TODO: Should allow name to be controlled fom command line.
        // May be more than one pedal in use.
        let this_name = "120Pedal".to_string();
        let midi_in = midir::MidiInput::new(this_name.as_str())?;
        for (index, port) in midi_in.ports().iter().enumerate() {
            // Each available input port.
            match midi_in.port_name(port) {
                Err(_) => continue,
                Ok(port_name) => {
                    eprintln!("DEBUGGING: port_name: {port_name}");
                    if port_name.as_str().contains(self.name.as_str()) {
                        // Found the port (first port that `card_name`
                        // is a subset of)

                        let this_port = midi_in
                            .ports()
                            .get(index)
                            .ok_or("Invalid port number")
                            .unwrap()
                            .clone();

                        let translate_table = self.translate_table.clone();
                        let connect = midi_in.connect(
                            &this_port,
                            format!("{}-in", this_name).as_str(),
                            move |_a, b, connection_cache| {
                                let c = match translate_table.get(&b[1]) {
                                    Some(&d) => d,
                                    None => b[1],
                                };
                                println!("MIDI in {:?}/{c}", &b);

                                f(&[192, c], connection_cache);
                            },
                            MidiData::default(),
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

        Ok(())
    }
}
