use any_spawner::Executor;
use gtk::{prelude::*, Application, ApplicationWindow, Orientation};
use leptos::prelude::*;
use leptos_gtk::LeptosGtk;
use std::{mem, thread, time::Duration};
mod leptos_gtk;

const APP_ID: &str = "dev.leptos.Counter";

// Basic GTK app setup from https://gtk-rs.org/gtk4-rs/stable/latest/book/hello_world.html
fn main() {
    // use the glib event loop to power the reactive system
    _ = Executor::init_glib();
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| load_css());

    app.connect_activate(|app| {
        // Connect to "activate" signal of `app`
        let owner = Owner::new();
        let view = owner.with(ui);
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
            button("-1", move || {
                println!("clicked -1");
                value.update(|n| *n -= 1);
            }),
            move || value.get().to_string(),
            button("+1", move || value.update(|n| *n += 1)),
        )),
        button("Swap", move || {
            rows.update(|items| {
                items.swap(1, 3);
            })
        }),
        hstack(rows),
    ))
}

fn button(
    label: impl Render<LeptosGtk>,
    callback: impl Fn() + Send + Sync + 'static,
) -> impl Render<LeptosGtk> {
    leptos_gtk::button()
        .child(label)
        .connect("clicked", move |_| {
            callback();
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

fn load_css() {
    use gtk::{gdk::Display, CssProvider};

    let provider = CssProvider::new();
    provider.load_from_path("style.css");

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
