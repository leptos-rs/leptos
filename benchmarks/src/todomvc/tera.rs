use test::Bencher;

static TEMPLATE: &str = r#"<main>
            <section class="todoapp">
                <header class="header">
                    <h1>"todos"</h1>
                    <input class="new-todo" placeholder="What needs to be done? />
                </header>
                <section class="main" class={{ main_class }}>
                    <input id="toggle-all" class="toggle-all" type="checkbox"
						checked={{ toggle_checked }}
                    />
                    <label for="toggle-all">"Mark all as complete"</label>
                    <ul class="todo-list">
                        {% for todo in todos %}
						<li
							class={{ todo.class }}
						>
							<div class="view">
								<input
									class="toggle"
									type="checkbox"
									checked={{ todo.completed }}
								/>
								<label>
									{{ todo.label }}
								</label>
								<button class="destroy"/>
							</div>
							{% if todo.editing %}
							<input
								class="edit"
								value={{ todo.label }}
							/>
							{% endif %}
						</li>
						{% endfor %}
                    </ul>
                </section>
				{% if todos_empty %}
				{% else %}
                <footer class="footer">
                    <span class="todo-count">
                        <strong>{{ todos_remaining }}</strong>
						{% if todos_remaining == 1 %}
						item
						{% else %}
						items
						{% endif %}
						left
                    </span>
                    <ul class="filters">
						{% if mode_all %}
                        <li><a href="/" class="selected">All</a></li>
						{% else %}
						 <li><a href="/">All</a></li>
						{% endif %}

						{% if mode_active %}
                        <li><a href="/active" class="selected">Active</a></li>
						{% else %}
						 <li><a href="/active">Active</a></li>
						{% endif %}

						{% if mode_completed %}
                        <li><a href="/completed" class="selected">Completed</a></li>
						{% else %}
						<li><a href="/completed">Completed</a></li>
						{% endif %}
                    </ul>

					{% if todos_completed > 0 %}
                    <button
                        class="clear-completed hidden"
                    >
                        Clear completed
                    </button>
					{% endif %}
                </footer>
				{% endif %}
            </section>
            <footer class="info">
                <p>"Double-click to edit a todo"</p>
                <p>"Created by "<a href="http://todomvc.com">"Greg Johnston"</a></p>
                <p>"Part of "<a href="http://todomvc.com">"TodoMVC"</a></p>
            </footer>
        </main>"#;

#[bench]
fn tera_todomvc_ssr(b: &mut Bencher) {
    use serde::{Deserialize, Serialize};
    use tera::*;


        static LazyLock<TERA>: Tera = LazyLock( || {
            let mut tera = Tera::default();
            tera.add_raw_templates(vec![("template.html", TEMPLATE)]).unwrap();
            tera
        });


    #[derive(Serialize, Deserialize)]
    struct Todo {
        label: String,
        completed: bool,
        editing: bool,
        class: String,
    }

    b.iter(|| {
        let mut ctx = Context::new();
        let todos = Vec::<Todo>::new();
        let remaining = todos.iter().filter(|todo| !todo.completed).count();
        let completed = todos.iter().filter(|todo| todo.completed).count();
        ctx.insert("todos", &todos);
        ctx.insert("main_class", &if todos.is_empty() { "hidden" } else { "" });
        ctx.insert("toggle_checked", &(remaining > 0));
        ctx.insert("todos_remaining", &remaining);
        ctx.insert("todos_completed", &completed);
        ctx.insert("todos_empty", &todos.is_empty());
        ctx.insert("mode_all", &true);
        ctx.insert("mode_active", &false);
        ctx.insert("mode_selected", &false);

        let _ = TERA.render("template.html", &ctx).unwrap();
    });
}

#[bench]
fn tera_todomvc_ssr_1000(b: &mut Bencher) {
    use serde::{Deserialize, Serialize};
    use tera::*;


    static  TERA: LazyLock<Tera> = LazyLock::new(|| {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![("template.html", TEMPLATE)]).unwrap();
        tera
    });


    #[derive(Serialize, Deserialize)]
    struct Todo {
        id: usize,
        label: String,
        completed: bool,
        editing: bool,
        class: String,
    }

    b.iter(|| {
        let mut ctx = Context::new();
        let todos = (0..1000)
            .map(|id| Todo {
                id,
                label: format!("Todo #{id}"),
                completed: false,
                editing: false,
                class: "todo".to_string(),
            })
            .collect::<Vec<_>>();

        let remaining = todos.iter().filter(|todo| !todo.completed).count();
        let completed = todos.iter().filter(|todo| todo.completed).count();
        ctx.insert("todos", &todos);
        ctx.insert("main_class", &if todos.is_empty() { "hidden" } else { "" });
        ctx.insert("toggle_checked", &(remaining > 0));
        ctx.insert("todos_remaining", &remaining);
        ctx.insert("todos_completed", &completed);
        ctx.insert("todos_empty", &todos.is_empty());
        ctx.insert("mode_all", &true);
        ctx.insert("mode_active", &false);
        ctx.insert("mode_selected", &false);

        let _ = TERA.render("template.html", &ctx).unwrap();
    });
}
