use test::Bencher;

use std::{cell::Cell, rc::Rc};
use leptos::*;

#[bench]
fn leptos_deep_memo_creation(b: &mut Bencher) {
    let runtime = create_runtime();

    b.iter(|| {
        create_scope(runtime, |cx| {
            let signal = create_rw_signal(cx, 0);
            let mut memos = Vec::<Memo<usize>>::new();
            for i in 0..1000usize {
                let prev = memos.get(i.saturating_sub(1)).copied();
                if let Some(prev) = prev {
                    memos.push(create_memo(cx, move |_| {
                        prev.get()
                    }));
                } else {
                    memos.push(create_memo(cx, move |_| {
                        signal.get()
                    }));
                }
            }
        })
        .dispose()
    });

    runtime.dispose();
}

#[bench]
fn leptos_deep_memo_update(b: &mut Bencher) {
    let runtime = create_runtime();

    b.iter(|| {
        create_scope(runtime, |cx| {
            let signal = create_rw_signal(cx, 0);
            let mut memos = Vec::<Memo<usize>>::new();
            for i in 0..1000usize {
                let prev = memos.get(i.saturating_sub(1)).copied();
                if let Some(prev) = prev {
                    memos.push(create_memo(cx, move |_| {
                        prev.get()
                    }));
                } else {
                    memos.push(create_memo(cx, move |_| {
                        signal.get()
                    }));
                }
            }
            signal.set(1);
            assert_eq!(memos[999].get(), 1);
        })
        .dispose()
    });

    runtime.dispose();
}

