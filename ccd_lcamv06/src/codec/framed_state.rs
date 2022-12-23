use super::{
    decoder::Decoder,
    encoder::Encoder,
    nb_traits::{sink::Sink, stream::Stream},
};
use core::borrow::{Borrow, BorrowMut};

pub(crate) struct FramedState<Codec, State, const READ_BUF_SIZE: usize, const WRITE_BUF_SIZE: usize>
{
    codec: Codec,
    state: State,
}

pub(crate) struct RWFrames<const R: usize, const W: usize> {
    read: ReadFrame<R>,
    write: WriteFrame<W>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
enum ReadState {
    #[default]
    Reading,
    Framing,
    Pausing,
    Paused,
    Errored,
}

pub(crate) struct ReadFrame<const BUF_SIZE: usize> {
    buf: [u8; BUF_SIZE],
    state: ReadState,
}

impl<const R: usize, const W: usize> Borrow<ReadFrame<R>> for RWFrames<R, W> {
    fn borrow(&self) -> &ReadFrame<R> {
        &self.read
    }
}
impl<const R: usize, const W: usize> BorrowMut<ReadFrame<R>> for RWFrames<R, W> {
    fn borrow_mut(&mut self) -> &mut ReadFrame<R> {
        &mut self.read
    }
}

pub(crate) struct WriteFrame<const BUF_SIZE: usize> {
    buf: [u8; BUF_SIZE],
}

impl<const R: usize, const W: usize> Borrow<WriteFrame<W>> for RWFrames<R, W> {
    fn borrow(&self) -> &WriteFrame<W> {
        &self.write
    }
}
impl<const R: usize, const W: usize> BorrowMut<WriteFrame<W>> for RWFrames<R, W> {
    fn borrow_mut(&mut self) -> &mut WriteFrame<W> {
        &mut self.write
    }
}

impl<C, S, const R: usize, const W: usize> Stream for FramedState<C, S, R, W>
where
    C: Decoder,
    S: BorrowMut<ReadFrame<R>>,
{
    type Item = Result<C::Item, C::Error>;

    fn poll_next(&mut self) -> nb::Result<Option<Self::Item>, core::convert::Infallible> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl<C, S, const R: usize, const W: usize> Sink for FramedState<C, S, R, W>
where
    C: Encoder,
    S: BorrowMut<WriteFrame<W>>,
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
