use routing::StaticSegment;
use routing_macro::path;

#[test]
fn parses_empty_list() {
    let output = path!("");
    assert_eq!(output, ());
    //let segments: Segments = syn::parse(path.into()).unwrap();
}
