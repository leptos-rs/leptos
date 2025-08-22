use leptos::prelude::*;

#[derive(Clone, Default)]
struct SimpleLoggerInner(Vec<String>);

// may not be the most efficient but it gets the job done
impl IntoRender for SimpleLoggerInner {
    type Output = AnyView;

    fn into_render(self) -> Self::Output {
        let entries = self
            .0
            .into_iter()
            .map(|msg| {
                view! {
                    <li>{msg}</li>
                }
            })
            .collect_view();
        view! {
            <section id="SimpleLogger">
                <h1>"Simple Logger history"</h1>
                <div id="logs">
                    <ul>
                        {entries}
                    </ul>
                </div>
            </section>
        }
        .into_any()
    }
}

#[derive(Clone, Copy, Default)]
pub struct SimpleLogger(RwSignal<SimpleLoggerInner>);

impl SimpleLogger {
    pub fn log(&self, msg: impl ToString) {
        self.0.update(|vec| vec.0.push(msg.to_string()));
    }

    pub fn render(&self) -> AnyView {
        self.0.get().into_render()
    }
}
