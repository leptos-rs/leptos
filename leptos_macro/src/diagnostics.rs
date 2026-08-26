#[derive(Default)]
pub(crate) struct Errors(Option<syn::Error>);

impl Errors {
    pub(crate) fn push(&mut self, error: syn::Error) {
        match &mut self.0 {
            Some(existing) => existing.combine(error),
            None => self.0 = Some(error),
        }
    }

    pub(crate) fn finish<T>(self, result: syn::Result<T>) -> syn::Result<T> {
        match (self.0, result) {
            (None, result) => result,
            (Some(error), Ok(_)) => Err(error),
            (Some(mut errors), Err(error)) => {
                errors.combine(error);
                Err(errors)
            }
        }
    }
}

pub(crate) fn message_with_help(
    message: impl std::fmt::Display,
    help: impl std::fmt::Display,
) -> String {
    format!("{message}\n\n  = help: {help}\n\n")
}
