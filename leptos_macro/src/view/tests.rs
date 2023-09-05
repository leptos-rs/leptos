use proc_macro2::TokenStream;
use std::str::FromStr;
use syn::parse_quote;

fn pretty(input: TokenStream) -> String {
    let type_item: syn::Item = parse_quote! {
        fn view(){
            #input
        }
    };

    let file = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![type_item],
    };

    prettyplease::unparse(&file)
}

macro_rules! assert_snapshot
{
    (@assert text $result:ident) => {
        insta::assert_snapshot!(pretty($result))
    };
    (@assert full $result:ident) => {
        insta::assert_debug_snapshot!($result)
    };
    (client_template($assert:ident) => $input: expr) => {
        {
            let tokens = TokenStream::from_str($input).unwrap();
            let nodes = rstml::parse2(tokens).unwrap();
            let result = crate::view::client_template::render_template(&&nodes);

            assert_snapshot!(@assert $assert result)
        }
    };
    (client_builder($assert:ident) => $input: expr) => {
        {
            let tokens = TokenStream::from_str($input).unwrap();
            let nodes = rstml::parse2(tokens).unwrap();
            let mode = crate::view::Mode::Client;
            let global_class = None;
            let call_site = None;
            let result = crate::view::render_view(&&nodes, mode, global_class, call_site);

            assert_snapshot!(@assert $assert result)
        }
    };
    (server_template($assert:ident) => $input: expr) => {
        {
            let tokens = TokenStream::from_str($input).unwrap();
            let nodes = rstml::parse2(tokens).unwrap();
            let mode = crate::view::Mode::Ssr;
            let global_class = None;
            let call_site = None;
            let result = crate::view::render_view(&&nodes, mode, global_class, call_site);

            assert_snapshot!(@assert $assert result)
        }
    }

}

macro_rules! for_all_modes {
    (@ $module: ident, $type: ident => $(
        $test_name:ident => $raw_str:expr
    ),*
    ) => {
        mod $module {
            use super::*;
            $(
                #[test]
                fn $test_name() {
                    assert_snapshot!($type(text) => $raw_str)
                }
            )*
            mod full_span {
                use super::*;
                $(
                    #[test]
                    fn $test_name() {
                        assert_snapshot!($type(full) => $raw_str)
                    }
                )*
            }

        }
    };
    (   $(
        $tts:tt
        )*
    ) => {
        for_all_modes!{@ csr, client_builder =>  $($tts)*}
        for_all_modes!{@ client_template, client_template => $($tts)*}
        for_all_modes!{@ ssr, server_template => $($tts)*}
    };

}

for_all_modes! {
    test_simple_counter => r#"
        <div>
            <button on:click=move |_| set_value(0)>"Clear"</button>
            <button on:click=move |_| set_value.update(|value| *value -= step)>"-1"</button>
            <span>"Value: " {value} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += step)>"+1"</button>
        </div>
    "#,
    test_counter_component => r#"
        <SimpleCounter
            initial_value=0
            step=1
        />
    "#,
    test_custom_event => r#"
        <ExternalComponent on:custom.event.clear=move |_: Event| set_value(0) />
    "#
}
