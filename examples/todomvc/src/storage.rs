use crate::Todo;
use leptos::{signal_prelude::*, Scope};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct TodoSerialized {
    pub id: Uuid,
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
            title: todo.title.get(),
            completed: todo.completed.get(),
        }
    }
}
