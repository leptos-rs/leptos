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
    /// Class set when all animations have finished.
    pub finally: Option<&'static str>,
}

impl Animation {
    pub(crate) fn next_state(
        &self,
        current: &AnimationState,
    ) -> (AnimationState, bool) {
        leptos::log!("Animation::next_state() current is {current:?}");
        let Animation {
            start,
            outro,
            intro,
            finally,
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
