use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("JSON frame is too large: {actual} bytes (maximum {maximum})")]
    TooLarge { actual: usize, maximum: usize },
    #[error("JSON frame is shorter than its four-byte length header")]
    MissingHeader,
    #[error("JSON frame length mismatch: declared {declared}, actual {actual}")]
    LengthMismatch { declared: usize, actual: usize },
    #[error("invalid JSON frame: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn encode_json_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, FrameError> {
    let payload = serde_json::to_vec(value)?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            actual: payload.len(),
            maximum: MAX_FRAME_BYTES,
        });
    }
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_json_frame<T: DeserializeOwned>(frame: &[u8]) -> Result<T, FrameError> {
    if frame.len().saturating_sub(4) > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            actual: frame.len().saturating_sub(4),
            maximum: MAX_FRAME_BYTES,
        });
    }
    if frame.len() < 4 {
        return Err(FrameError::MissingHeader);
    }
    let declared = u32::from_be_bytes(frame[..4].try_into().expect("four bytes checked")) as usize;
    let actual = frame.len() - 4;
    if declared > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            actual: declared,
            maximum: MAX_FRAME_BYTES,
        });
    }
    if declared != actual {
        return Err(FrameError::LengthMismatch { declared, actual });
    }
    Ok(serde_json::from_slice(&frame[4..])?)
}
