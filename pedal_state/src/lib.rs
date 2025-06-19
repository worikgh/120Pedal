use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
/// Store the current state of the connections
use std::fs::OpenOptions;
use std::io::Read;
use std::io::{self, Write};
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct PedalState {
    // The selected effect
    pub selected: Option<u8>,
    // The effects to choose from and their volumes, The volumes
    // are set in the Gui and not affected by this programme
    // f32 because that is what Pure Data wants
    pub choices: Vec<(u8, f32)>,
}
impl PedalState {
    pub fn new(command_table: &HashMap<u8, Vec<(String, String)>>) -> Self {
        let mut choices = command_table
            .iter()
            // Volume default
            .map(|(k, _)| (*k, 0.5))
            .collect::<Vec<(u8, f32)>>();
        choices.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            selected: None,
            choices, //: Vec::new(),
        }
    }
}
fn state_file_name(pedal_dir: &str) -> String {
    format!("{pedal_dir}/.state")
}
pub fn write_state(state: &PedalState, pedal_dir: &str) -> io::Result<()> {
    let json =
        serde_json::to_string(state).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let fname = state_file_name(pedal_dir);
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(fname)?;
    file.lock_exclusive()?;
    file.write_all(&json.into_bytes())?;
    Ok(())
}
pub fn read_state(pedal_dir: &str) -> io::Result<Option<PedalState>> {
    match OpenOptions::new()
        .read(true)
        .open(state_file_name(pedal_dir))
    {
        Ok(mut file) => {
            file.lock_exclusive()?;
            let mut json = String::new();
            file.read_to_string(&mut json)?;
            let result = serde_json::from_str::<PedalState>(&json)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(Some(result))
        }
        Err(err) => {
            eprintln!("Error PedalState.read_state: {err:?}");
            Ok(None)
        }
    }
}
