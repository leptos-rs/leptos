use bevy::prelude::*;

/// Event Processor
#[derive(Resource)]
pub struct EventProcessor<TSender, TReceiver> {
    pub sender: crossbeam_channel::Sender<TSender>,
    pub receiver: crossbeam_channel::Receiver<TReceiver>,
}

impl<TSender, TReceiver> Clone for EventProcessor<TSender, TReceiver> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

/// Events sent from the client to bevy
#[derive(Debug)]
pub enum ClientInEvents {
    /// Update the 3d model position from the client
    CounterEvt(CounterEvtData),
}

/// Events sent out from bevy to the client
#[derive(Debug)]
pub enum PluginOutEvents {
    /// TODO Feed back to the client an event from bevy
    Click,
}

/// Input event to update the bevy view from the client
#[derive(Clone, Debug, Event)]
pub struct CounterEvtData {
    /// Amount to move on the Y Axis
    pub value: f32,
}
