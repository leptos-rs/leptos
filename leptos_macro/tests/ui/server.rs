use leptos::prelude::*;

#[server(endpoint = "my_path", FooBar)]
pub async fn positional_argument_follows_keyword_argument(
) -> Result<(), ServerFnError> {
    Ok(())
}

#[server(endpoint = "first", endpoint = "second")]
pub async fn keyword_argument_repeated() -> Result<(), ServerFnError> {
    Ok(())
}

#[server(Foo, Bar)]
pub async fn expected_string_literal() -> Result<(), ServerFnError> {
    Ok(())
}
#[server(Foo, Bar, bazz)]
pub async fn expected_string_literal_2() -> Result<(), ServerFnError> {
    Ok(())
}

#[server("Foo")]
pub async fn expected_identifier() -> Result<(), ServerFnError> {
    Ok(())
}

#[server(Foo Bar)]
pub async fn expected_comma() -> Result<(), ServerFnError> {
    Ok(())
}

#[server(FooBar, "/foo/bar", "Cbor", "my_path", "extra")]
pub async fn unexpected_extra_argument() -> Result<(), ServerFnError> {
    Ok(())
}

#[server(encoding = "wrong")]
pub async fn encoding_not_found() -> Result<(), ServerFnError> {
    Ok(())
}

fn main() {}
