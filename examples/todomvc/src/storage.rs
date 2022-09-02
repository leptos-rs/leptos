use crate::Todo;
use leptos::Scope;
use miniserde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TodoSerialized {
    pub id: usize,
    pub title: String,
    pub completed: bool,
}

impl TodoSerialized {
    pub fn into_todo(self, cx: Scope) -> Todo {
        Todo::new_with_completed(cx, self.id, self.title, self.completed)
    }
}

impl From<&Todo> for TodoSerialized {
    fn from(todo: &Todo) -> Self {
        Self {
            id: todo.id,
            title: todo.title.get().to_string(),
            completed: (todo.completed)(),
        }
    }
}
