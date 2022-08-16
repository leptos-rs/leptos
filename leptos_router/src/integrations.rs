use leptos_reactive::{ReadSignal, Scope, WriteSignal};

use crate::LocationChange;

pub trait Integration {
    fn normalize(&self, cx: Scope) -> (ReadSignal<LocationChange>, WriteSignal<LocationChange>) {
        todo!()
    }
}

pub struct ServerIntegration {}

impl Integration for ServerIntegration {}

pub struct HashIntegration {}

impl Integration for HashIntegration {}

pub struct HistoryIntegration {}

impl Integration for HistoryIntegration {}
