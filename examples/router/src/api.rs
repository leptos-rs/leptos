use futures::{
    channel::oneshot::{self, Canceled},
    Future,
};
use leptos::leptos_dom::helpers::set_timeout;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactSummary {
    pub id: usize,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

pub async fn get_contacts(_search: String) -> Vec<ContactSummary> {
    // fake an API call with an artificial delay
    _ = delay(Duration::from_millis(300)).await;
    vec![
        ContactSummary {
            id: 0,
            first_name: "Bill".into(),
            last_name: "Smith".into(),
        },
        ContactSummary {
            id: 1,
            first_name: "Tim".into(),
            last_name: "Jones".into(),
        },
        ContactSummary {
            id: 2,
            first_name: "Sally".into(),
            last_name: "Stevens".into(),
        },
    ]
}

pub async fn get_contact(id: Option<usize>) -> Option<Contact> {
    // fake an API call with an artificial delay
    _ = delay(Duration::from_millis(500)).await;
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
        Some(1) => Some(Contact {
            id: 1,
            first_name: "Tim".into(),
            last_name: "Jones".into(),
            address_1: "56 Main Street".into(),
            address_2: "".into(),
            city: "Chattanooga".into(),
            state: "TN".into(),
            zip: "13371".into(),
            email: "timjones@lmail.com".into(),
            phone: "232-123-1337".into(),
        }),
        Some(2) => Some(Contact {
            id: 2,
            first_name: "Sally".into(),
            last_name: "Stevens".into(),
            address_1: "404 E 123rd St".into(),
            address_2: "Apt 7E".into(),
            city: "New York".into(),
            state: "NY".into(),
            zip: "10082".into(),
            email: "sally.stevens@wahoo.net".into(),
            phone: "242-121-3789".into(),
        }),
        _ => None,
    }
}

fn delay(
    duration: Duration,
) -> impl Future<Output = Result<(), Canceled>> + Send {
    let (tx, rx) = oneshot::channel();
    set_timeout(
        move || {
            _ = tx.send(());
        },
        duration,
    );
    rx
}
