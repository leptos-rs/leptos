use std::time::Duration;

use futures::{
    channel::oneshot::{self, Canceled},
    Future,
};
use leptos::set_timeout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSummary {
    pub id: usize,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub id: usize,
    pub first_name: String,
    pub last_name: String,
    pub address_1: String,
    pub address_2: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub email: String,
    pub phone: String,
}

pub async fn get_contacts(search: String) -> Vec<ContactSummary> {
    // fake an API call with an artificial delay
    delay(Duration::from_millis(100)).await;
    vec![ContactSummary {
        id: 0,
        first_name: "Bill".into(),
        last_name: "Smith".into(),
    }]
}

pub async fn get_contact(id: Option<usize>) -> Option<Contact> {
    // fake an API call with an artificial delay
    delay(Duration::from_millis(350)).await;
    match id {
        Some(0) => Some(Contact {
            id: 0,
            first_name: "Bill".into(),
            last_name: "Smith".into(),
            address_1: "12 Mulberry Lane".into(),
            address_2: "".into(),
            city: "Boston".into(),
            state: "MA".into(),
            zip: "02129".into(),
            email: "bill@smith.com".into(),
            phone: "617-121-1221".into(),
        }),
        _ => None,
    }
}

fn delay(duration: Duration) -> impl Future<Output = Result<(), Canceled>> {
    let (tx, rx) = oneshot::channel();
    set_timeout(
        move || {
            tx.send(());
        },
        duration,
    );
    rx
}
