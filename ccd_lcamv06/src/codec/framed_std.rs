use super::framed_state::{FramedState, RWFrames};
use super::nb_traits::{sink::Sink, stream::Stream};
use super::{decoder::Decoder, encoder::Encoder};

use thiserror::Error;

pub struct Framed<IO, Codec, const READ_BUF_SIZE: usize = 128, const WRITE_BUF_SIZE: usize = 128>
where
    Codec: Decoder + Encoder,
    IO: std::io::Read + std::io::Write,
{
    io: IO,
    inner:
        FramedState<Codec, RWFrames<READ_BUF_SIZE, WRITE_BUF_SIZE>, READ_BUF_SIZE, WRITE_BUF_SIZE>,
}

#[derive(Error, Debug)]
pub enum FramedError<E> {
    #[error("IO error: {0}")]
    IO(std::io::Error),
    #[error("{0}")]
    Other(E),
}

impl<IO, C, const R: usize, const W: usize> Stream for Framed<IO, C, R, W>
where
    C: Decoder + Encoder,
    IO: std::io::Read + std::io::Write,
{
    type Item = Result<<C as Decoder>::Item, <C as Decoder>::Error>;

    fn poll_next(&mut self) -> nb::Result<Option<Self::Item>, core::convert::Infallible> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl<IO, C, const R: usize, const W: usize> Sink for Framed<IO, C, R, W>
where
    C: Decoder + Encoder,
    IO: std::io::Read + std::io::Write,
{
    type Item = <C as Encoder>::Item;

    type Error = FramedError<<C as Encoder>::Error>;

    fn poll_ready(&mut self) -> nb::Result<(), Self::Error> {
        todo!()
    }

    fn start_send(&mut self, item: Self::Item) -> nb::Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(&mut self) -> nb::Result<(), Self::Error> {
        todo!()
    }

    fn poll_close(&mut self) -> nb::Result<(), Self::Error> {
        todo!()
    }
}
