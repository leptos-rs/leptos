use gtk::{prelude::*, Application, ApplicationWindow, Orientation};
use leptos::{
    prelude::*,
    reactive_graph::{effect::Effect, owner::Owner, signal::RwSignal},
    Executor, For, ForProps,
};
use leptos_gtk::{button, r#box, Box_, LeptosGtk};
use std::{mem, thread, time::Duration};
mod leptos_gtk;

const APP_ID: &str = "dev.leptos.Counter";

// Basic GTK app setup from https://gtk-rs.org/gtk4-rs/stable/latest/book/hello_world.html
fn main() {
    // use the glib event loop to power the reactive system
    _ = Executor::init_glib();

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        let owner = Owner::new();
        let view = owner.with(ui);

        // Connect to "activate" signal of `app`
        let state = view.build();

        let window = ApplicationWindow::builder()
            .application(app)
            .title("TachyGTK")
            .child(&state.0 .0)
            .build();
        // Present window
        window.present();

        mem::forget((owner, state));
    });

    app.run();
}

fn ui() -> Box_<impl Render<LeptosGtk>> {
    let value = RwSignal::new(0);
    let rows = RwSignal::new(vec![1, 2, 3, 4, 5]);

    Effect::new(move |_| {
        println!("value = {}", value.get());
    });

    // just an example of multithreaded reactivity
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(250));
        value.update(|n| *n += 1);
    });

    vstack((
        hstack((
            button("-1", move |_| value.update(|n| *n -= 1)),
            move || value.get().to_string(),
            button("+1", move |_| value.update(|n| *n += 1)),
        )),
        button("Swap", move |_| {
            rows.update(|items| {
                items.swap(1, 3);
            })
        }),
        vstack(For(ForProps::builder()
            .each(move || rows.get())
            .key(|k| *k)
            .children(|v| v)
            .build())),
    ))
}

fn vstack(children: impl Render<LeptosGtk>) -> Box_<impl Render<LeptosGtk>> {
    r#box(Orientation::Vertical, 12, children)
}

fn hstack(children: impl Render<LeptosGtk>) -> impl Render<LeptosGtk> {
    r#box(Orientation::Horizontal, 12, children)
}
