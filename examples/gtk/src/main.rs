use gtk::{
    glib::Value, prelude::*, Application, ApplicationWindow, Orientation,
    Widget,
};
use leptos::{
    prelude::*,
    reactive_graph::{effect::Effect, owner::Owner, signal::RwSignal},
    Executor, For, ForProps,
};
#[cfg(feature = "gtk")]
use leptos_gtk::{Element, LGtkWidget, LeptosGtk};
use std::{mem, thread, time::Duration};
#[cfg(feature = "gtk")]
mod leptos_gtk;

const APP_ID: &str = "dev.leptos.Counter";

// Basic GTK app setup from https://gtk-rs.org/gtk4-rs/stable/latest/book/hello_world.html
fn main() {
    // use the glib event loop to power the reactive system
    #[cfg(feature = "gtk")]
    {
        _ = Executor::init_glib();
        let app = Application::builder().application_id(APP_ID).build();

        app.connect_activate(|app| {
            let owner = Owner::new();
            let view = owner.with(ui);

            // Connect to "activate" signal of `app`
            let (root, state) = leptos_gtk::root(view);

            let window = ApplicationWindow::builder()
                .application(app)
                .title("TachyGTK")
                .child(&root)
                .build();
            // Present window
            window.present();

            mem::forget((owner, state));
        });

        app.run();
    }

    #[cfg(feature = "wasm")]
    {
        _ = Executor::init_wasm_bindgen();
    }
}

fn ui() -> impl Render<LeptosGtk> {
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

fn button(
    label: impl Render<LeptosGtk>,
    callback: impl Fn(&[Value]) + Send + Sync + 'static,
) -> impl Render<LeptosGtk> {
    leptos_gtk::button()
        .child(label)
        .connect("clicked", move |value| {
            callback(value);
            None
        })
}

fn vstack(children: impl Render<LeptosGtk>) -> impl Render<LeptosGtk> {
    leptos_gtk::r#box()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .child(children)
}

fn hstack(children: impl Render<LeptosGtk>) -> impl Render<LeptosGtk> {
    leptos_gtk::r#box()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .child(children)
}
