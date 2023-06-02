use criterion::{criterion_group, criterion_main, Criterion};

fn rs_fan_out(c: &mut Criterion) {
    use reactive_signals::{runtimes::ClientRuntime, signal};

    c.bench_function("rs_fan_out", |b| {
        b.iter(|| {
            let cx = ClientRuntime::bench_root_scope();
            let sig = signal!(cx, 0);
            let memos = (0..1000)
                .map(|_| signal!(cx, move || sig.get()))
                .collect::<Vec<_>>();
            assert_eq!(memos.iter().map(|m| m.get()).sum::<i32>(), 0);
            sig.set(1);
            assert_eq!(memos.iter().map(|m| m.get()).sum::<i32>(), 1000);
        });
    });
}

fn l021_fan_out(c: &mut Criterion) {
    use l021::*;

    c.bench_function("l021_fan_out", |b| {
        let runtime = create_runtime();
        b.iter(|| {
            create_scope(runtime, |cx| {
                let sig = create_rw_signal(cx, 0);
                let memos = (0..1000)
                    .map(|_| create_memo(cx, move |_| sig.get()))
                    .collect::<Vec<_>>();
                assert_eq!(memos.iter().map(|m| m.get()).sum::<i32>(), 0);
                sig.set(1);
                assert_eq!(memos.iter().map(|m| m.get()).sum::<i32>(), 1000);
            })
            .dispose()
        });
        runtime.dispose();
    });
}

fn sycamore_fan_out(c: &mut Criterion) {
    use sycamore::reactive::*;

    c.bench_function("sycamore_fan_out", |b| {
        b.iter(|| {
            let d = create_scope(|cx| {
                let sig = create_signal(cx, 0);
                let memos = (0..1000)
                    .map(|_| create_memo(cx, move || sig.get()))
                    .collect::<Vec<_>>();
                assert_eq!(memos.iter().map(|m| *(*m.get())).sum::<i32>(), 0);
                sig.set(1);
                assert_eq!(
                    memos.iter().map(|m| *(*m.get())).sum::<i32>(),
                    1000
                );
            });
            unsafe { d.dispose() };
        });
    });
}

fn leptos_fan_out(c: &mut Criterion) {
    use leptos_reactive::*;
    let runtime = create_runtime();

    c.bench_function("leptos_fan_out", |b| {
        b.iter(|| {
            create_scope(runtime, |cx| {
                let sig = create_rw_signal(cx, 0);
                let memos = (0..1000)
                    .map(|_| create_memo(cx, move |_| sig.get()))
                    .collect::<Vec<_>>();
                assert_eq!(memos.iter().map(|m| m.get()).sum::<i32>(), 0);
                sig.set(1);
                assert_eq!(memos.iter().map(|m| m.get()).sum::<i32>(), 1000);
            })
            .dispose()
        });
    });
    runtime.dispose();
}

criterion_group!(
    fan_out,
    rs_fan_out,
    l021_fan_out,
    sycamore_fan_out,
    leptos_fan_out
);
criterion_main!(fan_out);
