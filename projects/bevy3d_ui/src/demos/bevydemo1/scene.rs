use super::eventqueue::events::{
    ClientInEvents, CounterEvtData, EventProcessor, PluginOutEvents,
};
use super::eventqueue::plugin::DuplexEventsPlugin;
use super::state::{Shared, SharedResource, SharedState};
use bevy::prelude::*;

/// Represents the Cube in the scene
#[derive(Component, Copy, Clone)]
pub struct Cube;

/// Represents the 3D Scene
#[derive(Clone)]
pub struct Scene {
    is_setup: bool,
    canvas_id: String,
    evt_plugin: DuplexEventsPlugin,
    shared_state: Shared<SharedState>,
    processor: EventProcessor<ClientInEvents, PluginOutEvents>,
}

impl Scene {
    /// Create a new instance
    pub fn new(canvas_id: String) -> Scene {
        let plugin = DuplexEventsPlugin::new();
        Scene {
            is_setup: false,
            canvas_id,
            evt_plugin: plugin.clone(),
            shared_state: SharedState::new(),
            processor: plugin.get_processor(),
        }
    }

    /// Get the shared state
    pub fn get_state(&self) -> Shared<SharedState> {
        self.shared_state.clone()
    }

    /// Get the event processor
    pub fn get_processor(
        &self,
    ) -> EventProcessor<ClientInEvents, PluginOutEvents> {
        self.processor.clone()
    }

    /// Setup and attach the bevy instance to the html canvas element
    pub fn setup(&mut self) {
        if self.is_setup {
            return;
        };
        App::new()
            .add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    canvas: Some(self.canvas_id.clone()),
                    ..default()
                }),
                ..default()
            }))
            .add_plugins(self.evt_plugin.clone())
            .insert_resource(SharedResource(self.shared_state.clone()))
            .add_systems(Startup, setup_scene)
            .add_systems(Update, handle_bevy_event)
            .run();
        self.is_setup = true;
    }
}

/// Setup the scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    resource: Res<SharedResource>,
) {
    let name = resource.0.lock().unwrap().name.clone();
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(
            -std::f32::consts::FRAC_PI_2,
        )),
    ));

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Cube,
    ));

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((Text::new(name), TextFont::default()));
}

/// Move the Cube on event
fn handle_bevy_event(
    mut counter_event_reader: EventReader<CounterEvtData>,
    mut cube_query: Query<&mut Transform, With<Cube>>,
) {
    let mut cube_transform = cube_query.get_single_mut().expect("no cube :(");
    for _ev in counter_event_reader.read() {
        cube_transform.translation += Vec3::new(0.0, _ev.value, 0.0);
    }
}
