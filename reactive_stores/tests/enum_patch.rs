use reactive_graph::{
    effect::Effect,
    traits::{Get, Set},
};
use reactive_stores::{Patch, PatchField, Store, StorePath};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, PartialEq, Store, Patch)]
enum State {
    Idle,
    Pair(i32, String),
    Ready { title: String, count: i32 },
}

fn path(segments: &[usize]) -> StorePath {
    segments.iter().map(|segment| (*segment).into()).collect()
}

fn patch_paths<T: PatchField>(old: &mut T, new: T) -> Vec<StorePath> {
    let mut paths = Vec::new();
    old.patch_field(
        new,
        &path(&[7]),
        &mut |path| paths.push(path.clone()),
        None,
    );
    paths
}

#[test]
fn matching_variants_only_notify_changed_fields() {
    let mut state = State::Idle;
    assert!(patch_paths(&mut state, State::Idle).is_empty());

    state = State::Ready {
        title: "hello".into(),
        count: 1,
    };
    assert!(
        patch_paths(
            &mut state,
            State::Ready {
                title: "hello".into(),
                count: 1
            }
        )
        .is_empty()
    );
    assert_eq!(
        patch_paths(
            &mut state,
            State::Ready {
                title: "hello".into(),
                count: 2
            }
        ),
        vec![path(&[7, 1])]
    );
    assert_eq!(
        patch_paths(
            &mut state,
            State::Ready {
                title: "bye".into(),
                count: 2
            }
        ),
        vec![path(&[7, 0])]
    );
    assert_eq!(
        state,
        State::Ready {
            title: "bye".into(),
            count: 2
        }
    );

    state = State::Pair(1, "hello".into());
    assert!(patch_paths(&mut state, State::Pair(1, "hello".into())).is_empty());
    assert_eq!(
        patch_paths(&mut state, State::Pair(1, "bye".into())),
        vec![path(&[7, 1])]
    );
    assert_eq!(
        patch_paths(&mut state, State::Pair(2, "bye".into())),
        vec![path(&[7, 0])]
    );
    assert_eq!(state, State::Pair(2, "bye".into()));
}

#[test]
fn different_variants_notify_the_enum_path() {
    let mut state = State::Idle;
    for next in [
        State::Pair(1, "hello".into()),
        State::Ready {
            title: "hello".into(),
            count: 1,
        },
        State::Idle,
    ] {
        assert_eq!(patch_paths(&mut state, next), vec![path(&[7])]);
    }
    assert_eq!(state, State::Idle);
}

#[derive(Store, Patch)]
struct Outer {
    state: State,
    unrelated: i32,
}

#[test]
fn nested_enum_uses_its_own_path() {
    let mut value = Outer {
        state: State::Pair(1, "hello".into()),
        unrelated: 0,
    };
    assert_eq!(
        patch_paths(
            &mut value,
            Outer {
                state: State::Pair(2, "hello".into()),
                unrelated: 0
            }
        ),
        vec![path(&[7, 0, 0])]
    );
    assert_eq!(
        patch_paths(
            &mut value,
            Outer {
                state: State::Idle,
                unrelated: 0
            }
        ),
        vec![path(&[7, 0])]
    );
}

#[test]
fn generic_payloads_need_neither_clone_nor_whole_enum_equality() {
    #[derive(Patch)]
    struct Payload {
        value: i32,
    }

    #[derive(Patch)]
    enum Generic<T = Payload>
    where
        T: PatchField,
    {
        Value(T),
        Empty,
    }

    let mut value = Generic::Value(Payload { value: 1 });
    assert_eq!(
        patch_paths(&mut value, Generic::Value(Payload { value: 2 })),
        vec![path(&[7, 0, 0])]
    );
    assert_eq!(patch_paths(&mut value, Generic::Empty), vec![path(&[7])]);
}

#[test]
fn custom_patch_and_field_names_do_not_conflict_with_generated_bindings() {
    #[derive(Debug, PartialEq)]
    struct Custom(String);

    #[derive(Patch)]
    enum CustomState {
        Named {
            #[patch(|this, new| this.0 = new.0.to_uppercase())]
            path: Custom,
            new: i32,
            notify: i32,
            keys: i32,
            new_path: i32,
            __patch_old_0: i32,
        },
    }
    let make = |text: &str| CustomState::Named {
        path: Custom(text.into()),
        new: 1,
        notify: 2,
        keys: 3,
        new_path: 4,
        __patch_old_0: 5,
    };
    let mut value = make("FIRST");
    assert!(patch_paths(&mut value, make("FIRST")).is_empty());
    assert_eq!(patch_paths(&mut value, make("second")), vec![path(&[7, 0])]);
    let CustomState::Named { path: value, .. } = value;
    assert_eq!(value.0, "SECOND");
}

