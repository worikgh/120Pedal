//! Reads a stream of MIDI data fro mthe stdin
//! Writes the data on the stdout with Note On and Note Off notes transposed
use std::collections::HashMap;
use std::num::ParseIntError;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
mod midi_status;
use crate::midi_status::MidiStatus;

/// Helper function for reading `u8` from `&str`.  Hex in prefixed
/// with "0x", else decimal
fn str_u8(inp:&str) -> Result<u8, ParseIntError> {
    if inp.len() > 1 && &inp[..2] == "0x" {
	u8::from_str_radix(inp.trim_start_matches("0x"), 16)
    }else{
	inp.parse::<u8>()
    }
}

/// Build the table to translate MIDI inputs.
/// Transposes "Note On" and "Note Off" notes
fn make_table(description: &str) -> Result<HashMap<u8, u8>, Box<dyn Error>> {
    description
        .lines()
        .filter(|&s| s.starts_with("t "))
        .map(|s| {
            let parts: Vec<&str> = s[2..].split_whitespace().collect();
            if parts.len() != 2 {
                return Err(format!("Invalid translation line: {}", s).into());
            }
            let k = str_u8(parts[0])?;
            let v = str_u8( parts[1])?;
            Ok((k, v))
        })
        .collect()
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

                    // When a status byte arrives flush the buffer
                    if !working.is_empty() {
                        write_working(&working);
                        working.truncate(0);
                    }

                    // Put the status byte in the buffer
                    working.push(byte);
                    continue;
                } else {
                    // Data byte
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

#[allow(dead_code)]
trait Translator {}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_table_valid_input() {
        let config = "t 60 62\nt 64 65\n# comment line\nt 67 69\n";
        let table = make_table(config).unwrap();

        assert_eq!(table.len(), 3);
        assert_eq!(table.get(&60), Some(&62));
        assert_eq!(table.get(&64), Some(&65));
        assert_eq!(table.get(&67), Some(&69));
        assert_eq!(table.get(&0), None); // Not in table
    }

    #[test]
    fn test_make_table_empty_input() {
        let config = "";
        let table = make_table(config).unwrap();
        assert!(table.is_empty());
    }

    #[test]
    fn test_make_table_with_comments() {
        let config = "# This is a comment\nt 60 62\n# Another comment\nt 64 65\n";
        let table = make_table(config).unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table.get(&60), Some(&62));
        assert_eq!(table.get(&64), Some(&65));
    }

    #[test]
    fn test_make_table_with_hex_values() {
        let config = "t 60 0x2\nt 11 0xc2\n";
        let table = make_table(config).unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table.get(&60), Some(&2));
        assert_eq!(table.get(&11), Some(&0xc2));
    }
    #[test]
    fn test_make_table_with_hex_keys() {
        let config = "t 0x60 20\nt 0x11 2\n";
        let table = make_table(config).unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table.get(&0x60), Some(&20));
        assert_eq!(table.get(&0x11), Some(&2));
    }

    #[test]
    fn test_make_table_with_hex() {
        let config = "t 0x0x60 0x20\nt 0x11 0x02\n";
        let table = make_table(config).unwrap();

        assert_eq!(table.len(), 2);
        assert_eq!(table.get(&0x60), Some(&0x20));
        assert_eq!(table.get(&0x11), Some(&2));
    }

    #[test]
    fn test_make_table_invalid_line_format() {
        let config = "t 60 62\nt 64\n";
        let result = make_table(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_make_table_non_numeric_input() {
        let config = "t 60 62\nt sixty four\n";
        let result = make_table(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_make_table_out_of_range_values() {
        let config = "t 60 900\n"; // 200 is > 127
        let result = make_table(config);
        assert!(result.is_err());
    }
}
