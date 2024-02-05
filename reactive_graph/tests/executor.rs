use reactive_graph::executor::Executor;
use std::rc::Rc;

#[cfg(feature = "futures-executor")]
#[test]
fn futures_executor_test() {
    Executor::init_futures_executor().expect("couldn't set executor");
    let rc = Rc::new(());
    Executor::spawn_local(async {
        _ = rc;
    });
    Executor::spawn(async {});
}