#[test]
fn empty_shapes_and_explicit_discriminants_are_supported() {
    #[derive(Patch)]
    enum Empty {}
    fn assert_patch<T: PatchField>() {}
    assert_patch::<Empty>();

    #[derive(Patch)]
    enum Shapes {
        Unit,
        Named {},
        Tuple(),
    }
    let mut value = Shapes::Named {};
    assert!(patch_paths(&mut value, Shapes::Named {}).is_empty());
    assert_eq!(patch_paths(&mut value, Shapes::Tuple()), vec![path(&[7])]);
    assert!(patch_paths(&mut value, Shapes::Tuple()).is_empty());
    assert_eq!(patch_paths(&mut value, Shapes::Unit), vec![path(&[7])]);

    #[derive(Patch)]
    enum Discriminants {
        First = 10,
        Second = 20,
    }
    let mut value = Discriminants::First;
    assert!(patch_paths(&mut value, Discriminants::First).is_empty());
    assert_eq!(
        patch_paths(&mut value, Discriminants::Second),
        vec![path(&[7])]
    );
}

async fn tick() {
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
}

fn observe_fn<T>(
    read: impl Fn() -> T + Send + Sync + 'static,
) -> Arc<AtomicUsize> {
    let count = Arc::new(AtomicUsize::new(0));
    Effect::new_sync({
        let count = Arc::clone(&count);
        move |_| {
            _ = read();
            count.fetch_add(1, Ordering::Relaxed);
        }
    });
    count
}

fn observe<T: Get + Send + Sync + 'static>(field: T) -> Arc<AtomicUsize> {
    let count = Arc::new(AtomicUsize::new(0));
    Effect::new_sync({
        let count = Arc::clone(&count);
        move |_| {
            _ = field.get();
            count.fetch_add(1, Ordering::Relaxed);
        }
    });
    count
}

#[tokio::test]
async fn named_field_handles_are_independent_for_patches_and_writes() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(State::Ready {
        title: "hello".into(),
        count: 1,
    });
    let title = store.ready_title().unwrap();
    let count = store.ready_count().unwrap();
    let title_runs = observe(title);
    let count_runs = observe(count);
    tick().await;

    store.patch(State::Ready {
        title: "hello".into(),
        count: 2,
    });
    tick().await;
    assert_eq!(title_runs.load(Ordering::Relaxed), 1);
    assert_eq!(count_runs.load(Ordering::Relaxed), 2);
    assert_eq!(count.get(), 2);

    title.set("bye".into());
    tick().await;
    assert_eq!(title_runs.load(Ordering::Relaxed), 2);
    assert_eq!(count_runs.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn tuple_field_handles_are_independent_for_patches_and_writes() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(State::Pair(1, "hello".into()));
    let first = store.pair_0().unwrap();
    let second = store.pair_1().unwrap();
    let first_runs = observe(first);
    let second_runs = observe(second);
    tick().await;

    store.patch(State::Pair(1, "bye".into()));
    tick().await;
    assert_eq!(first_runs.load(Ordering::Relaxed), 1);
    assert_eq!(second_runs.load(Ordering::Relaxed), 2);
    assert_eq!(second.get(), "bye");

    first.set(2);
    tick().await;
    assert_eq!(first_runs.load(Ordering::Relaxed), 2);
    assert_eq!(second_runs.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn variant_changes_recheck_accessors_without_notifying_outer_siblings() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(Outer {
        state: State::Idle,
        unrelated: 0,
    });
    let unrelated_runs = observe(store.unrelated());
    let accessor_runs = Arc::new(AtomicUsize::new(0));
    Effect::new_sync({
        let runs = Arc::clone(&accessor_runs);
        move |_| {
            if let Some(count) = store.state().ready_count() {
                _ = count.get();
            }
            runs.fetch_add(1, Ordering::Relaxed);
        }
    });
    tick().await;
    store.patch(Outer {
        state: State::Ready {
            title: "hello".into(),
            count: 1,
        },
        unrelated: 0,
    });
    tick().await;
    assert_eq!(accessor_runs.load(Ordering::Relaxed), 2);
    assert_eq!(store.state().ready_count().unwrap().get(), 1);
    store.patch(Outer {
        state: State::Idle,
        unrelated: 0,
    });
    tick().await;
    assert_eq!(accessor_runs.load(Ordering::Relaxed), 3);
    assert!(store.state().ready_count().is_none());
    assert_eq!(unrelated_runs.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn named_accessor_inside_effect_tracks_only_its_payload_and_variant() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(State::Idle);
    let runs = observe_fn(move || store.ready_count().map(|count| count.get()));
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 1);

    store.patch(State::Ready {
        title: "hello".into(),
        count: 1,
    });
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 2);

    store.patch(State::Ready {
        title: "bye".into(),
        count: 1,
    });
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 2);

    store.ready_title().unwrap().set("direct".into());
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 2);

    store.patch(State::Ready {
        title: "direct".into(),
        count: 2,
    });
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 3);

    store.ready_count().unwrap().set(3);
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 4);

    store.patch(State::Idle);
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 5);

    store.patch(State::Ready {
        title: "again".into(),
        count: 4,
    });
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 6);

    store.ready_count().unwrap().set(5);
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 7);
}

