/// Represents an HTTP method that can be handled by this route.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Method {
    /// The [`GET`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/GET) method
    /// requests a representation of the specified resource.
    #[default]
    Get,
    /// The [`POST`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/POST) method
    /// submits an entity to the specified resource, often causing a change in
    /// state or side effects on the server.
    Post,
    /// The [`PUT`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/PUT) method
    /// replaces all current representations of the target resource with the request payload.
    Put,
    /// The [`DELETE`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/DELETE) method
    /// deletes the specified resource.
    Delete,
    /// The [`PATCH`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/PATCH) method
    /// applies partial modifications to a resource.
    Patch,
}
