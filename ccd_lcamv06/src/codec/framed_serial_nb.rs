use super::framed_state::{FramedState, RWFrames};
use super::nb_traits::{sink::Sink, stream::Stream};
use super::{decoder::Decoder, encoder::Encoder};

use embedded_hal_nb::serial;

pub struct FramedSerialNB<Serial, Codec, const READ_BUF_SIZE: usize = 128, const WRITE_BUF_SIZE: usize = 128>
where
    Codec: Decoder + Encoder,
    Serial: serial::Read + serial::Write,
{
    serial: Serial,
    inner:
        FramedState<Codec, RWFrames<READ_BUF_SIZE, WRITE_BUF_SIZE>, READ_BUF_SIZE, WRITE_BUF_SIZE>,
}

impl<S, C, const R: usize, const W: usize> Stream for FramedSerialNB<S, C, R, W>
where
    C: Decoder + Encoder,
    S: serial::Read + serial::Write,
{
    type Item = Result<<C as Decoder>::Item, <C as Decoder>::Error>;

    fn poll_next(&mut self) -> nb::Result<Option<Self::Item>, core::convert::Infallible> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl<S, C, const R: usize, const W: usize> Sink for FramedSerialNB<S, C, R, W>
where
    C: Decoder + Encoder,
    S: serial::Read + serial::Write,
{
    type Item = <C as Encoder>::Item;

    type Error = <C as Encoder>::Error;

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
