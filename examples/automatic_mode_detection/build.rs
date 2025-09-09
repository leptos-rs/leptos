use leptos_compile_validator::validate_with_context;

fn main() {
    println!("cargo:rerun-if-env-changed=LEPTOS_MODE");
    println!("cargo:rerun-if-env-changed=LEPTOS_TARGET");
    
    // Perform enhanced validation with context awareness
    let validation_result = validate_with_context();
    if !validation_result.is_empty() {
        panic!("Leptos validation failed");
    }
}
