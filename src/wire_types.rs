use crate::traits::private::Sealed;

/// A WireType.
pub trait WireType: Sealed {
    const BITS: u8;
}

macro_rules! wires {
    ($($id:ident = $bits:expr),*$(,)?) => {$(
        seal!(for $id);
        pub struct $id;
        impl WireType for $id {
            const BITS: u8 = $bits;
        }
    )*};
}

wires! {
    VarIntWire = 0,
    Fixed64Wire = 1,
    LengthDelimitedWire = 2,
    // unsupported wires
    // StartGroupWire = 3,
    // EndGroupWire = 4,
    Fixed32Wire = 5,
}
