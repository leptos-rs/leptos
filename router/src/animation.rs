/// Configures what animation should be shown when transitioning
/// between two root routes. Defaults to `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Animation {
    /// No animation set up.
    None,
    /// Animated using CSS classes.
    Classes {
        /// Class set when a route is first painted.
        start: Option<&'static str>,
        /// Class set when a route is fading out.
        outro: Option<&'static str>,
        /// Class set when a route is fading in.
        intro: Option<&'static str>,
        /// Class set when all animations have finished.
        finally: Option<&'static str>,
    },
}

impl Animation {
    pub(crate) fn next_state(&self, current: &AnimationState) -> (AnimationState, bool) {
        leptos::log!("Animation::next_state() current is {current:?}");
        match self {
            Self::None => (AnimationState::Finally, true),
            Self::Classes {
                start,
                outro,
                intro,
                finally,
            } => match current {
                AnimationState::Outro => {
                    let next = if start.is_some() {
                        AnimationState::Start
                    } else if intro.is_some() {
                        AnimationState::Intro
                    } else {
                        AnimationState::Finally
                    };
                    (next, true)
                }
                AnimationState::Start => {
                    let next = if intro.is_some() {
                        AnimationState::Intro
                    } else {
                        AnimationState::Finally
                    };
                    (next, false)
                }
                AnimationState::Intro => (AnimationState::Finally, false),
                AnimationState::Finally => {
                    if outro.is_some() {
                        (AnimationState::Outro, false)
                    } else if start.is_some() {
                        (AnimationState::Start, true)
                    } else if intro.is_some() {
                        (AnimationState::Intro, true)
                    } else {
                        (AnimationState::Finally, true)
                    }
                }
            },
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum AnimationState {
    Outro,
    Start,
    Intro,
    Finally,
}

impl Default for Animation {
    fn default() -> Self {
        Self::None
    }
}