#[cfg(feature = "gtk")]
use gtk::{
    glib::Value, prelude::*, Application, ApplicationWindow, Orientation,
    Widget,
};
#[cfg(feature = "wasm")]
use leptos::tachys::{dom::body, html::element, html::event as ev};
use leptos::{
    logging,
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

    #[cfg(all(feature = "wasm", not(feature = "gtk")))]
    {
        console_error_panic_hook::set_once();
        _ = Executor::init_wasm_bindgen();
        let owner = Owner::new();
        let view = owner.with(ui);
        let mut state = view.build();
        state.mount(&body().into(), None);
        mem::forget((owner, state));
    }
}

#[cfg(feature = "gtk")]
type Rndr = LeptosGtk;
#[cfg(all(feature = "wasm", not(feature = "gtk")))]
type Rndr = Dom;

fn ui() -> impl Render<Rndr> {
    let value = RwSignal::new(0);
    let rows = RwSignal::new(vec![1, 2, 3, 4, 5]);

    Effect::new(move |_| {
        logging::log!("value = {}", value.get());
    });

    // just an example of multithreaded reactivity
    #[cfg(feature = "gtk")]
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(250));
        value.update(|n| *n += 1);
    });

    vstack((
        hstack((
            button("-1", move || value.update(|n| *n -= 1)),
            move || value.get().to_string(),
            button("+1", move || value.update(|n| *n += 1)),
        )),
        button("Swap", move || {
            rows.update(|items| {
                items.swap(1, 3);
            })
        }),
        hstack(For(ForProps::builder()
            .each(move || rows.get())
            .key(|k| *k)
            .children(|v| v)
            .build())),
    ))
}

fn button(
    label: impl Render<Rndr>,
    callback: impl Fn() + Send + Sync + 'static,
) -> impl Render<Rndr> {
    #[cfg(feature = "gtk")]
    {
        leptos_gtk::button()
            .child(label)
            .connect("clicked", move |_| {
                callback();
                None
            })
    }
    #[cfg(all(feature = "wasm", not(feature = "gtk")))]
    {
        element::button()
            .on(ev::click, move |_| callback())
            .child(label)
    }
}

fn vstack(children: impl Render<Rndr>) -> impl Render<Rndr> {
    #[cfg(feature = "gtk")]
    {
        leptos_gtk::r#box()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .child(children)
    }
    #[cfg(all(feature = "wasm", not(feature = "gtk")))]
    {
        element::div()
            .style(("display", "flex"))
            .style(("flex-direction", "column"))
            .style(("align-items", "center"))
            .style(("justify-content", "center"))
            .style(("margin", "1rem"))
            .child(children)
    }
}

fn hstack(children: impl Render<Rndr>) -> impl Render<Rndr> {
    #[cfg(feature = "gtk")]
    {
        leptos_gtk::r#box()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .child(children)
    }
    #[cfg(all(feature = "wasm", not(feature = "gtk")))]
    {
        element::div()
            .style(("display", "flex"))
            .style(("align-items", "center"))
            .style(("justify-content", "center"))
            .style(("margin", "1rem"))
            .child(children)
    }
}

#[cfg(feature = "gtk")]
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
