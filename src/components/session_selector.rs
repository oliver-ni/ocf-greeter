use dioxus::prelude::*;

use crate::sessions::Session;

#[derive(PartialEq, Props, Clone)]
pub struct SessionSelectorProps {
    sessions: Vec<Session>,
    session: Option<Session>,
    onchange: EventHandler<Option<Session>>,
}

#[component]
pub fn SessionSelector(props: SessionSelectorProps) -> Element {
    let options = rsx! {
        for session in &props.sessions {
            option { value: session.slug.as_str(), {session.name.as_str()} }
        }
    };

    let onchange = move |event: FormEvent| {
        let session = props.sessions.iter().find(|session| session.slug == event.value()).cloned();
        props.onchange.call(session)
    };

    rsx! {
        select {
            class: "bg-black/10 border-none rounded text-xs self-end focus:ring-0 focus:bg-black/20",
            onchange: onchange,
            value: props.session.as_ref().map(|session| session.slug.clone()),
            option { disabled: true, selected: matches!(props.session, None), "Select a session" }
            {options}
        }
    }
}
