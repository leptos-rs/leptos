use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Prop {
    pub key: String,
    pub value: PropValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PropValue {
    None,
    Static(String),
    Vec(Vec<PropValue>),
    ReadSignal(u64),
    WriteSignal,
    RwSignal(u64),
}
