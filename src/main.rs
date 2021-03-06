#![feature(proc_macro_hygiene, decl_macro)]

use mysql::prelude::Queryable;
use mysql::Pool;
use rand::{distributions::Alphanumeric, Rng};
use rocket::request::Form;
use rocket::{http::Cookie, http::Cookies, response::Redirect};
use rocket::{response::content, State};

#[macro_use]
extern crate rocket;

#[derive(FromForm)]
struct LoginData {
    username: String,
    password: String,
}

#[get("/logout")]
fn logout(mut cookies: Cookies) -> content::Html<&'static str> {
    cookies.remove_private(Cookie::named("user_id"));

    content::Html(
        "
    <h1>Logged out</h1>
    ",
    )
}

#[get("/login")]
fn login() -> content::Html<&'static str> {
    content::Html(
        "
    <h1>Login</h1>
    <form method='POST' action='/do_login'>
        <input type='text' name='username' />
        <input type='text' name='password' />
        <input type='submit' />
    </form>
    ",
    )
}

#[post("/do_login", data = "<login>")]
fn do_login(
    login: Form<LoginData>,
    dbpool: State<Pool>,
    mut cookies: Cookies,
) -> content::Html<String> {
    let mut conn = dbpool.get_conn().unwrap();

    let users = conn
        .exec::<(u32, String, Vec<u8>, String), _, _>(
            "SELECT id, username, passhash, passsalt FROM users WHERE username = ?",
            (&login.username,),
        )
        .unwrap();

    if users.len() != 1 {
        return content::Html(String::from("No user"));
    }

    let (id, username, passhash, passsalt) = users[0].clone();
    let hash_bytes = argon2rs::argon2i_simple(&login.password.as_str(), &passsalt);

    if hash_bytes != passhash.as_slice() {
        return content::Html(String::from("Bad pass"));
    }

    cookies.add_private(Cookie::new("user_id", id.to_string()));

    content::Html(format!("Logged in as {}", username))
}

#[get("/newuser")]
fn newuser() -> content::Html<&'static str> {
    content::Html(
        "
    <h1>New User</h1>
    <form method='POST' action='/do_newuser'>
        <input type='text' name='username' />
        <input type='text' name='password' />
        <input type='submit' />
    </form>
    ",
    )
}

#[post("/do_newuser", data = "<login>")]
fn do_newuser(login: Form<LoginData>, dbpool: State<Pool>) -> Redirect {
    let salt: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let hash_bytes = argon2rs::argon2i_simple(&login.password.as_str(), &salt.as_str());
    let mut conn = dbpool.get_conn().unwrap();

    conn.exec_drop(
        "INSERT INTO users (username, passhash, passsalt) VALUES (?, ?, ?)",
        (&login.username, &hash_bytes, &salt),
    )
    .unwrap();

    Redirect::to("/")
}

#[get("/")]
fn index(dbpool: State<Pool>, mut cookies: Cookies) -> content::Html<String> {
    let maybe_user = cookies.get_private("user_id");

    if let Some(user_id) = maybe_user {
        let mut conn = dbpool.get_conn().unwrap();
        let users = conn
            .query_map(
                "SELECT id, username, passsalt FROM users",
                |(id, username, passsalt): (u32, String, String)| {
                    format!("{} {} {}", id, username, passsalt)
                },
            )
            .unwrap();

        content::Html(format!(
            "Logged in as {}<br>{}",
            user_id,
            users.join("<br>")
        ))
    } else {
        content::Html(String::from("<a href=/login>Need to log in</a>"))
    }
}

fn main() {
    populate_debug_db();

    let url = "mysql://root:iZR3xjHNtAYVLqIU@localhost:3306/rocketapp";
    let dbpool = Pool::new(url).unwrap();

    rocket::ignite()
        .manage(dbpool)
        .mount(
            "/static",
            rocket_contrib::serve::StaticFiles::from("./static"),
        )
        .mount(
            "/",
            routes![index, login, do_login, newuser, do_newuser, logout],
        )
        .launch();
}

fn populate_debug_db() {
    let url = "mysql://root:iZR3xjHNtAYVLqIU@localhost:3306/information_schema";
    let dbpool = Pool::new(url).unwrap();
    let conn = &mut dbpool.get_conn().unwrap();

    let user_tables = conn
        .query::<u32, _>("SELECT table_id FROM INNODB_TABLES WHERE name = \"rocketapp/users\"")
        .unwrap()
        .len();

    if user_tables == 1 {
        return;
    }

    conn.query_drop("CREATE DATABASE rocketapp").unwrap();

    let url = "mysql://root:iZR3xjHNtAYVLqIU@localhost:3306/rocketapp";
    let dbpool = Pool::new(url).unwrap();
    let conn = &mut dbpool.get_conn().unwrap();

    conn.query_drop(
        "
    CREATE TABLE users (
        id INT NOT NULL AUTO_INCREMENT,
        username VARCHAR(255) NOT NULL,
        passhash BINARY(32),
        passsalt CHAR(32),
        PRIMARY KEY (ID)
    )",
    )
    .unwrap();
}
