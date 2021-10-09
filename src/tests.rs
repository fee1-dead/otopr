use crate::{decoding::{Decodable, DecodableMessage, Deserializer}, encoding::{Encodable, EncodableMessage, ProtobufSerializer}};

#[test]
fn test1() -> crate::decoding::Result<()> {
    // Taken from https://developers.google.com/protocol-buffers/docs/encoding#simple

    #[derive(crate::EncodableMessage, crate::DecodableMessage, Default)]
    struct Test1(#[otopr(1)] i32);

    let mut buf = Vec::with_capacity(3);
    EncodableMessage::encode(&Test1(150), &mut ProtobufSerializer::new(&mut buf));

    assert_eq!(&[0x08, 0x96, 0x01], buf.as_slice());

    let t: Test1 = DecodableMessage::decode(&mut Deserializer::new(&mut buf.as_slice()))?;
    assert_eq!(t.0, 150);
    Ok(())
}

#[test]
fn test2() {
    // https://developers.google.com/protocol-buffers/docs/encoding#strings

    #[derive(crate::EncodableMessage)]
    struct Test2<'a>(#[otopr(2)] &'a str);

    let mut buf = Vec::with_capacity(9);
    EncodableMessage::encode(&Test2("testing"), &mut (&mut buf).into());

    assert_eq!(
        &[0x12, 0x07, 0x74, 0x65, 0x73, 0x74, 0x69, 0x6e, 0x67],
        buf.as_slice()
    );
}

#[test]
fn test_varint() -> otopr::decoding::Result<()> {
    use otopr::VarInt;

    let num = u64::read(&mut Deserializer::new(
        &mut [
            0b1_1111111,
            0b1_1111111,
            0b1_1111111,
            0b1_1111111,
            0b1_1111111,
            0b1_1111111,
            0b1_1111111,
            0b1_1111111,
            0b1_1111111,
            0b0_0000001,
        ]
        .as_ref(),
    ))?;
    assert_eq!(num, u64::MAX);
    Ok(())
}

#[test]
fn test_enumeration() -> otopr::decoding::Result<()> {
    use otopr::Enumeration;

    #[derive(Enumeration, PartialEq, Eq, Debug)]
    enum Foo {
        Bar = 0,
        Baz = 1,
        Qux = 2,
    }

    assert_eq!(Foo::Bar, Foo::default());

    let mut buf = Vec::with_capacity(1);

    macro_rules! case {
        ($variant:ident = $num:expr) => {
            $num.encode(&mut (&mut buf).into());

            let case = Foo::decode(&mut (&mut buf.as_slice()).into())?;
        
            assert_eq!(Foo::$variant, case);
        
            buf.clear();
        }
    }

    case!(Bar = 0);
    case!(Baz = 1);
    case!(Qux = 2);

    // Fall back
    case!(Bar = 100);

    Ok(())
}