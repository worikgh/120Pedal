//! Code to translate MIDI signals from a value in 0..127 to another
//! value 0..127.  This is needed because a MIDI Pedal can output a
//! variety of MIDI values and this software requires a different
//! range
#![allow(unused_imports)]
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;

pub struct MidiTranslate {
    // Translation table
    table: HashMap<u8, u8>,

    // The port name of the pedal
    name: String,
}
impl MidiTranslate {
    /// Create a `MidiTranslate`.  The name of the Pedal's MIDI
    /// interface and the translation table is in a file identified by
    /// `cfg_file`.  The configuration file has two sorts of entries,
    /// one per line: 1. `name <string>` 2. `t <u8> <u8>`.  The first
    /// (`1`) supplies the port name of the pedal, there must be
    /// exactly one of these.  The second (`2`) defines a translation
    /// fromone MIDI value to another.  And MIDI values not listed on
    /// the LHS of a `t` rule is passed unchanged.  `name` must be
    /// present
    pub fn new(cfg_file: &str) -> Self {
        // Read the configuration file into memory, it i line orientated

        // The contents of the configuration file as a `String`
        let mut s: String = "".to_string();

        // The contents of the configuration file as a vector
        let contents: Vec<&str>;

	contents = {
            // let mut file = File::open(cfg_file).expect(&format!("Could not open file: {}", cfg_file));
            // file.read_to_string(&mut s)
            // 	.expect("Could not read file contents from: {cfg_file}");
            // s.split("\n").collect::<Vec<&str>>()
            let mut file =
                File::open(cfg_file).expect(&format!("Could not open file: {}", cfg_file));
            file.read_to_string(&mut s)
                .expect("Could not read file contents");
            s.split('\n').collect()
        };

        // Get the translation commands
        let table:HashMap<u8, u8>  = contents
            .iter()
            .filter(|&s| s.len() > 1 && &s[0..2] == "t ")
            .map(|&s| {
		let parts:Vec<&str> = s[2..].split_whitespace().collect();
		if parts.len() != 2 {
		    panic!("Invalid translation line: {parts:?}");
		}
		let k = parts[0].parse::<u8>().expect(&format!("Invalid key: {}", parts[0]));
		let v = parts[1].parse::<u8>().expect(&format!("Invalid value: {}", parts[1]));
		(k, v)
	    })
            .collect();

	// Get the name.  There must be exactly one
	let name:String = {
	    let name_vec:Vec<&str> = contents
            .iter()
            .filter(|&s| s.len() > 1 && &s[0..5] == "name ")
            .map(|&s| s)
		.collect();
	    if name_vec.len() != 1 {
		panic!("There must be exactly one `name` configuration.  There are {}", name_vec.len());
	    }
	    if name_vec[0].len() < 6 {
		panic!("The `name` is missing");
	    }
	    name_vec[0][5..].to_string()
	};
        // Get the name of the pedal MIDI device.

        Self {
            table,
            name,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_new_creates_instance() {
        // Create a temporary config file with dummy content
	const TEST_NAME:&str = "f";
        let cfg_file = "test_config.txt";
        let mut file = File::create(cfg_file).expect("Could not create config file");
        writeln!(file, "{}", &format!("name {}\nt 3 6\n", TEST_NAME)).expect("Could not write to config file"); // Example content

        // Instantiate MidiTranslate
        let midi_translate = MidiTranslate::new(cfg_file);

        // Check if the instance contains the name
        assert_eq!(midi_translate.name, TEST_NAME.to_string());

	// Check it translates 3 -> 6
	assert_eq!(midi_translate.table.get(&3), Some(&6));
        // Clean up the temporary file
        std::fs::remove_file(cfg_file).expect("Could not remove test config file");
    }
}
