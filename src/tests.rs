use crate::{decoding::{DecodableMessage, Deserializer}, encoding::ProtobufSerializer, traits::EncodableMessage};

#[test]
fn test1() -> crate::decoding::Result<()> {
    // Taken from https://developers.google.com/protocol-buffers/docs/encoding#simple

    #[derive(crate::EncodableMessage, crate::DecodableMessage, Default)]
    struct Test1(#[otopr(1)] i32);

    let mut buf = Vec::with_capacity(3);
    Test1(150).encode(&mut ProtobufSerializer::new(&mut buf));

    assert_eq!(&[0x08, 0x96, 0x01], buf.as_slice());

    let t = Test1::decode(&mut Deserializer::new(&mut buf.as_slice()))?;
    assert_eq!(t.0, 150);
    Ok(())
}

#[test]
fn test2() {
    // https://developers.google.com/protocol-buffers/docs/encoding#strings

    #[derive(crate::EncodableMessage)]
    struct Test2<'a>(#[otopr(2)] &'a str);

    let mut buf = Vec::with_capacity(9);
    Test2("testing").encode(&mut ProtobufSerializer::new(&mut buf));

    assert_eq!(&[0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67], buf.as_slice());
}