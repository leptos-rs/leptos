use leptos_reactive::Scope;

pub enum Class<'a> {
    Value(bool),
    Fn(&'a dyn Fn() -> bool),
}

pub trait IntoClass<'a> {
    fn into_class(self, cx: Scope<'a>) -> Class<'a>;
}

impl<'a> IntoClass<'a> for bool {
    fn into_class(self, _cx: Scope<'a>) -> Class<'a> {
        Class::Value(self)
    }
}

impl<'a, T> IntoClass<'a> for T
where
    T: Fn() -> bool + 'a,
{
    fn into_class(self, cx: Scope<'a>) -> Class<'a> {
        let modified_fn = cx.create_ref(self);
        Class::Fn(modified_fn)
    }
}
