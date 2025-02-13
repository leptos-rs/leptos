use crate::demos::bevydemo1::eventqueue::events::{
    ClientInEvents, CounterEvtData,
};
use crate::demos::bevydemo1::scene::Scene;
use leptos::prelude::*;

/// 3d view component
#[component]
pub fn Demo1() -> impl IntoView {
    // Setup a Counter
    let initial_value: i32 = 0;
    let step: i32 = 1;
    let (value, set_value) = signal(initial_value);

    // Setup a bevy 3d scene
    let scene = Scene::new("#bevy".to_string());
    let sender = scene.get_processor().sender;
    let (sender_sig, _set_sender_sig) = signal(sender);
    let (scene_sig, _set_scene_sig) = signal(scene);

    // We need to add the 3D view onto the canvas post render.
    Effect::new(move |_| {
        request_animation_frame(move || {
            scene_sig.get_untracked().setup();
        });
    });

    view! {
        <div>
            <button on:click=move |_| set_value.set(0)>"Clear"</button>
            <button on:click=move |_| {
                set_value.update(|value| *value -= step);
                let newpos = (step as f32) / 10.0;
                sender_sig
                    .get()
                    .send(ClientInEvents::CounterEvt(CounterEvtData { value: -newpos }))
                    .expect("could not send event");
            }>"-1"</button>
            <span>"Value: " {value} "!"</span>
            <button on:click=move |_| {
                set_value.update(|value| *value += step);
                let newpos = step as f32 / 10.0;
                sender_sig
                    .get()
                    .send(ClientInEvents::CounterEvt(CounterEvtData { value: newpos }))
                    .expect("could not send event");
            }>"+1"</button>
        </div>

        <canvas id="bevy" width="800" height="600"></canvas>
    }
}
