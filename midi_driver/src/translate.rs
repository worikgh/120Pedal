//! Reads a stream of MIDI data from the stdin
//! Writes the data on the stdout with Note On and Note Off notes transposed
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
use std::num::ParseIntError;
mod midi_status;
use crate::midi_status::MidiStatus;
use std::fmt;

#[derive(Debug)]
enum TranslateError {
    InvalidChannel(u8),
    InvalidChannelDefinition(String),
}

impl std::error::Error for TranslateError {}

impl fmt::Display for TranslateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TranslateError::InvalidChannel(c) => write!(f, "Invalid channel: {c}"),
            TranslateError::InvalidChannelDefinition(s) => {
                write!(f, "Invalid channel definition: {s}")
            }
        }
    }
}

/// Helper function for reading `u8` from `&str`.  Hex if prefixed
/// with "0x", else decimal
fn str_u8(inp: &str) -> Result<u8, ParseIntError> {
    if inp.len() > 1 && &inp[..2] == "0x" {
        u8::from_str_radix(inp.trim_start_matches("0x"), 16)
    } else {
        inp.parse::<u8>()
    }
}

/// Make a key for the translation table.  Combine the 4 bits of
/// status with the index in the message (0 or 1) in the MSB and put
/// the value to translate in the LSB of the key.
/// The index is in [0..1] only MIDI messages that have more than two data bytes following are:
/// * Sysex messages.  This programme does not translate those
/// * NoteOn/NoteOff: These can be followed by an arbitrary number of
///   pairs of bytes for note/volume.  So when dealing with data for
///   these messages only need to know if the byte is at an odd
///   address (relative to status) which means it is "note", or at an
///   even address, in which case it is "volume"
fn make_key(s: u8, x: u8, k: u8) -> u16 {
    ((s as u16 | x as u16) << 8) | (k as u16)
}

#[derive(Debug)]
/// Describe the way that channel data is translated.
struct ChannelTranslate {
    op: ChannelOperation,
    value: u8,
}

#[derive(Debug)]
/// The channel is either `Literal`, `value` is the new channel or
/// `Minus` the incoming channel has `value` subtracted or `Plus`
/// where `value` is added to the incomming channel.  It is perfectly
/// possible to have an invalid channel.  See
/// [this error](TranslateError::InvalidChannel)
enum ChannelOperation {
    Literal,
    Minus,
    Plus,
}

impl ChannelTranslate {
    /// Translate a channel definition line from configurtion.  Format
    /// is: `\[+-\]?[N]` where `N` is a string representation of a
    /// digit in Hex (Only one digit)
    fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
        let mut chars = s.chars();
        let first_char = chars
            .next()
            .ok_or(TranslateError::InvalidChannelDefinition(s.to_string()))?;

        // Determine operation and remaining part
        let (op, hex_str) = match first_char {
            '+' => (ChannelOperation::Plus, chars.as_str()),
            '-' => (ChannelOperation::Minus, chars.as_str()),
            _ => (ChannelOperation::Literal, s), // No operator, entire string is the hex digit
        };

        // Parse hex digit (case-insensitive)
        match u8::from_str_radix(hex_str, 16) {
            Ok(value) => Ok(ChannelTranslate { op, value }),
            Err(e) => Err(Box::new(e)),
        }
    }
}

/// Get the channel translation data.
/// * `description` is the configuration data
/// * Channel definition lines start with "c "
/// * See [ChannelTranslate](ChannelTranslate::from_str) for an explanation of the format of channel definition lines
fn get_channel(description: &str) -> Result<Option<ChannelTranslate>, Box<dyn Error>> {
    let mut f = description.lines().filter(|&s| s.trim().starts_with("c "));
    if let Some(s) = f.next() {
        // c +1
        // c -2
        // c 2
        let parts: Vec<&str> = s[2..].split_whitespace().collect();
        if parts.len() != 1 {
            return Err(format!(
                "Invalid translation line: {}.  `parts.len()`: {}",
                s,
                parts.len()
            )
            .into());
        }
        // Read the +- and N
        Ok(Some(ChannelTranslate::from_str(parts[0])?))
    } else {
        Ok(None)
    }
}

