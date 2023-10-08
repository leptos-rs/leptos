use leptos_macro::Lens;

#[derive(Lens)]
enum FailOnEnum {
    This(u32),
    Should(String),
    Panic,
}

#[derive(Lens)]
union FaulOnUnion {
    this: u32,
    should: std::mem::ManuallyDrop<String>,
    panic: std::mem::ManuallyDrop<Box<String>>,
}

fn main() {}
