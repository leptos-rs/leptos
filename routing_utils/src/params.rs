extern crate alloc;
use alloc::{string::String, vec::Vec};

pub(crate) type Params<K> = Vec<(K, String)>;
