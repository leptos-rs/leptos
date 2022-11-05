mod loader;

use std::{future::Future, pin::Pin};

pub use loader::*;

pub(crate) type PinnedFuture<T> = Pin<Box<dyn Future<Output = T>>>;
