pub trait Sink {
    type Item;
    type Error;

    fn poll_ready(&mut self) -> nb::Result<(), Self::Error>;
    fn start_send(&mut self, item: Self::Item) -> nb::Result<(), Self::Error>;
    fn poll_flush(&mut self) -> nb::Result<(), Self::Error>;
    fn poll_close(&mut self) -> nb::Result<(), Self::Error>;
}
