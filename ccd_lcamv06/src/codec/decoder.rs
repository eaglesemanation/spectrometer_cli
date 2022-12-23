use bytes::BytesMut;
use thiserror::Error;

#[derive(Error, PartialEq, Eq, Clone, Copy)]
pub enum DecoderError<E> {
    #[error("Unexpected EOF caused decoder to fail")]
    UnexpectedEof,
    #[error("{0}")]
    Other(E),
}

impl<E> From<E> for DecoderError<E> {
    fn from(value: E) -> Self {
        DecoderError::Other(value)
    }
}

pub trait Decoder {
    type Error: core::error::Error;
    type Item;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error>;

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, DecoderError<Self::Error>> {
        match self.decode(buf)? {
            Some(frame) => Ok(Some(frame)),
            None => {
                if buf.is_empty() {
                    Ok(None)
                } else {
                    Err(DecoderError::UnexpectedEof)
                }
            }
        }
    }
}
