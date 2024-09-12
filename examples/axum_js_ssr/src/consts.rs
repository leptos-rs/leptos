// Example programs from the Rust Programming Language Book

pub const CH03_05A: &str = r#"fn main() {
    let number = 3;

    if number < 5 {
        println!("condition was true");
    } else {
        println!("condition was false");
    }
}




"#;

// For some reason, swapping the code examples "fixes" example 6.  It
// might have something to do with the lower complexity of highlighting
// a shorter example.  Anyway, including extra newlines for the shorter
// example to match with the longer in order to avoid reflowing the
// table during the async resource loading for CSR.

pub const CH05_02A: &str = r#"fn main() {
    let width1 = 30;
    let height1 = 50;

    println!(
        "The area of the rectangle is {} square pixels.",
        area(width1, height1)
    );
}

fn area(width: u32, height: u32) -> u32 {
    width * height
}
"#;

pub const LEPTOS_HYDRATED: &str = "_leptos_hydrated";
