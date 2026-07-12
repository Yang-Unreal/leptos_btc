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
    // A signal we bump to force the todo list to refetch (server is source of truth).
    let (refetch, set_refetch) = signal(0);
    let todos = Resource::new(move || refetch.get(), |_| get_todos());

    // Local mirror of the list so we can update the UI instantly (optimistic)
    // instead of waiting for the server round-trip on every mutation.
    let todos_local = RwSignal::new(None::<Vec<Todo>>);
    Effect::new(move |_| {
        if let Some(Ok(list)) = todos.get() {
            todos_local.set(Some(list));
        }
    });

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
            // Optimistic UI update already happened in the click handler.
            // Only resync from the server if the request actually failed.
            if toggle_todo(id).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
            Ok::<(), ServerFnError>(())
        }
    });

    let delete = Action::new(move |id: &i32| {
        let id = *id;
        async move {
            if delete_todo(id).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
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
            if update_todo(id, title).await.is_err() {
                set_refetch.update(|n| *n += 1);
            }
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
                {move || match (todos.get(), todos_local.get()) {
                    // Local mirror has data (also covers post-mutation state).
                    (_, Some(list)) => view! {
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
                                                                // Optimistic: update title locally now.
                                                                todos_local.update(|opt| {
                                                                    if let Some(list) = opt {
                                                                        if let Some(t) = list
                                                                            .iter_mut()
                                                                            .find(|t| t.id == id)
                                                                        {
                                                                            t.title = value.clone();
                                                                        }
                                                                    }
                                                                });
                                                                set_editing.set(None);
                                                                update.dispatch((id, value));
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
                                                                // Optimistic: flip completed locally now.
                                                                todos_local.update(|opt| {
                                                                    if let Some(list) = opt {
                                                                        if let Some(t) = list
                                                                            .iter_mut()
                                                                            .find(|t| t.id == id)
                                                                        {
                                                                            t.completed = !t.completed;
                                                                        }
                                                                    }
                                                                });
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
                                                                // Optimistic: remove locally now.
                                                                todos_local.update(|opt| {
                                                                    if let Some(list) = opt {
                                                                        list.retain(|t| t.id != id);
                                                                    }
                                                                });
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
                    .into_any(),
                    // Server returned an error and we have nothing cached.
                    (Some(Err(e)), None) => {
                        view! { <div class="error">{format!("Error: {}", e)}</div> }
                            .into_view()
                            .into_any()
                    }
                    // Still loading / reconciling.
                    _ => view! { <div>"Loading…"</div> }.into_view().into_any(),
                }}
            </Transition>
        </section>
    }
}