#[tokio::test]
async fn tuple_accessor_inside_effect_tracks_only_its_payload_and_variant() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(State::Idle);
    let runs = observe_fn(move || store.pair_0().map(|first| first.get()));
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 1);

    store.patch(State::Pair(1, "hello".into()));
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 2);

    store.patch(State::Pair(1, "bye".into()));
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 2);

    store.pair_1().unwrap().set("direct".into());
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 2);

    store.patch(State::Pair(2, "direct".into()));
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 3);

    store.pair_0().unwrap().set(3);
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 4);

    store.patch(State::Idle);
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 5);

    store.patch(State::Pair(4, "again".into()));
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 6);

    store.pair_0().unwrap().set(5);
    tick().await;
    assert_eq!(runs.load(Ordering::Relaxed), 7);
}

#[tokio::test]
async fn enum_predicates_track_variant_changes_but_not_payload_changes() {
    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(State::Idle);
    let runs = [
        observe_fn(move || store.idle()),
        observe_fn(move || store.ready()),
        observe_fn(move || store.pair()),
    ];
    let assert_runs = |expected| {
        for (predicate, runs) in
            ["idle", "ready", "pair"].into_iter().zip(&runs)
        {
            assert_eq!(runs.load(Ordering::Relaxed), expected, "{predicate}");
        }
    };
    tick().await;
    assert_runs(1);

    store.patch(State::Ready {
        title: "hello".into(),
        count: 1,
    });
    tick().await;
    assert_runs(2);

    store.patch(State::Ready {
        title: "bye".into(),
        count: 2,
    });
    tick().await;
    assert_runs(2);

    store.ready_title().unwrap().set("direct".into());
    tick().await;
    assert_runs(2);
    store.ready_count().unwrap().set(3);
    tick().await;
    assert_runs(2);

    store.patch(State::Pair(1, "hello".into()));
    tick().await;
    assert_runs(3);

    store.patch(State::Pair(2, "bye".into()));
    tick().await;
    assert_runs(3);

    store.pair_0().unwrap().set(3);
    tick().await;
    assert_runs(3);
    store.pair_1().unwrap().set("direct".into());
    tick().await;
    assert_runs(3);

    store.patch(State::Idle);
    tick().await;
    assert_runs(4);

    // Whole-value writes notify even when the variant has not changed.
    store.set(State::Idle);
    tick().await;
    assert_runs(5);
}

#[tokio::test]
async fn cached_nested_enum_checks_track_ancestors_without_leaf_reads() {
    #[derive(Store)]
    struct Root {
        outer: Outer,
        sibling: i32,
    }

    _ = any_spawner::Executor::init_tokio();
    let store = Store::new(Root {
        outer: Outer {
            state: State::Idle,
            unrelated: 0,
        },
        sibling: 0,
    });
    // Cache the enum's parent handle outside every observer so constructing it
    // cannot supply the ancestor subscriptions these checks need.
    let state = store.outer().state();
    let runs = [
        observe_fn(move || state.ready_count().is_some()),
        observe_fn(move || state.pair_0().is_some()),
        observe_fn(move || state.idle()),
        observe_fn(move || state.ready()),
        observe_fn(move || state.pair()),
    ];
    let assert_runs = |expected| {
        for (check, runs) in ["ready_count", "pair_0", "idle", "ready", "pair"]
            .into_iter()
            .zip(&runs)
        {
            assert_eq!(runs.load(Ordering::Relaxed), expected, "{check}");
        }
    };
    tick().await;
    assert_runs(1);

    store.sibling().set(1);
    tick().await;
    assert_runs(1);
    store.outer().unrelated().set(1);
    tick().await;
    assert_runs(1);

    store.set(Root {
        outer: Outer {
            state: State::Ready {
                title: "hello".into(),
                count: 1,
            },
            unrelated: 1,
        },
        sibling: 1,
    });
    tick().await;
    assert_runs(2);

    state.ready_count().unwrap().set(2);
    tick().await;
    assert_runs(2);

    store.outer().set(Outer {
        state: State::Pair(1, "hello".into()),
        unrelated: 1,
    });
    tick().await;
    assert_runs(3);

    state.patch(State::Pair(2, "bye".into()));
    tick().await;
    assert_runs(3);

    state.patch(State::Idle);
    tick().await;
    assert_runs(4);

    store.outer().set(Outer {
        state: State::Ready {
            title: "again".into(),
            count: 3,
        },
        unrelated: 1,
    });
    tick().await;
    assert_runs(5);

    store.sibling().set(2);
    tick().await;
    assert_runs(5);
    store.outer().unrelated().set(2);
    tick().await;
    assert_runs(5);
}