#[bench]
fn leptos_create_1000_signals(b: &mut Bencher) {

    let runtime = create_runtime();

    b.iter(|| {
        create_scope(runtime, |cx| {
            let acc = Rc::new(Cell::new(0));
            let sigs = (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
            let reads = sigs.iter().map(|(r, _)| *r).collect::<Vec<_>>();
            let writes = sigs.iter().map(|(_, w)| *w).collect::<Vec<_>>();
            let memo = create_memo(cx, move |_| reads.iter().map(|r| r.get()).sum::<i32>());
            assert_eq!(memo(), 499500);
        })
        .dispose()
    });

    runtime.dispose();
}

#[bench]
fn leptos_create_and_update_1000_signals(b: &mut Bencher) {
    let runtime = create_runtime();

    b.iter(|| {
        create_scope(runtime, |cx| {
            let acc = Rc::new(Cell::new(0));
            let sigs = (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
            let reads = sigs.iter().map(|(r, _)| *r).collect::<Vec<_>>();
            let writes = sigs.iter().map(|(_, w)| *w).collect::<Vec<_>>();
            let memo = create_memo(cx, move |_| reads.iter().map(|r| r.get()).sum::<i32>());
            assert_eq!(memo(), 499500);
            create_isomorphic_effect(cx, {
                let acc = Rc::clone(&acc);
                move |_| {
                    acc.set(memo());
                }
            });
            assert_eq!(acc.get(), 499500);

            writes[1].update(|n| *n += 1);
            writes[10].update(|n| *n += 1);
            writes[100].update(|n| *n += 1);

            assert_eq!(acc.get(), 499503);
            assert_eq!(memo(), 499503);
        })
        .dispose()
    });

    runtime.dispose();
}

#[bench]
fn leptos_create_and_dispose_1000_scopes(b: &mut Bencher) {
    let runtime = create_runtime();

    b.iter(|| {
        let acc = Rc::new(Cell::new(0));
        let disposers = (0..1000)
            .map(|_| {
                create_scope(runtime, {
                    let acc = Rc::clone(&acc);
                    move |cx| {
                        let (r, w) = create_signal(cx, 0);
                        create_isomorphic_effect(cx, {
                            move |_| {
                                acc.set(r());
                            }
                        });
                        w.update(|n| *n += 1);
                    }
                })
            })
            .collect::<Vec<_>>();
        for disposer in disposers {
            disposer.dispose();
        }
    });

    runtime.dispose();
}

#[bench]
fn sycamore_create_1000_signals(b: &mut Bencher) {
    use sycamore::reactive::{create_effect, create_memo, create_scope, create_signal};

    b.iter(|| {
        let d = create_scope(|cx| {
            let acc = Rc::new(Cell::new(0));
            let sigs = Rc::new((0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>());
            let memo = create_memo(cx, {
                let sigs = Rc::clone(&sigs);
                move || sigs.iter().map(|r| *r.get()).sum::<i32>()
            });
            assert_eq!(*memo.get(), 499500);
        });
        unsafe { d.dispose() };
    });
}

#[bench]
fn sycamore_deep_memo_creation(b: &mut Bencher) {
    use sycamore::reactive::*;

    b.iter(|| {
        let d = create_scope(|cx| {
            let signal = create_signal(cx, 0);
            let mut memos = Vec::<&ReadSignal<usize>>::new();
            for i in 0..1000usize {
                let prev = memos.get(i.saturating_sub(1)).copied();
                if let Some(prev) = prev {
                    memos.push(create_memo(cx, move || {
                        *prev.get()
                    }));
                } else {
                    memos.push(create_memo(cx, move || {
                        *signal.get()
                    }));
                }
            }
        });
        unsafe { d.dispose() };
    });
}

#[bench]
fn sycamore_deep_memo_update(b: &mut Bencher) {
    use sycamore::reactive::*;

    b.iter(|| {
        let d = create_scope(|cx| {
            let signal = create_signal(cx, 0);
            let mut memos = Vec::<&ReadSignal<usize>>::new();
            for i in 0..1000usize {
                let prev = memos.get(i.saturating_sub(1)).copied();
                if let Some(prev) = prev {
                    memos.push(create_memo(cx, move || {
                        *prev.get()
                    }));
                } else {
                    memos.push(create_memo(cx, move || {
                        *signal.get()
                    }));
                }
            }
            signal.set(1);
            assert_eq!(*memos[999].get(), 1);
        });
        unsafe { d.dispose() };
    });
}
#[bench]
fn sycamore_create_and_update_1000_signals(b: &mut Bencher) {
    use sycamore::reactive::{create_effect, create_memo, create_scope, create_signal};

    b.iter(|| {
        let d = create_scope(|cx| {
            let acc = Rc::new(Cell::new(0));
            let sigs = Rc::new((0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>());
            let memo = create_memo(cx, {
                let sigs = Rc::clone(&sigs);
                move || sigs.iter().map(|r| *r.get()).sum::<i32>()
            });
            assert_eq!(*memo.get(), 499500);
            create_effect(cx, {
                let acc = Rc::clone(&acc);
                move || {
                    acc.set(*memo.get());
                }
            });
            assert_eq!(acc.get(), 499500);

            sigs[1].set(*sigs[1].get() + 1);
            sigs[10].set(*sigs[10].get() + 1);
            sigs[100].set(*sigs[100].get() + 1);

            assert_eq!(acc.get(), 499503);
            assert_eq!(*memo.get(), 499503);
        });
        unsafe { d.dispose() };
    });
}

#[bench]
fn sycamore_create_and_dispose_1000_scopes(b: &mut Bencher) {
    use sycamore::reactive::{create_effect, create_scope, create_signal};

    b.iter(|| {
        let acc = Rc::new(Cell::new(0));
        let disposers = (0..1000)
            .map(|_| {
                create_scope({
                    let acc = Rc::clone(&acc);
                    move |cx| {
                        let s = create_signal(cx, 0);
                        create_effect(cx, {
                            move || {
                                acc.set(*s.get());
                            }
                        });
                        s.set(*s.get() + 1);
                    }
                })
            })
            .collect::<Vec<_>>();
        for disposer in disposers {
            unsafe {
                disposer.dispose();
            }
        }
    });
}
