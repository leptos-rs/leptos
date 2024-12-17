use reactive_graph::{
    owner::Owner,
    signal::{arc_signal, signal, ArcRwSignal, RwSignal},
    traits::{
        Dispose, Get, GetUntracked, IntoInner, Read, Set, Update,
        UpdateUntracked, With, WithUntracked, Write,
    },
};

#[test]
fn create_arc_rw_signal() {
    let a = ArcRwSignal::new(0);
    assert_eq!(a.read(), 0);
    assert_eq!(a.get(), 0);
    assert_eq!(a.get_untracked(), 0);
    assert_eq!(a.with_untracked(|n| n + 1), 1);
    assert_eq!(a.with(|n| n + 1), 1);
    assert_eq!(format!("{}", a.read()), "0");
}

#[test]
fn update_arc_rw_signal() {
    let a = ArcRwSignal::new(0);
    *a.write() += 1;
    assert_eq!(a.get(), 1);
    a.update(|n| *n += 1);
    assert_eq!(a.get(), 2);
    a.update_untracked(|n| *n += 1);
    assert_eq!(a.get(), 3);
    a.set(4);
    assert_eq!(a.get(), 4);
}

#[test]
fn create_arc_signal() {
    let (a, _) = arc_signal(0);
    assert_eq!(a.read(), 0);
    assert_eq!(a.get(), 0);
    assert_eq!(a.with_untracked(|n| n + 1), 1);
    assert_eq!(a.with(|n| n + 1), 1);
}

#[test]
fn update_arc_signal() {
    let (a, set_a) = arc_signal(0);
    *set_a.write() += 1;
    assert_eq!(a.get(), 1);
    set_a.update(|n| *n += 1);
    assert_eq!(a.get(), 2);
    set_a.update_untracked(|n| *n += 1);
    assert_eq!(a.get(), 3);
    set_a.set(4);
    assert_eq!(a.get(), 4);
}

#[test]
fn create_rw_signal() {
    let owner = Owner::new();
    owner.set();

    let a = RwSignal::new(0);
    assert_eq!(a.read(), 0);
    assert_eq!(a.get(), 0);
    assert_eq!(a.with_untracked(|n| n + 1), 1);
    assert_eq!(a.with(|n| n + 1), 1);
}

#[test]
fn update_rw_signal() {
    let owner = Owner::new();
    owner.set();

    let a = RwSignal::new(1);
    assert_eq!(a.read(), 1);
    assert_eq!(a.get(), 1);
    a.update(|n| *n += 1);
    assert_eq!(a.get(), 2);
    a.update_untracked(|n| *n += 1);
    assert_eq!(a.get(), 3);
    a.set(4);
    assert_eq!(a.get(), 4);
}

#[test]
fn create_signal() {
    let owner = Owner::new();
    owner.set();

    let (a, _) = signal(0);
    assert_eq!(a.read(), 0);
    assert_eq!(a.get(), 0);
    assert_eq!(a.get_untracked(), 0);
    assert_eq!(a.with_untracked(|n| n + 1), 1);
    assert_eq!(a.with(|n| n + 1), 1);
}

#[test]
fn update_signal() {
    let owner = Owner::new();
    owner.set();

    let (a, set_a) = signal(1);
    assert_eq!(a.get(), 1);
    set_a.update(|n| *n += 1);
    assert_eq!(a.get(), 2);
    set_a.update_untracked(|n| *n += 1);
    assert_eq!(a.get(), 3);
    set_a.set(4);
    assert_eq!(a.get(), 4);
}

#[test]
fn into_inner_signal() {
    let owner = Owner::new();
    owner.set();

    let rw_signal = RwSignal::new(1);
    assert_eq!(rw_signal.get(), 1);
    assert_eq!(rw_signal.into_inner(), Some(1));
}

#[test]
fn into_inner_arc_signal() {
    let owner = Owner::new();
    owner.set();

    let (a, b) = arc_signal(2);
    assert_eq!(a.get(), 2);
    std::mem::drop(b);
    assert_eq!(a.into_inner(), Some(2));
}

#[test]
fn into_inner_non_arc_signal() {
    let owner = Owner::new();
    owner.set();

    let (a, b) = signal(2);
    assert_eq!(a.get(), 2);
    b.dispose();
    assert_eq!(a.into_inner(), Some(2));
}
