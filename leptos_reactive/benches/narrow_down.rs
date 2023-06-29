use criterion::{criterion_group, criterion_main, Criterion};
use std::{cell::Cell, rc::Rc};

fn rs_narrow_down(c: &mut Criterion) {
    use reactive_signals::{runtimes::ClientRuntime, signal};

    c.bench_function("rs_narrow_down", |b| {
        b.iter(|| {
            let cx = ClientRuntime::bench_root_scope();
            let sigs =
                Rc::new((0..1000).map(|n| signal!(cx, n)).collect::<Vec<_>>());
            let memo = signal!(cx, {
                let sigs = Rc::clone(&sigs);
                move || sigs.iter().map(|r| r.get()).sum::<i32>()
            });
            assert_eq!(memo.get(), 499500);
        });
    });
}

fn l021_narrow_down(c: &mut Criterion) {
    use l021::*;

    c.bench_function("l021_narrow_down", |b| {
        let runtime = create_runtime();
        b.iter(|| {
            create_scope(runtime, |cx| {
                let sigs =
                    (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
                let reads = sigs.iter().map(|(r, _)| *r).collect::<Vec<_>>();
                let memo = create_memo(cx, move |_| {
                    reads.iter().map(|r| r.get()).sum::<i32>()
                });
                assert_eq!(memo(), 499500);
            })
            .dispose()
        });
        runtime.dispose();
    });
}

fn sycamore_narrow_down(c: &mut Criterion) {
    use sycamore::reactive::*;

    c.bench_function("sycamore_narrow_down", |b| {
        b.iter(|| {
            let d = create_scope(|cx| {
                let sigs = Rc::new(
                    (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>(),
                );
                let memo = create_memo(cx, {
                    let sigs = Rc::clone(&sigs);
                    move || sigs.iter().map(|r| *r.get()).sum::<i32>()
                });
                assert_eq!(*memo.get(), 499500);
            });
            unsafe { d.dispose() };
        });
    });
}

fn leptos_narrow_down(c: &mut Criterion) {
    use leptos_reactive::*;
    let runtime = create_runtime();

    c.bench_function("leptos_narrow_down", |b| {
        b.iter(|| {
            create_scope(runtime, |cx| {
                let sigs =
                    (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
                let reads = sigs.iter().map(|(r, _)| *r).collect::<Vec<_>>();
                let memo = create_memo(cx, move |_| {
                    reads.iter().map(|r| r.get()).sum::<i32>()
                });
                assert_eq!(memo(), 499500);
            })
            .dispose()
        });
    });
    runtime.dispose();
}

criterion_group!(
    narrow_down,
    rs_narrow_down,
    l021_narrow_down,
    sycamore_narrow_down,
    leptos_narrow_down
);
criterion_main!(narrow_down);
