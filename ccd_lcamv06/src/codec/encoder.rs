use bytes::BytesMut;

pub trait Encoder {
    type Error: core::error::Error;
    type Item;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error>;
}
