use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Unit,
    Static(Cow<'static, str>),
    Param(Cow<'static, str>),
    OptionalParam(Cow<'static, str>),
    Splat(Cow<'static, str>),
}

impl PathSegment {
    pub fn as_raw_str(&self) -> &str {
        match self {
            PathSegment::Unit => "",
            PathSegment::Static(i) => i,
            PathSegment::Param(i) => i,
            PathSegment::OptionalParam(i) => i,
            PathSegment::Splat(i) => i,
        }
    }
}

pub trait ExpandOptionals {
    fn expand_optionals(&self) -> Vec<Vec<PathSegment>>;
}

impl ExpandOptionals for Vec<PathSegment> {
    fn expand_optionals(&self) -> Vec<Vec<PathSegment>> {
        let mut segments = vec![self.to_vec()];
        let mut checked = Vec::new();
        while let Some(next_to_check) = segments.pop() {
            let mut had_optional = false;
            for (idx, segment) in next_to_check.iter().enumerate() {
                if let PathSegment::OptionalParam(name) = segment {
                    had_optional = true;
                    let mut unit_variant = next_to_check.to_vec();
                    unit_variant.remove(idx);
                    let mut param_variant = next_to_check.to_vec();
                    param_variant[idx] = PathSegment::Param(name.clone());
                    segments.push(unit_variant);
                    segments.push(param_variant);
                    break;
                }
            }
            if !had_optional {
                checked.push(next_to_check.to_vec());
            }
        }
        checked
    }
}

#[cfg(test)]
mod tests {
    use crate::{ExpandOptionals, PathSegment};

    #[test]
    fn expand_optionals_on_plain() {
        let plain = vec![
            PathSegment::Static("a".into()),
            PathSegment::Param("b".into()),
        ];
        assert_eq!(plain.expand_optionals(), vec![plain]);
    }

    #[test]
    fn expand_optionals_once() {
        let plain = vec![
            PathSegment::OptionalParam("a".into()),
            PathSegment::Static("b".into()),
        ];
        assert_eq!(
            plain.expand_optionals(),
            vec![
                vec![
                    PathSegment::Param("a".into()),
                    PathSegment::Static("b".into())
                ],
                vec![PathSegment::Static("b".into())]
            ]
        );
    }

    #[test]
    fn expand_optionals_twice() {
        let plain = vec![
            PathSegment::OptionalParam("a".into()),
            PathSegment::OptionalParam("b".into()),
            PathSegment::Static("c".into()),
        ];
        assert_eq!(
            plain.expand_optionals(),
            vec![
                vec![
                    PathSegment::Param("a".into()),
                    PathSegment::Param("b".into()),
                    PathSegment::Static("c".into()),
                ],
                vec![
                    PathSegment::Param("a".into()),
                    PathSegment::Static("c".into()),
                ],
                vec![
                    PathSegment::Param("b".into()),
                    PathSegment::Static("c".into()),
                ],
                vec![PathSegment::Static("c".into())]
            ]
        );
    }
}
