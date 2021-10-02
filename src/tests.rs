use crate::{encoding::ProtobufSerializer, traits::EncodableMessage};

#[test]
fn test1() {
    // Taken from https://developers.google.com/protocol-buffers/docs/encoding#simple

    #[derive(crate::EncodableMessage)]
    struct Test1(#[otopr(1)] i32);

    let mut buf = Vec::with_capacity(3);
    Test1(150).encode(&mut ProtobufSerializer::new(&mut buf));

    assert_eq!(&[0x08, 0x96, 0x01], buf.as_slice());
}