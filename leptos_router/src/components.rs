use leptos_core as leptos;
use leptos_dom::IntoChild;
use leptos_macro::{component, Props};
use leptos_reactive::Scope;
use serde::{de::DeserializeOwned, Serialize};

pub struct RouterProps<C, D>
where
    C: for<'a> IntoChild<'a>,
    D: Serialize + DeserializeOwned + 'static,
{
    base: Option<String>,
    data: Option<Box<dyn Fn() -> D>>,
    children: C,
}

pub fn Router<C, D>(cx: Scope, props: RouterProps<C, D>)
where
    C: for<'a> IntoChild<'a>,
    D: Serialize + DeserializeOwned + 'static,
{
}

/* pub fn Router = (props: RouterProps) => {
  const { source, url, base, data, out } = props;
  const integration =
    source || (isServer ? staticIntegration({ value: url || "" }) : pathIntegration());
  const routerState = createRouterContext(integration, base, data, out);

  return (
    <RouterContextObj.Provider value={routerState}>{props.children}</RouterContextObj.Provider>
  );
};
 */
