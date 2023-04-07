use criterion::{criterion_group, criterion_main, Criterion};

fn rs_deep_update(c: &mut Criterion) {
    use reactive_signals::{
        runtimes::ClientRuntime, signal, types::Func, Signal,
    };

    c.bench_function("rs_deep_update", |b| {
        b.iter(|| {
            let sc = ClientRuntime::bench_root_scope();
            let signal = signal!(sc, 0);
            let mut memos = Vec::<Signal<Func<i32>, ClientRuntime>>::new();
            for i in 0..1000usize {
                let prev = memos.get(i.saturating_sub(1)).copied();
                if let Some(prev) = prev {
                    memos.push(signal!(sc, move || prev.get() + 1))
                } else {
                    memos.push(signal!(sc, move || signal.get() + 1))
                }
            }
            signal.set(1);
            assert_eq!(memos[999].get(), 1001);
        });
    });
}

fn l021_deep_update(c: &mut Criterion) {
    use l021::*;

    c.bench_function("l021_deep_update", |b| {
        let runtime = create_runtime();
        b.iter(|| {
            create_scope(runtime, |cx| {
                let signal = create_rw_signal(cx, 0);
                let mut memos = Vec::<Memo<usize>>::new();
                for i in 0..1000usize {
                    let prev = memos.get(i.saturating_sub(1)).copied();
                    if let Some(prev) = prev {
                        memos.push(create_memo(cx, move |_| prev.get() + 1));
                    } else {
                        memos.push(create_memo(cx, move |_| signal.get() + 1));
                    }
                }
                signal.set(1);
                assert_eq!(memos[999].get(), 1001);
            })
            .dispose()
        });
        runtime.dispose();
    });
}

fn sycamore_deep_update(c: &mut Criterion) {
    use sycamore::reactive::*;

    c.bench_function("sycamore_deep_update", |b| {
        b.iter(|| {
            let d = create_scope(|cx| {
                let signal = create_signal(cx, 0);
                let mut memos = Vec::<&ReadSignal<usize>>::new();
                for i in 0..1000usize {
                    let prev = memos.get(i.saturating_sub(1)).copied();
                    if let Some(prev) = prev {
                        memos.push(create_memo(cx, move || *prev.get() + 1));
                    } else {
                        memos.push(create_memo(cx, move || *signal.get() + 1));
                    }
                }
                signal.set(1);
                assert_eq!(*memos[999].get(), 1001);
            });
            unsafe { d.dispose() };
        });
    });
}

fn leptos_deep_update(c: &mut Criterion) {
    use leptos::*;
    let runtime = create_runtime();

    c.bench_function("leptos_deep_update", |b| {
        b.iter(|| {
            create_scope(runtime, |cx| {
                let signal = create_rw_signal(cx, 0);
                let mut memos = Vec::<Memo<usize>>::new();
                for i in 0..1000usize {
                    let prev = memos.get(i.saturating_sub(1)).copied();
                    if let Some(prev) = prev {
                        memos.push(create_memo(cx, move |_| prev.get() + 1));
                    } else {
                        memos.push(create_memo(cx, move |_| signal.get() + 1));
                    }
                }
                signal.set(1);
                assert_eq!(memos[999].get(), 1001);
            })
            .dispose()
        });
    });
    runtime.dispose();
}

criterion_group!(
    deep,
    rs_deep_update,
    l021_deep_update,
    sycamore_deep_update,
    leptos_deep_update
);
criterion_main!(deep);
