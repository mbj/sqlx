use crate::message::{BackendMessage, BackendMessageFormat};
use crate::bytes::Bytes;
use crate::Error;

pub struct ParseComplete;

impl BackendMessage for ParseComplete {
    const FORMAT: BackendMessageFormat = BackendMessageFormat::ParseComplete;

    fn decode_body(_bytes: Bytes) -> Result<Self, Error> {
        Ok(ParseComplete)
    }
}
