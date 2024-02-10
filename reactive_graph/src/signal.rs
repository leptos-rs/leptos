mod arc_read;
mod arc_rw;
mod arc_write;
mod guards;
mod read;
mod rw;
mod subscriber_traits;
mod write;

pub use arc_read::*;
pub use arc_rw::*;
pub use arc_write::*;
pub use guards::*;
pub use read::*;
pub use rw::*;
pub use write::*;

#[inline(always)]
#[track_caller]
pub fn arc_signal<T>(value: T) -> (ArcReadSignal<T>, ArcWriteSignal<T>) {
    ArcRwSignal::new(value).split()
}

#[inline(always)]
#[track_caller]
pub fn signal<T: Send + Sync>(value: T) -> (ReadSignal<T>, WriteSignal<T>) {
    RwSignal::new(value).split()
}

#[inline(always)]
#[track_caller]
#[deprecated = "This function is being renamed to `signal()` to conform to \
                Rust idioms."]
pub fn create_signal<T: Send + Sync>(
    value: T,
) -> (ReadSignal<T>, WriteSignal<T>) {
    signal(value)
}
