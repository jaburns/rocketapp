#![feature(proc_macro_hygiene, decl_macro)]

use mysql::prelude::Queryable;
use mysql::Pool;
use rocket::http::Cookies;
use rocket::request::Form;
use rocket::{response::content, State};

#[macro_use]
extern crate rocket;

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

#[derive(FromForm)]
struct LoginData {
    username: String,
    password: String,
}

#[post("/do_login", data = "<login>")]
fn do_login(login: Form<LoginData>) -> content::Html<String> {
    let (password, salt) = (login.password.as_str(), "saltysalty");
    println!("argon2i_simple(\"{}\", \"{}\"):", password, salt);

    let bytes = argon2rs::argon2i_simple(&password, &salt);

    let mut hash = String::new();
    for byte in bytes.iter() {
        hash += format!("{:02x}", byte).as_str();
    }
    println!("Hashed: {}", hash);

    let encoded = argon2rs::verifier::Encoded::default2i(
        &login.password.clone().into_bytes(),
        b"saltysalty",
        b"",
        b"",
    );
    let verified = encoded.verify(login.password.as_ref());

    println!("Verified: {}", verified);

    content::Html(format!("Submitted {} {}", login.username, login.password))
}

#[get("/")]
fn index(dbpool: State<Pool>, mut cookies: Cookies) -> content::Html<String> {
    let maybe_user = cookies.get_private("user_id");

    if let Some(user_id) = maybe_user {
        let mut conn = dbpool.get_conn().unwrap();
        let users = conn
            .query_map(
                "SELECT id, username, passhash FROM users",
                |(id, username, hashedpass): (u32, String, String)| {
                    format!("{} {} {}", id, username, hashedpass)
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
        .mount("/", routes![index, login, do_login])
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

    conn.query_drop("CREATE TABLE users ( id INT NOT NULL, username VARCHAR(255) NOT NULL, passhash VARCHAR(255), PRIMARY KEY (ID) )").unwrap();
    conn.query_drop("INSERT INTO users (id, username, passhash) VALUES (0, 'user0', 'pass0')")
        .unwrap();
    conn.query_drop("INSERT INTO users (id, username, passhash) VALUES (1, 'user1', 'pass1')")
        .unwrap();
}
