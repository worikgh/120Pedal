//! Read MIDI signals tripples on STDIN
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Read;
mod midi_status;
use crate::midi_status::MidiStatus;
use std::process::Child;
use std::process::Command;
fn run_command(_command: &str) -> Result<Child, Box<dyn Error>> {
    let child = Command::new("my_program").spawn().expect("Failed to start");
    // Do other work here...
    Ok(child)
}

fn make_table(description: &str) -> Result<HashMap<u8, String>, Box<dyn Error>> {
    description
        .lines()
        .filter(|s| s.starts_with("c "))
        .map(|s| {
            let (byte, command) = s
                .split_once(' ')
                .ok_or(format!("Line '{}' has invalid format", s))?;
            let byte = byte.parse::<u8>()?;
            Ok((byte, command.to_string()))
        })
        .collect()
}
fn main() -> Result<(), Box<dyn Error>> {
    let cfg_file_name = env::args().nth(1).unwrap();
    let mut s: String = "".to_string();
    let mut file = File::open(&cfg_file_name)
        .unwrap_or_else(|e| panic!("{e:?}: Could not open file: {cfg_file_name}"));
    file.read_to_string(&mut s)
        .expect("Could not read file contents");
    let command_table: HashMap<u8, String> = make_table(&s)?;
    let status: Option<MidiStatus> = None;
    // Read stdin a byte at a time
    let mut buffer = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock(); // Lock the stdin handle for efficient reading

    // Count bytes in stream to identify note bytes
    let mut counter: u32 = 0;

    // Hold the children run in response to a command
    let mut children: HashMap<String, Vec<Child>> = HashMap::new();
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
                if byte & 0x80 == 0x80 {
                    // Status byte:
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
				let child = run_command(command)?;
				children.entry(command.to_string())
				    .or_default()  // Creates empty Vec if key doesn't exist
				    .push(child);   // Appends the new child
			    }
                        }
                    }
                };
            }
        }
    }
    Ok(())
}
