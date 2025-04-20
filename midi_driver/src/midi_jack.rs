//! Read MIDI on standin.
//! Make Jack connections
use crate::jack_connections::JackConnections;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Read;

mod jack_connections;
mod midi_status;
use crate::midi_status::MidiStatus;

/// Associate u8 -> jack connection
/// Jack connetion is two strings: src, dst
#[allow(clippy::type_complexity)]
fn make_table(description: &str) -> Result<(HashMap<u8, (String, String)>, u8), Box<dyn Error>> {
    let mut r1 = HashMap::new();
    let lines: Vec<&str> = description.lines().collect();
    for s in lines.iter() {
        if !s.starts_with("j ") {
            continue;
        }
        // let (src, dst) = s[2..]
	let mut sp = s.split_whitespace();
	let byte = sp.next().ok_or(format!("Invalid jack MIDI definition: {s}"))?;
	let byte = byte.parse::<u8>()?;
	let src = sp.next().ok_or(format!("Invalid jack MIDI definition: {s}"))?;
	let dst = sp.next().ok_or(format!("Invalid jack MIDI definition: {s}"))?;
        r1.insert(byte, (src.to_string(), dst.to_string()));
    }
    let channel = description
        .lines()
        .rev() // If more than one, use last
        .find(|s| s.starts_with("c "))
        .unwrap_or("0");
    let channel: u8 = channel.parse()?;
    Ok((r1, channel))
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut jack_connetions = JackConnections::new("midi_client");
    let cfg_file_name = env::args().nth(1).unwrap();
    let mut s: String = "".to_string();
    let mut file = File::open(&cfg_file_name)
        .unwrap_or_else(|e| panic!("{e:?}: Could not open file: {cfg_file_name}"));
    file.read_to_string(&mut s)
        .expect("Could not read file contents");
    let (command_table, channel): (HashMap<u8, (String,String)>, u8) = make_table(&s)?;

    // Track MIDI status
    let mut status: Option<MidiStatus> = None;

    // Read stdin a byte at a time
    let mut buffer = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock(); // Lock the stdin handle for efficient reading

    // The currently selected effect
    let mut effect:Option<u8> = None;

    loop {
        match handle.read(&mut buffer) {
            Ok(0) =>
            // EOF
            {
                break
            }
            Err(e) => return Err(Box::new(e)),
            Ok(2..) => panic!("Cannot happen"),
            Ok(1) => {
                let byte = buffer[0];
                if byte & 0x80 == 0x80 {
                    // status
                    if byte & 0x0f == channel {
                        // Status byte on this channel:
                        status = MidiStatus::from_byte(byte);
                    } else {
                        status = None;
                    }
                } else {
                    // Data byte
                    if let Some(MidiStatus::ProgramChange(_)) = status.as_ref() {
                        if let Some(command) = command_table.get(&byte) {
			    if let Some(current) = effect {
				let (src, dst) = command_table.get(&current).unwrap();
				jack_connetions.unmake_connection(src,dst)?;
			    }
			    effect = Some(byte);
			    jack_connetions.make_connection(&command.0, &command.1)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
