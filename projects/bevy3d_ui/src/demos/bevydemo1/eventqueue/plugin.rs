use super::events::*;
use bevy::prelude::*;

/// Events plugin for bevy
#[derive(Clone)]
pub struct DuplexEventsPlugin {
    /// Client processor for sending ClientInEvents, receiving PluginOutEvents
    client_processor: EventProcessor<ClientInEvents, PluginOutEvents>,
    /// Internal processor for sending PluginOutEvents, receiving ClientInEvents
    plugin_processor: EventProcessor<PluginOutEvents, ClientInEvents>,
}

impl DuplexEventsPlugin {
    /// Create a new instance
    pub fn new() -> DuplexEventsPlugin {
        // For sending messages from bevy to the client
        let (bevy_sender, client_receiver) = crossbeam_channel::bounded(50);
        // For sending message from the client to bevy
        let (client_sender, bevy_receiver) = crossbeam_channel::bounded(50);
        DuplexEventsPlugin {
            client_processor: EventProcessor {
                sender: client_sender,
                receiver: client_receiver,
            },
            plugin_processor: EventProcessor {
                sender: bevy_sender,
                receiver: bevy_receiver,
            },
        }
    }

    /// Get the client event processor
    pub fn get_processor(
        &self,
    ) -> EventProcessor<ClientInEvents, PluginOutEvents> {
        self.client_processor.clone()
    }
}

/// Build the bevy plugin and attach
impl Plugin for DuplexEventsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.plugin_processor.clone())
            .init_resource::<Events<CounterEvtData>>()
            .add_systems(PreUpdate, input_events_system);
    }
}

/// Send the event to bevy using EventWriter
fn input_events_system(
    int_processor: Res<EventProcessor<PluginOutEvents, ClientInEvents>>,
    mut counter_event_writer: EventWriter<CounterEvtData>,
) {
    for input_event in int_processor.receiver.try_iter() {
        match input_event {
            ClientInEvents::CounterEvt(event) => {
                // Send event through Bevy's event system
                counter_event_writer.send(event);
            }
        }
    }
}
