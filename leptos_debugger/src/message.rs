use serde::{Serialize, Deserialize};
use crate::{Prop, PropValue};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Component(ComponentMessage),
    DynChild(DynChildMessage),
    Each(EachMessage),
    Element(ElementMessage),
    Text(TextMessage),
    Unit(UnitMessage),

    Root(RootMessage),
    Props(PropsMessage),
    Signal(SignalMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ComponentMessage {
    Create {
        parent_id: String,
        id: String,
        name: String,
    },
    CleanupChildren(String),
    DeepCleanupChildren(String)
}

impl From<ComponentMessage> for Message{
    fn from(value: ComponentMessage) -> Self {
        Self::Component(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DynChildMessage {
    Create {
        parent_id: String,
        id: String,
    },
    DeepCleanupChildren(String)
}

impl From<DynChildMessage> for Message{
    fn from(value: DynChildMessage) -> Self {
        Self::DynChild(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EachMessage {
    Create { parent_id: String, id: String },
}

impl From<EachMessage> for Message{
    fn from(value: EachMessage) -> Self {
        Self::Each(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ElementMessage {
    Create { parent_id: String, id: String },
}

impl From<ElementMessage> for Message{
    fn from(value: ElementMessage) -> Self {
        Self::Element(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TextMessage {
    Create { parent_id: String, content: String },
}

impl From<TextMessage> for Message{
    fn from(value: TextMessage) -> Self {
        Self::Text(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum UnitMessage {
    Create { parent_id: String },
}

impl From<UnitMessage> for Message{
    fn from(value: UnitMessage) -> Self {
        Self::Unit(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RootMessage {
    Create { id: String },
}

impl From<RootMessage> for Message{
    fn from(value: RootMessage) -> Self {
        Self::Root(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PropsMessage {
    Create { id: String, props: Vec<Prop> },
}

impl From<PropsMessage> for Message{
    fn from(value: PropsMessage) -> Self {
        Self::Props(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignalMessage {
    Update { id: u64, value: PropValue },
    Cleanup(u64)
}

impl From<SignalMessage> for Message{
    fn from(value: SignalMessage) -> Self {
        Self::Signal(value)
    }
}