mod upstream_regressions {
    //! Regression test: the `Patch` derive supports enums.
    //! - patching within the same variant updates fields in place and only
    //!   notifies the fields that changed,
    //! - patching across variants replaces the value and notifies subscribers.

    use reactive_graph::{
        effect::Effect,
        owner::Owner,
        traits::{Get, Read, ReadUntracked},
    };
    use reactive_stores::{Patch, Store};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug, Clone, PartialEq, Patch, Store, Default)]
    enum Mode {
        #[default]
        Idle,
        Running {
            progress: u32,
            label: String,
        },
        Done(u32),
    }

    #[derive(Debug, Clone, PartialEq, Patch, Store, Default)]
    struct State {
        mode: Mode,
    }

    async fn tick() {
        tokio::time::sleep(std::time::Duration::from_micros(1)).await;
    }

    #[tokio::test]
    async fn same_variant_patch_only_notifies_changed_field() {
        _ = any_spawner::Executor::init_tokio();
        let owner = Owner::new();
        owner.set();

        let store = Store::new(State {
            mode: Mode::Running {
                progress: 1,
                label: "a".into(),
            },
        });

        let progress_count = Arc::new(AtomicUsize::new(0));
        let label_count = Arc::new(AtomicUsize::new(0));

        let progress_sf = store.mode().running_progress().unwrap();
        let label_sf = store.mode().running_label().unwrap();

        Effect::new_sync({
            let c = Arc::clone(&progress_count);
            move |_: Option<()>| {
                let _ = progress_sf.read();
                c.fetch_add(1, Ordering::Relaxed);
            }
        });
        Effect::new_sync({
            let c = Arc::clone(&label_count);
            move |_: Option<()>| {
                let _ = label_sf.read();
                c.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;

        // Patch within the same variant, changing only `progress`.
        store.patch(State {
            mode: Mode::Running {
                progress: 2,
                label: "a".into(),
            },
        });
        tick().await;

        assert_eq!(
            store.mode().running_progress().unwrap().get(),
            2,
            "progress should be patched in place"
        );
        assert_eq!(
            progress_count.load(Ordering::Relaxed),
            2,
            "progress effect should re-run after its field changed"
        );
        assert_eq!(
            label_count.load(Ordering::Relaxed),
            1,
            "label effect should not re-run when only progress changed"
        );
    }

    #[tokio::test]
    async fn cross_variant_patch_replaces_and_notifies() {
        _ = any_spawner::Executor::init_tokio();
        let owner = Owner::new();
        owner.set();

        let store = Store::new(State { mode: Mode::Idle });

        let is_running_count = Arc::new(AtomicUsize::new(0));
        let is_running = Arc::new(AtomicUsize::new(0));

        Effect::new_sync({
            let c = Arc::clone(&is_running_count);
            let r = Arc::clone(&is_running);
            move |_: Option<()>| {
                r.store(store.mode().running() as usize, Ordering::Relaxed);
                c.fetch_add(1, Ordering::Relaxed);
            }
        });
        tick().await;
        assert_eq!(is_running.load(Ordering::Relaxed), 0);

        // Patch across variants: Idle -> Running.
        store.patch(State {
            mode: Mode::Running {
                progress: 5,
                label: "x".into(),
            },
        });
        tick().await;

        assert_eq!(
            is_running_count.load(Ordering::Relaxed),
            2,
            "variant-matcher effect should re-run after a variant change"
        );
        assert_eq!(is_running.load(Ordering::Relaxed), 1);
        assert!(matches!(
            store.read_untracked().mode,
            Mode::Running { progress: 5, .. }
        ));
    }

    #[tokio::test]
    async fn unnamed_variant_patches_in_place() {
        _ = any_spawner::Executor::init_tokio();
        let owner = Owner::new();
        owner.set();

        let store = Store::new(State {
            mode: Mode::Done(1),
        });
        store.patch(State {
            mode: Mode::Done(7),
        });
        assert!(matches!(store.read_untracked().mode, Mode::Done(7)));
    }
}