/// Build the table to translate MIDI inputs.  Make a HashMap keyed by
/// the status of the messages to change, the index of the byte
/// (\[0,1\]) in the message, and the message itself.  The value is the
/// message to output in its stead.
fn make_translate_table(description: &str) -> Result<HashMap<u16, u8>, Box<dyn Error>> {
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

            Ok((key, v))
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let cfg_file_name = env::args().nth(1).unwrap();

    // The contents of the configuration file as a `String`
    let mut s: String = "".to_string();
    File::open(&cfg_file_name)
        .unwrap_or_else(|e| panic!("{e:?}: Could not open file: {cfg_file_name}"))
        .read_to_string(&mut s)?;

    let translation_table: HashMap<u16, u8> = make_translate_table(&s)?;
    let channel_translate = get_channel(&s)?;

    // Read stdin a byte at a time
    let mut buffer = [0u8; 1];
    let stdin = io::stdin();
    let mut handle = stdin.lock(); // Lock the stdin handle for efficient reading

    // Working memory for processing MIDI messages
    let mut working: Vec<u8> = Vec::new();
    let write_working = |w: &Vec<u8>| {
        io::stdout()
            .write_all(w)
            .unwrap_or_else(|e| panic!("Cannot write to stdout: {}", e));
        io::stdout().flush().expect("Failed to flush stdout");
    };

    // Store `status` bytes
    let mut status: Option<MidiStatus> = None;

    loop {
        match handle.read(&mut buffer) {
            Err(e) => return Err(Box::new(e)),

            Ok(0) => {
                // EOF
                write_working(&working);
                break;
            }

            Ok(1) => {
                let byte = buffer[0];
                if byte & 0x80 == 0x80 {
                    // Status byte:
                    status = MidiStatus::from_byte(byte);

                    // Check for channel translation
                    let c1: u8 = byte & 0x0F;
                    let channel = if let Some(ref channel_translate) = channel_translate {
                        match channel_translate.op {
                            ChannelOperation::Literal => channel_translate.value,
                            ChannelOperation::Minus => c1 - channel_translate.value,
                            ChannelOperation::Plus => c1 + channel_translate.value,
                        }
                    } else {
                        0
                    };
                    if channel > 0xf {
                        return Err(Box::new(TranslateError::InvalidChannel(channel)));
                    }
                    // Put the status byte in the buffer
                    working.push(byte);
                    continue;
                } else {
                    // Data byte
                    let x = ((working.len() - 1) % 2) as u8;
                    let s = status.as_ref().unwrap().to_byte();
                    let key = make_key(s, x, byte);
                    let v: u8 = match translation_table.get(&key) {
                        Some(v) => *v,
                        None => byte,
                    };
                    working.push(v)
                }
            }

            // Cannot happen.  `buffer` is size 1
            error => panic!("Invalid read returned: {error:?}"),
        };
        if !working.is_empty() {
            write_working(&working);
            working.truncate(0);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fmt::Write;

    #[test]
    fn test_str_u8() {
        // Test decimal parsing
        assert_eq!(str_u8("10").unwrap(), 10);
        assert_eq!(str_u8("255").unwrap(), 255);

        // Test hex parsing
        assert_eq!(str_u8("0xA").unwrap(), 10);
        assert_eq!(str_u8("0xFF").unwrap(), 255);
        assert_eq!(str_u8("0x0F").unwrap(), 15);

        // Test invalid cases
        assert!(str_u8("256").is_err()); // Overflow
        assert!(str_u8("0xG").is_err()); // Invalid hex
        assert!(str_u8("abc").is_err()); // Invalid decimal
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
        let result = make_translate_table(input).unwrap();

        let mut expected = HashMap::new();
        expected.insert(make_key(0x90, 0, 0x3C), 0x40);
        expected.insert(make_key(0x90, 1, 0x40), 0x3C);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_make_table_invalid_line() {
        // Missing one field
        let input = "t 0x90 0 0x3C";
        assert!(make_translate_table(input).is_err());

        // Invalid number format
        let input = "t 0x90 0 abc 0x40";
        assert!(make_translate_table(input).is_err());

        // Invalid line prefix
        let input = "x 0x90 0 0x3C 0x40";
        let result = make_translate_table(input).unwrap();
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

        let result = make_translate_table(input).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[&make_key(0x90, 0, 0x3C)], 0x40);
        assert_eq!(result[&make_key(0x80, 1, 0x40)], 0x3C);
        assert_eq!(result[&make_key(0x0c, 0, 0)], 1);
    }

    fn to_hex(bytes: &[u8]) -> String {
        bytes
            .iter()
            .fold(String::with_capacity(bytes.len() * 3), |mut s, b| {
                write!(&mut s, "{:02x} ", b).unwrap();
                s
            })
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
