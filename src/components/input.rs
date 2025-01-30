use dioxus::prelude::*;

#[derive(PartialEq, Props, Clone)]
pub struct InputProps {
    #[props(default = true)]
    secure: bool,
    #[props(default = false)]
    disabled: bool,
    #[props(into)]
    placeholder: String,
    #[props(into)]
    value: String,
    oninput: EventHandler<FormEvent>,
}

#[component]
pub fn Input(props: InputProps) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-2",
            input {
                class: "bg-black/10 border-none rounded text-center focus:ring-0 focus:bg-black/20 disabled:opacity-50",
                type: if props.secure { "password" } else { "text" },
                disabled: props.disabled,
                placeholder: props.placeholder,
                value: props.value,
                oninput: props.oninput
            }
        }
    }
}
