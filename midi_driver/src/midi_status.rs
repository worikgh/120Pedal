#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiStatus {
    NoteOff(u8),          // 0x80..=0x8F
    NoteOn(u8),           // 0x90..=0x9F
    PolyphonicAftertouch(u8), // 0xA0..=0xAF
    ControlChange(u8),    // 0xB0..=0xBF
    ProgramChange(u8),    // 0xC0..=0xCF
    ChannelAftertouch(u8), // 0xD0..=0xDF
    PitchBend(u8),        // 0xE0..=0xEF
    SystemExclusive,      // 0xF0
    TimeCodeQuarterFrame, // 0xF1
    SongPositionPointer,  // 0xF2
    SongSelect,           // 0xF3
    Undefined1,           // 0xF4
    Undefined2,           // 0xF5
    TuneRequest,          // 0xF6
    EndOfExclusive,       // 0xF7
    TimingClock,          // 0xF8
    Undefined3,           // 0xF9
    Start,                // 0xFA
    Continue,             // 0xFB
    Stop,                 // 0xFC
    Undefined4,           // 0xFD
    ActiveSensing,        // 0xFE
    Reset,                // 0xFF
}

impl MidiStatus {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x80..=0x8F => Some(MidiStatus::NoteOff(byte & 0x0F)),
            0x90..=0x9F => Some(MidiStatus::NoteOn(byte & 0x0F)),
            0xA0..=0xAF => Some(MidiStatus::PolyphonicAftertouch(byte & 0x0F)),
            0xB0..=0xBF => Some(MidiStatus::ControlChange(byte & 0x0F)),
            0xC0..=0xCF => Some(MidiStatus::ProgramChange(byte & 0x0F)),
            0xD0..=0xDF => Some(MidiStatus::ChannelAftertouch(byte & 0x0F)),
            0xE0..=0xEF => Some(MidiStatus::PitchBend(byte & 0x0F)),
            0xF0 => Some(MidiStatus::SystemExclusive),
            0xF1 => Some(MidiStatus::TimeCodeQuarterFrame),
            0xF2 => Some(MidiStatus::SongPositionPointer),
            0xF3 => Some(MidiStatus::SongSelect),
            0xF4 => Some(MidiStatus::Undefined1),
            0xF5 => Some(MidiStatus::Undefined2),
            0xF6 => Some(MidiStatus::TuneRequest),
            0xF7 => Some(MidiStatus::EndOfExclusive),
            0xF8 => Some(MidiStatus::TimingClock),
            0xF9 => Some(MidiStatus::Undefined3),
            0xFA => Some(MidiStatus::Start),
            0xFB => Some(MidiStatus::Continue),
            0xFC => Some(MidiStatus::Stop),
            0xFD => Some(MidiStatus::Undefined4),
            0xFE => Some(MidiStatus::ActiveSensing),
            0xFF => Some(MidiStatus::Reset),
            _ => None,
        }
    }
}
