//! Reads a stream of MIDI data fro mthe stdin
//! Writes the data on the stdout with Note On and Note Off notes transposed
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
mod midi_status;
use crate::midi_status::MidiStatus;
/// Build the table to translate MIDI inputs.
/// Transposes "Note On" and "Note Off" notes
fn make_table(description: &str) -> Result<HashMap<u8, u8>, Box<dyn Error>> {
    // let mut result = HashMap::new();
    // return result;
    Ok(description
        .split("\n")
        .collect::<Vec<&str>>()
        .iter()
        .filter(|&s| s.len() > 1 && &s[0..2] == "t ")
        .map(|&s| {
            let parts: Vec<&str> = s[2..].split_whitespace().collect();
            if parts.len() != 2 {
                panic!("Invalid translation line: {parts:?}");
            }
            let k = parts[0]
                .parse::<u8>()
                .unwrap_or_else(|e| panic!("{e:?}: Invalid key: {}", parts[0]));
            let v = parts[1]
                .parse::<u8>()
                .unwrap_or_else(|e| panic!("{e:?}: Invalid value: {}", parts[1]));
            (k, v)
        })
        .collect())
}
fn main() -> Result<(), Box<dyn Error>> {
    let cfg_file_name = env::args().nth(1).unwrap();
    // The contents of the configuration file as a `String`
    let mut s: String = "".to_string();
    let mut file = File::open(&cfg_file_name)
        .unwrap_or_else(|e| panic!("{e:?}: Could not open file: {cfg_file_name}"));
    file.read_to_string(&mut s)
        .expect("Could not read file contents");
    let translation_table: HashMap<u8, u8> = make_table(&s)?;
    let mut status: Option<MidiStatus> = None;
    // Read stdin a byte at a time
    let mut buffer = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock(); // Lock the stdin handle for efficient reading

    // Working memory for processing MIDI messages
    let mut working: Vec<u8> = Vec::new();
    let write_working = |w: &Vec<u8>| {
        io::stdout()
            .write_all(w)
            .unwrap_or_else(|e| panic!("Cannot write to stdout: {}", e))
    };

    loop {
        match handle.read(&mut buffer) {
            Ok(0) => {
                // EOF
                write_working(&working);
                break;
            }
            Err(e) => return Err(Box::new(e)),
            Ok(_) => {
                let byte = buffer[0];
                if byte & 0x80 == 0x80 {
                    // Status byte:
                    status = MidiStatus::from_byte(byte);
                    if !working.is_empty() {
                        write_working(&working);
                        working.truncate(0);
                    }
                    working.push(byte);
                    continue;
                } else {
                    match status.as_ref() {
                        Some(MidiStatus::NoteOn(_)) | Some(MidiStatus::NoteOff(_)) => {
                            match working.len() % 2 {
                                // If there are an odd number of data in
                                // `working` this is a note and must be
                                // tranlated, if not it is velocity
                                1 => working.push(*translation_table.get(&byte).unwrap()),
                                // Velocity.
                                0 => working.push(byte),
                                _ => unreachable!("Will not happen"),
                            }
                        }
                        // All other bytes just pass through
                        _ => working.push(byte),
                    };
                }
            }
        };
    }
    Ok(())
}
