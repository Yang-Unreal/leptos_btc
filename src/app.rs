use leptos::html::Input;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use crate::todo::*;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos_btc.css"/>

        // sets the document title
        <Title text="Leptos + Postgres CRUD"/>

        // full-stack CRUD UI
        <main>
            <Todos/>
        </main>
    }
}

/// A full-stack CRUD example backed by Postgres (via SQLx server functions).
#[component]
pub fn Todos() -> impl IntoView {
    // A signal we bump to force the todo list to refetch after a mutation.
    let (refetch, set_refetch) = signal(0);
    let todos = Resource::new(move || refetch.get(), |_| get_todos());

    let title_ref = NodeRef::<Input>::new();

    let add = Action::new(move |title: &String| {
        let title = title.clone();
        async move {
            add_todo(title).await?;
            set_refetch.update(|n| *n += 1);
            Ok::<(), ServerFnError>(())
        }
    });

    let toggle = Action::new(move |id: &i32| {
        let id = *id;
        async move {
            toggle_todo(id).await?;
            set_refetch.update(|n| *n += 1);
            Ok::<(), ServerFnError>(())
        }
    });

    let delete = Action::new(move |id: &i32| {
        let id = *id;
        async move {
            delete_todo(id).await?;
            set_refetch.update(|n| *n += 1);
            Ok::<(), ServerFnError>(())
        }
    });

    // Which todo is currently being edited (None = none).
    let (editing, set_editing) = signal(Option::<i32>::None);

    let update = Action::new(move |args: &(i32, String)| {
        let (id, title) = args;
        let id = *id;
        let title = title.clone();
        async move {
            update_todo(id, title).await?;
            set_refetch.update(|n| *n += 1);
            Ok::<(), ServerFnError>(())
        }
    });

    view! {
        <section class="todos">
            <h2>"Todos (Postgres + SQLx)"</h2>
            <form
                class="todo-form"
                on:submit=move |ev| {
                    ev.prevent_default();
                    let value = title_ref.get().map(|el| el.value()).unwrap_or_default();
                    if !value.trim().is_empty() {
                        add.dispatch(value);
                        if let Some(input) = title_ref.get() {
                            input.set_value("");
                        }
                    }
                }
            >
                <input node_ref=title_ref type="text" placeholder="What needs to be done?"/>
                <button type="submit">"Add"</button>
            </form>

            <Transition fallback=move || view! { <p>"Loading todos…"</p> }>
                {move || match todos.get() {
                    None => view! { <div>"Loading…"</div> }.into_view().into_any(),
                    Some(Err(e)) => {
                        view! { <div class="error">{format!("Error: {}", e)}</div> }
                            .into_view()
                            .into_any()
                    }
                    Some(Ok(list)) => view! {
                        <div>
                            <ul class="todo-list">
                                {list
                                    .into_iter()
                                    .map(|todo| {
                                        let id = todo.id;
                                        let edit_ref = NodeRef::<Input>::new();
                                        let is_editing = editing.get() == Some(id);
                                        view! {
                                            <li class:completed=todo.completed>
                                                {if is_editing {
                                                    view! {
                                                        <input
                                                            node_ref=edit_ref
                                                            type="text"
                                                            class="edit-input"
                                                            prop:value=todo.title
                                                        />
                                                        <button
                                                            class="save"
                                                            on:click=move |_| {
                                                                let value = edit_ref
                                                                    .get()
                                                                    .map(|el| el.value())
                                                                    .unwrap_or_default();
                                                                update.dispatch((id, value));
                                                                set_editing.set(None);
                                                            }
                                                        >"Save"</button>
                                                        <button
                                                            class="cancel"
                                                            on:click=move |_| set_editing.set(None)
                                                        >"Cancel"</button>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <input
                                                            type="checkbox"
                                                            prop:checked=todo.completed
                                                            on:click=move |_| {
                                                                toggle.dispatch(id);
                                                            }
                                                        />
                                                        <span class="title">{todo.title}</span>
                                                        <button
                                                            class="edit"
                                                            on:click=move |_| set_editing.set(Some(id))
                                                        >"✎"</button>
                                                        <button
                                                            class="delete"
                                                            on:click=move |_| {
                                                                delete.dispatch(id);
                                                            }
                                                        >"✕"</button>
                                                    }.into_any()
                                                }}
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        </div>
                    }
                    .into_view()
                    .into_any()
                }}
            </Transition>
        </section>
    }
}
