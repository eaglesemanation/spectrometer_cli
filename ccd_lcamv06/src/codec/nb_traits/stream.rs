use core::convert::Infallible;

pub trait Stream {
    type Item;

    fn poll_next(&mut self) -> nb::Result<Option<Self::Item>, Infallible>;
    fn size_hint(&self) -> (usize, Option<usize>); 
}
