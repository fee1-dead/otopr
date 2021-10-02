
#[derive(crate::EncodableMessage)]
pub struct Timestamp {
    #[otopr(1)]
    pub seconds: i64,
    #[otopr(2)]
    pub nanos: i32,
}