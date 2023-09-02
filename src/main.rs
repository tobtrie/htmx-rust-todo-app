use std::{ops::Deref, sync::Mutex};

use actix_files as fs;
use actix_web::{
    error, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use derive_more::{Display, Error};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
struct Todo {
    id: u128,
    name: String,
    done: bool,
}

impl Todo {
    fn render(&self) -> Markup {
        let id = format!("todo-{}", self.id);
        html!(
            li id=(id) class="flex flex-row"{
                div .line-through[self.done] ."flex-1" {
                    (self.name)
                }
                input type="checkbox" checked[self.done] hx-post=(format!("/{}/done", self.id)) hx-trigger="click" hx-target=(format!("#{}", id)) hx-swap="outerHTML" ;
            }
        )
    }
}
struct AppState {
    todos: Vec<Todo>,
    last_index: u128,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "my error: {}", name)]
struct ApiError {
    name: &'static str,
}

impl error::ResponseError for ApiError {}

fn render_list(todos: &[Todo]) -> Markup {
    html! {
        @for todo in todos.iter() {
            (todo.render())
        }
    }
}

#[get("/")]
async fn index(data: web::Data<Mutex<AppState>>) -> Result<Markup, ApiError> {
    let state = match data.lock() {
        Ok(state) => state,
        Err(_) => return Err(ApiError { name: "mutex lock" }),
    };
    let body = html! {
        (DOCTYPE)
        script src="/assets/tailwind.min.js" {}
        script src="/assets/htmx.min.js"{}
        link rel="icon" type="image/png" href="/assets/favicon.png";
        link src="/assets/global.css" rel="stylesheet" {}
        title {
            "Todo"
        }

        body ."min-h-sreen" .text-white .bg-black ."p-4" {
        main class="container m-auto flex flex-col gap-4" {
            h1 class="text-2xl" {
                "Daily todos"
            }
            form
            class="flex flex-row gap-4"
            hx-post="/add"
            hx-target="#todo-list"
            hx-swap="beforeend" {
                input name="prompt" class="flex-1 border rounded border-neutral-400 text-sm px-4 py-2 bg-black" {}
                button class="rounded bg-blue-500 px-4 py-2" {"Add"}
            }
            div ."text-neutral-400" hx-get="/statistic" hx-trigger="changedTodos from:body"{
                (format!("Complited {} of {} todos", state.todos.iter().filter(|todo| todo.done).count(), state.todos.len()))
            }
            ul #todo-list {
                (render_list(state.todos.deref()))
            }
        }
        }
    };
    Ok(body)
}

#[derive(Deserialize)]
struct FormData {
    prompt: String,
}

#[post("/add")]
async fn add(data: web::Data<Mutex<AppState>>, form: web::Form<FormData>) -> impl Responder {
    let mut state = match data.lock() {
        Ok(val) => val,
        Err(_) => return HttpResponse::Ok().body(
            html! {
                div class="bg-red-500"{ "An error occured during accouring the lock of the mutex" }
            }
            .into_string(),
        ),
    };
    let id: u128 = state.last_index;
    let todo = Todo {
        id: id,
        name: form.prompt.clone(),
        done: false,
    };
    state.todos.push(todo.clone());
    state.last_index += 1;
    HttpResponse::Ok()
        .append_header(("HX-Trigger", "changedTodos"))
        .body(todo.render().into_string())
}

#[post("{id}/done")]
async fn toggle_done(req: HttpRequest, data: web::Data<Mutex<AppState>>) -> impl Responder {
    let mut state = match data.lock() {
        Ok(state) => state,
        Err(_) => return Err(ApiError { name: "mutex lock" }),
    };

    let id: u128 = match req.match_info().get("id") {
        Some(id) => id.parse().unwrap(),
        None => {
            return Err(ApiError {
                name: "path variable",
            })
        }
    };

    let mut todo: Vec<&mut Todo> = state
        .todos
        .iter_mut()
        .filter(|todo| todo.id == id)
        .collect();
    if todo.len() > 0 {
        let item = todo.get_mut(0).unwrap();
        item.done = !item.done;
        return Ok(HttpResponse::Ok()
            .append_header(("HX-Trigger", "changedTodos"))
            .body(item.render().into_string()));
    }
    Ok(HttpResponse::NoContent().body(()))
}

#[get("/statistic")]
async fn render_stats(data: web::Data<Mutex<AppState>>) -> impl Responder {
    let state = match data.lock() {
        Ok(state) => state,
        Err(_) => return Err(ApiError { name: "mutex lock" }),
    };

    Ok(html! {
        span {
            (format!("Complited {} of {} todos", state.todos.iter().filter(|todo| todo.done).count(), state.todos.len()))
        }
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = 8080;

    let data = web::Data::new(Mutex::new(AppState {
        todos: vec![],
        last_index: 0,
    }));
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::clone(&data))
            .service(fs::Files::new("/assets", "./static").show_files_listing())
            .service(index)
            .service(add)
            .service(toggle_done)
            .service(render_stats)
    })
    .bind(("127.0.0.1", port))?
    .run();
    println!("The server runs on http://0.0.0.0:{}", port);
    server.await
}
