use leptos_reactive::{Callable, Callback, SyncCallback};

#[test]
fn call_test(){
    let callback = Callback::new(|x: i32| x*2);
    assert!(callback.call(4)==8);
}
