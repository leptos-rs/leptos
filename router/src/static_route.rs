/// The mode to use when rendering the route statically.
/// On mode `Upfront`, the route will be built with the server is started using the provided static
/// data. On mode `Incremental`, the route will be built on the first request to it and then cached
/// and returned statically for subsequent requests.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StaticMode {
    #[default]
    Upfront,
    Incremental,
}

// TODO
#[derive(Debug, Clone)]
pub struct StaticDataMap;

impl StaticDataMap {
    pub fn new() -> Self {
        Self
    }
}
