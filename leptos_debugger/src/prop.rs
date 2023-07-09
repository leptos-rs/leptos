#[derive(Debug, Clone)]
pub struct Prop {
    pub key: String,
    pub value: PropValue,
}

#[derive(Debug, Clone)]
pub enum PropValue {
    None,
    Static(String),
    ReadSignal(u64),
}
