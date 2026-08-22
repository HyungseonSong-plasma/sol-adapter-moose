use crate::canonical_json;
use serde_json::Value;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const STDIO_FRAME_TERMINATOR: u8 = b'\n';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdioFrameError {
    EmptyFrame,
    InvalidUtf8,
    InvalidJson(String),
    NonObjectMessage,
    UnterminatedFrame { buffered_bytes: usize },
}

impl Display for StdioFrameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFrame => write!(formatter, "stdio frame must contain a JSON object"),
            Self::InvalidUtf8 => write!(formatter, "stdio frame must be valid UTF-8"),
            Self::InvalidJson(detail) => {
                write!(formatter, "stdio frame is not valid JSON: {detail}")
            }
            Self::NonObjectMessage => write!(formatter, "stdio frame must contain one JSON object"),
            Self::UnterminatedFrame { buffered_bytes } => write!(
                formatter,
                "stdio reached EOF with an unterminated frame ({buffered_bytes} buffered bytes)"
            ),
        }
    }
}

impl Error for StdioFrameError {}

/// Encodes one JSON-RPC object as canonical compact JSON followed by LF.
pub fn encode_stdio_frame(json: &str) -> Result<Vec<u8>, StdioFrameError> {
    let value: Value = serde_json::from_str(json)
        .map_err(|error| StdioFrameError::InvalidJson(error.to_string()))?;
    if !value.is_object() {
        return Err(StdioFrameError::NonObjectMessage);
    }

    let mut frame = canonical_json(&value).into_bytes();
    frame.push(STDIO_FRAME_TERMINATOR);
    Ok(frame)
}

/// Incrementally separates LF-delimited JSON-RPC messages from arbitrary
/// process-output chunks. Canonical output uses LF; CRLF input is accepted.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StdioFrameDecoder {
    pending: Vec<u8>,
}

impl StdioFrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending_bytes(&self) -> usize {
        self.pending.len()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<Result<String, StdioFrameError>> {
        self.pending.extend_from_slice(bytes);
        let mut frames = Vec::new();

        while let Some(terminator) = self
            .pending
            .iter()
            .position(|byte| *byte == STDIO_FRAME_TERMINATOR)
        {
            let remainder = self.pending.split_off(terminator + 1);
            let mut frame = std::mem::replace(&mut self.pending, remainder);
            frame.pop();
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }

            if frame.iter().all(u8::is_ascii_whitespace) {
                frames.push(Err(StdioFrameError::EmptyFrame));
                continue;
            }

            frames.push(String::from_utf8(frame).map_err(|_| StdioFrameError::InvalidUtf8));
        }

        frames
    }

    pub fn finish(&mut self) -> Result<(), StdioFrameError> {
        if self.pending.is_empty() {
            return Ok(());
        }

        let buffered_bytes = self.pending.len();
        self.pending.clear();
        Err(StdioFrameError::UnterminatedFrame { buffered_bytes })
    }
}
