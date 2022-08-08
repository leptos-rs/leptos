use crate::AnyComputation;

pub(crate) trait Owner {
    fn owned<'a>(&self) -> Vec<&'a dyn AnyComputation>;

    fn cleanups<'a>(&self) -> Vec<&'a mut dyn FnMut()>;

    fn owner<'a>(&self) -> Option<&'a dyn Owner>;
}
