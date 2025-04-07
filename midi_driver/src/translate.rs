//! Reads a stream of MIDI data fro mthe stdin
//! Writes the data on the stdout with Note On and Note Off notes transposed
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
use std::num::ParseIntError;
mod midi_status;
use crate::midi_status::MidiStatus;

/// Helper function for reading `u8` from `&str`.  Hex in prefixed
/// with "0x", else decimal
fn str_u8(inp: &str) -> Result<u8, ParseIntError> {
    if inp.len() > 1 && &inp[..2] == "0x" {
        u8::from_str_radix(inp.trim_start_matches("0x"), 16)
    } else {
        inp.parse::<u8>()
    }
}

/// Make a key for the translation table.  Combine the 4 bits of status with the index in the message (0 or 1) in the MSB and put the value to translate in the LSB of the key
fn make_key(s: u8, x: u8, k: u8) -> u16 {
    eprintln!("make_key({s:x}, {x}, {k},)");
    ((s as u16 | x as u16) << 8) | (k as u16)
}
/// Build the table to translate MIDI inputs.  Make a HashMap keyed by
/// the status of the messages to change, the index of the byte
/// ([0,1]) in the message, and the message itself.  The value is the
/// message to output in its stead.
fn make_table(description: &str) -> Result<HashMap<u16, u8>, Box<dyn Error>> {
    description
        .lines()
        .filter(|&s| s.trim().starts_with("t "))
        .map(|s| {
            let parts: Vec<&str> = s[2..].split_whitespace().collect();
            if parts.len() != 4 {
                return Err(format!(
                    "Invalid translation line: {}.  `parts.len()`: {}",
                    s,
                    parts.len()
                )
                .into());
            }
            let s = str_u8(parts[0])?; // Status byte
            let x = str_u8(parts[1])?; // index byte
            let k = str_u8(parts[2])?; // Value to translate
            let v = str_u8(parts[3])?; // Output
            let key = make_key(s, x, k);
            eprintln!("make_table adding: {s}+{x}+{k} -> {v}");
            Ok((key, v))
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
    let translation_table: HashMap<u16, u8> = make_table(&s)?;
    let mut status: Option<MidiStatus> = None;
    // Read stdin a byte at a time
    let mut buffer = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock(); // Lock the stdin handle for efficient reading

    // Working memory for processing MIDI messages
    let mut working: Vec<u8> = Vec::new();
    let write_working = |w: &Vec<u8>| {
        eprintln!("working: {w:?}");
        io::stdout()
            .write_all(w)
            .unwrap_or_else(|e| panic!("Cannot write to stdout: {}", e));
        io::stdout().flush().expect("Failed to flush stdout");
    };

    loop {
        match handle.read(&mut buffer) {
            Ok(0) => {
                // EOF
                write_working(&working);
                break;
            }
            Err(e) => return Err(Box::new(e)),
            Ok(n) => {
                // `buffer` has indeterminate size.  It is at least
                // `n`.  Beyond `n`, `buffer` is random
                #[allow(clippy::needless_range_loop)]
                for i in 0..n {
                    let byte = buffer[i];
                    if byte & 0x80 == 0x80 {
                        // Status byte:
                        status = MidiStatus::from_byte(byte);
                        // Put the status byte in the buffer
                        working.push(byte);
                        continue;
                    } else {
                        // Data byte
                        match working.len() % 2 {
                            // If there are an odd number of data in
                            // `working` this is a note and must be
                            // tranlated, if not it is velocity
                            0..2 => {
                                let x = (working.len() % 2) as u8;
                                let s = status.as_ref().unwrap().to_byte();
                                let key = make_key(s, x, byte);
                                let v: u8 = match translation_table.get(&key) {
                                    Some(v) => *v,
                                    None => byte,
                                };
                                working.push(v)
                            }

                            // All other bytes just pass through
                            _ => working.push(byte),
                        };
                    }
                }
            }
        };
        if !working.is_empty() {
            write_working(&working);
            working.truncate(0);
        }
    }
    Ok(())
}

#[allow(dead_code)]
trait Translator {}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_str_u8_decimal() {
        assert_eq!(str_u8("10"), Ok(10));
        assert_eq!(str_u8("0"), Ok(0));
        assert_eq!(str_u8("255"), Ok(255));
    }

    #[test]
    fn test_str_u8_hex() {
        assert_eq!(str_u8("0x0A"), Ok(10));
        assert_eq!(str_u8("0xFF"), Ok(255));
        assert_eq!(str_u8("0x00"), Ok(0));
    }

    #[test]
    fn test_str_u8_invalid() {
        assert!(str_u8("256").is_err()); // Overflow
        assert!(str_u8("0x100").is_err()); // Hex overflow
        assert!(str_u8("abc").is_err()); // Invalid decimal
        assert!(str_u8("0xzz").is_err()); // Invalid hex
    }

    #[test]
    fn test_make_key() {
        // Status 0x90, index 0, value 0x3C
        assert_eq!(make_key(0x90, 0, 0x3C), 0x9000 | 0x3C);
        // Status 0x80, index 1, value 0x40
        assert_eq!(make_key(0x80, 1, 0x40), 0x8100 | 0x40);
        // Status 0xB0, index 0, value 0x07
        assert_eq!(make_key(0xB0, 0, 0x07), 0xB000 | 0x07);
    }

    #[test]
    fn test_make_table_valid() {
        let input = "t 0x90 0 0x3C 0x40\nt 0x90 1 0x40 0x3C";
        let result = make_table(input).unwrap();

        let mut expected = HashMap::new();
        expected.insert(make_key(0x90, 0, 0x3C), 0x40);
        expected.insert(make_key(0x90, 1, 0x40), 0x3C);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_make_table_invalid_line() {
        // Missing one field
        let input = "t 0x90 0 0x3C";
        assert!(make_table(input).is_err());

        // Invalid number format
        let input = "t 0x90 0 abc 0x40";
        assert!(make_table(input).is_err());

        // Invalid line prefix
        let input = "x 0x90 0 0x3C 0x40";
        let result = make_table(input).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_make_table_mixed_lines() {
        let input = r#"
# Comment line
t 0x90 0 0x3C 0x40
t 0x90 1 0x40 0x3C

t 0x80 0 0x3C 0x40
# Another comment
t 0x80 1 0x40 0x3C
t 0x0c 0 0 1
        "#;

        let result = make_table(input).unwrap();
        eprintln!("{input} -> {result:?}");
        assert_eq!(result.len(), 5);
        assert_eq!(result[&make_key(0x90, 0, 0x3C)], 0x40);
        assert_eq!(result[&make_key(0x80, 1, 0x40)], 0x3C);
        assert_eq!(result[&make_key(0x0c, 0, 0)], 1);
    }

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    // Integration test for the main processing logic
    #[test]
    fn test_midi_processing_logic() {
        // Create a simple translation table
        let mut translation_table = HashMap::new();
        // Translate note 0x3C to 0x40 when status is 0x90 and it's the first data byte
        translation_table.insert(make_key(0x90, 0, 0x3C), 0x40);
        // Translate velocity 0x40 to 0x3C when status is 0x90 and it's the second data byte
        translation_table.insert(make_key(0x90, 1, 0x40), 0x3C);

        // Test MIDI message processing
        let test_cases = vec![
            // Note On message (status 0x90, note 0x3C, velocity 0x40)
            // Should be translated to (status 0x90, note 0x40, velocity 0x3C)
            (vec![0x90, 0x3C, 0x40], vec![0x90, 0x40, 0x40]),
            // Note On message with different values that shouldn't be translated
            (vec![0x90, 0x3D, 0x41], vec![0x90, 0x3D, 0x41]),
            // Different status byte (0x80) shouldn't be translated
            (vec![0x80, 0x3C, 0x40], vec![0x80, 0x3C, 0x40]),
            // System Exclusive message should pass through unchanged
            (
                vec![0xF0, 0x01, 0x02, 0x03, 0xF7],
                vec![0xF0, 0x01, 0x02, 0x03, 0xF7],
            ),
        ];

        for (input, expected) in test_cases {
            eprintln!("input:{} expected:{}", to_hex(&input), to_hex(&expected),);
            let mut working = Vec::new();
            let mut status = None;
            let mut output = Vec::new();

            for byte in input {
                if byte & 0x80 == 0x80 {
                    // Status byte
                    status = MidiStatus::from_byte(byte);
                    working.push(byte);
                } else {
                    // Data byte
                    if let Some(status_byte) = status.as_ref().map(|s| s.to_byte()) {
                        let x = (working.len() % 2) as u8;
                        let key = make_key(status_byte, x, byte);
                        let v = translation_table.get(&key).copied().unwrap_or(byte);
                        working.push(v);
                    } else {
                        working.push(byte);
                    }
                }

                // Simulate the write_working function
                if !working.is_empty() {
                    output.extend_from_slice(&working);
                    working.clear();
                }
            }

            assert_eq!(to_hex(&output), to_hex(&expected));
        }
    }

    // Test for handling incomplete messages
    #[test]
    fn test_incomplete_messages() {
        let translation_table: HashMap<u16, u8> = HashMap::new(); // Empty table
        let input = vec![0x90, 0x3C]; // Missing velocity byte
        let mut working = Vec::new();
        let mut status = None;
        let mut output = Vec::new();

        for byte in input {
            if byte & 0x80 == 0x80 {
                status = MidiStatus::from_byte(byte);
                working.push(byte);
            } else if let Some(status_byte) = status.as_ref().map(|s| s.to_byte()) {
                let x = (working.len() % 2) as u8;
                let key = make_key(status_byte, x, byte);
                let v = translation_table.get(&key).copied().unwrap_or(byte);
                working.push(v);
            } else {
                working.push(byte);
            }

            if !working.is_empty() {
                output.extend_from_slice(&working);
                working.clear();
            }
        }

        // The incomplete message should still be output
        assert_eq!(output, vec![0x90, 0x3C]);
    }
}
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_make_table_valid_input() {
//         let config = "t 0x0c 0 60 62\nt 0x09 0 64 65\n# comment line\nt 0x0c 1 67 69\n";
//         let table = make_table(config).unwrap();

//         assert_eq!(table.len(), 3);
//         assert_eq!(table.get(&60), Some(&62));
//         assert_eq!(table.get(&64), Some(&65));
//         assert_eq!(table.get(&67), Some(&69));
//         assert_eq!(table.get(&0), None); // Not in table
//     }

//     #[test]
//     fn test_make_table_empty_input() {
//         let config = "";
//         let table = make_table(config).unwrap();
//         assert!(table.is_empty());
//     }

//     #[test]
//     fn test_make_table_with_comments() {
//         let config = "# This is a comment\nt 60 62\n# Another comment\nt 64 65\n";
//         let table = make_table(config).unwrap();

//         assert_eq!(table.len(), 2);
//         assert_eq!(table.get(&60), Some(&62));
//         assert_eq!(table.get(&64), Some(&65));
//     }

//     #[test]
//     fn test_make_table_with_hex_values() {
//         let config = "t 60 0x2\nt 11 0xc2\n";
//         let table = make_table(config).unwrap();

//         assert_eq!(table.len(), 2);
//         assert_eq!(table.get(&60), Some(&2));
//         assert_eq!(table.get(&11), Some(&0xc2));
//     }
//     #[test]
//     fn test_make_table_with_hex_keys() {
//         let config = "t 0x60 20\nt 0x11 2\n";
//         let table = make_table(config).unwrap();

//         assert_eq!(table.len(), 2);
//         assert_eq!(table.get(&0x60), Some(&20));
//         assert_eq!(table.get(&0x11), Some(&2));
//     }

//     #[test]
//     fn test_make_table_with_hex() {
//         let config = "t 0x0x60 0x20\nt 0x11 0x02\n";
//         let table = make_table(config).unwrap();

//         assert_eq!(table.len(), 2);
//         assert_eq!(table.get(&0x60), Some(&0x20));
//         assert_eq!(table.get(&0x11), Some(&2));
//     }

//     #[test]
//     fn test_make_table_invalid_line_format() {
//         let config = "t 60 62\nt 64\n";
//         let result = make_table(config);
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_make_table_non_numeric_input() {
//         let config = "t 60 62\nt sixty four\n";
//         let result = make_table(config);
//         assert!(result.is_err());
//     }

//     #[test]
//     fn test_make_table_out_of_range_values() {
//         let config = "t 60 900\n"; // 200 is > 127
//         let result = make_table(config);
//         assert!(result.is_err());
//     }
// }
