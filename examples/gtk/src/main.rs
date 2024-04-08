use gtk::{prelude::*, Application, ApplicationWindow, Button};
use leptos::*;

const APP_ID: &str = "dev.leptos.Counter";

// Basic GTK app setup from https://gtk-rs.org/gtk4-rs/stable/latest/book/hello_world.html
fn main() {
    let _ = create_runtime();
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run();
}

fn build_ui(app: &Application) {
    let button = counter_button();

    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Leptos-GTK")
        .child(&button)
        .build();

    // Present window
    window.present();
}

fn counter_button() -> Button {
    let (value, set_value) = create_signal(0);

    // Create a button with label and margins
    let button = Button::builder()
        .label("Count: ")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Connect to "clicked" signal of `button`
    button.connect_clicked(move |_| {
        // Set the label to "Hello World!" after the button has been clicked on
        set_value.update(|value| *value += 1);
    });

    create_effect({
        let button = button.clone();
        move |_| {
            button.set_label(&format!("Count: {}", value.get()));
        }
    });

    button
}
