use leptos::prelude::*;

#[derive(Clone, Default)]
pub struct SimpleLogger(Vec<String>);

impl SimpleLogger {
    pub fn log(&mut self, msg: impl ToString) {
        self.0.push(msg.to_string());
    }
}

// may not be the most efficient but it gets the job done
impl IntoRender for SimpleLogger {
    type Output = AnyView;

    fn into_render(self) -> Self::Output {
        let entries = self.0
            .into_iter()
            .map(|msg| view! {
                <li>{msg}</li>
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
