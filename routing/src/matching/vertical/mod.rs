use super::PartialPathMatch;

pub trait ChooseRoute {
    fn choose_route<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>>;
}
