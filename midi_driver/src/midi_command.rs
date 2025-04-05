//! Read MIDI signals tripples on STDIN
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Read;
mod midi_status;
use crate::midi_status::MidiStatus;
fn run_command(_command: &str) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn make_table(description: &str) -> Result<(HashMap<u8, String>, u8), Box<dyn Error>> {
    let mut r1 = HashMap::new();
    let lines: Vec<&str> = description.lines().collect();
    for s in lines.iter() {
        if !s.starts_with("x ") {
            continue;
        }
        let (byte, command) = s[2..]
            .split_once(' ')
            .ok_or(format!("Line '{}' has invalid format", s))?;
        let byte: u8 = byte.parse()?;
        r1.insert(byte, command.to_string());
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
    let cfg_file_name = env::args().nth(1).unwrap();
    let mut s: String = "".to_string();
    let mut file = File::open(&cfg_file_name)
        .unwrap_or_else(|e| panic!("{e:?}: Could not open file: {cfg_file_name}"));
    file.read_to_string(&mut s)
        .expect("Could not read file contents");
    let (command_table, channel): (HashMap<u8, String>, u8) = make_table(&s)?;
    let status: Option<MidiStatus> = None;
    // Read stdin a byte at a time
    let mut buffer = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock(); // Lock the stdin handle for efficient reading

    // Count bytes in stream to identify note bytes
    let mut counter: u32 = 0;

    loop {
        match handle.read(&mut buffer) {
            Ok(0) =>
            // EOF
            {
                break
            }
            Err(e) => return Err(Box::new(e)),
            Ok(_) => {
                let byte = buffer[0];
                if byte & 0x80 == (0x80 | channel) {
                    // Status byte on this channel:
                    if let Some(MidiStatus::NoteOn(_)) = MidiStatus::from_byte(byte) {
                        // Only status that is important is NoteOn
                        counter = 0;
                    }
                    continue;
                } else {
                    // Data byte
                    if let Some(MidiStatus::NoteOn(_)) = status.as_ref() {
                        counter += 1;
                        if counter % 2 == 1 {
                            // This is an odd byte it is a note, so may be a command
                            if let Some(command) = command_table.get(&byte) {
                                run_command(command)?;
                            }
                        }
                    }
                };
            }
        }
    }
    Ok(())
}
