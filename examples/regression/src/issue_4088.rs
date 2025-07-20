use leptos::{either::Either, prelude::*};
#[allow(unused_imports)]
use leptos_router::{
    components::{Outlet, ParentRoute, Redirect, Route},
    path, MatchNestedRoutes, NavigateOptions,
};
use serde::{Deserialize, Serialize};

#[component]
pub fn Routes4088() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("4088") view=|| view!{ <LoggedIn/> }>
			<ParentRoute path=path!("") view=||view!{<AssignmentsSelector/>}>
				<Route path=path!("/:team_id") view=||view!{<AssignmentsForTeam/>} />
				<Route path=path!("") view=||view!{ <p>No class selected</p> }/>
			</ParentRoute>
        </ParentRoute>
    }
    .into_inner()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: usize,
}

#[server]
pub async fn get_user_info() -> Result<Option<UserInfo>, ServerFnError> {
    Ok(Some(UserInfo { id: 42 }))
}

#[component]
pub fn LoggedIn() -> impl IntoView {
    let user_info_resource =
        Resource::new(|| (), move |_| async { get_user_info().await });

    view! {

      <Transition fallback=move || view!{
            "loading"
            }
        >
        {move || {
            user_info_resource.get()
                .map(|a|
                    match a {
                        Ok(Some(a)) => Either::Left(view! {
                            <LoggedInContent user_info={a} />
                        }),
                        _ => Either::Right(view!{
                            <Redirect path="/not_logged_in"/>
                        })
                    })
        }}
        </Transition>
    }
}

#[component]
/// Component which provides UserInfo and renders it's child
/// Can also contain some code to check for specific situations (e.g. privacy policies accepted or not? redirect if needed...)
pub fn LoggedInContent(user_info: UserInfo) -> impl IntoView {
    provide_context(user_info.clone());

    if user_info.id == 42 {
        Either::Left(Outlet())
    } else {
        Either::Right(
            view! { <Redirect path="/somewhere" options={NavigateOptions::default()}/> },
        )
    }
}

#[component]
/// This component also uses Outlet (so nested Outlet)
fn AssignmentsSelector() -> impl IntoView {
    let user_info = use_context::<UserInfo>().expect("user info not provided");

    view! {
        <p>"Assignments for user with ID: "{user_info.id}</p>
        <ul id="nav">
            <li><a href="/4088/1">"Class 1"</a></li>
            <li><a href="/4088/2">"Class 2"</a></li>
            <li><a href="/4088/3">"Class 3"</a></li>
        </ul>

        <Outlet />
    }
}

#[component]
fn AssignmentsForTeam() -> impl IntoView {
    // THIS FAILS -> Because of the nested outlet in LoggedInContent > AssignmentsSelector?
    // It did not fail when LoggedIn did not use a resource and transition (but a hardcoded UserInfo in the component)
    let user_info = use_context::<UserInfo>().expect("user info not provided");

    let items = vec!["Assignment 1", "Assignment 2", "Assignment 3"];
    view! {
        <p id="result">"Assignments for team of user with id " {user_info.id}</p>
        <ul>
            {
            items.into_iter().map(|item| {
                view! {
                    <Assignment name=item.to_string() />
                }
            }).collect_view()
            }
        </ul>
    }
}

#[component]
fn Assignment(name: String) -> impl IntoView {
    let user_info = use_context::<UserInfo>().expect("user info not provided");

    view! {
        <li>{name}" "{user_info.id}</li>
    }
}
