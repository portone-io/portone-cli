use crate::ui::IoStreams;

pub struct Factory {
    pub io: IoStreams,
}

impl Factory {
    pub fn detect() -> Self {
        Self::new(IoStreams::detect())
    }

    pub fn new(io: IoStreams) -> Self {
        Self { io }
    }
}
