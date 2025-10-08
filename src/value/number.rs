#[derive(Debug)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
}
