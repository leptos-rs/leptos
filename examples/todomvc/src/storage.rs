use crate::Todo;
use leptos::signal_prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct TodoSerialized {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
}

impl TodoSerialized {
    pub fn into_todo(self) -> Todo {
        Todo::new_with_completed(self.id, self.title, self.completed)
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
