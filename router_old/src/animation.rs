/// Configures what animation should be shown when transitioning
/// between two root routes. Defaults to `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Animation {
    /// Class set when a route is first painted.
    pub start: Option<&'static str>,
    /// Class set when a route is fading out.
    pub outro: Option<&'static str>,
    /// Class set when a route is fading in.
    pub intro: Option<&'static str>,
    /// Class set when a route is fading out, if it’s a “back” navigation.
    pub outro_back: Option<&'static str>,
    /// Class set when a route is fading in, if it’s a “back” navigation.
    pub intro_back: Option<&'static str>,
    /// Class set when all animations have finished.
    pub finally: Option<&'static str>,
}

impl Animation {
    pub(crate) fn next_state(
        &self,
        current: &AnimationState,
        is_back: bool,
    ) -> (AnimationState, bool) {
        let Animation {
            start,
            outro,
            intro,
            intro_back,
            ..
        } = self;
        match current {
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
            AnimationState::OutroBack => {
                let next = if start.is_some() {
                    AnimationState::Start
                } else if intro_back.is_some() {
                    AnimationState::IntroBack
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
            AnimationState::IntroBack => (AnimationState::Finally, false),
            AnimationState::Finally => {
                if outro.is_some() {
                    if is_back {
                        (AnimationState::OutroBack, false)
                    } else {
                        (AnimationState::Outro, false)
                    }
                } else if start.is_some() {
                    (AnimationState::Start, true)
                } else if intro.is_some() {
                    if is_back {
                        (AnimationState::IntroBack, false)
                    } else {
                        (AnimationState::Intro, false)
                    }
                } else {
                    (AnimationState::Finally, true)
                }
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum AnimationState {
    Outro,
    OutroBack,
    Start,
    Intro,
    IntroBack,
    Finally,
}
