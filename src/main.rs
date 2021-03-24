#![feature(proc_macro_hygiene, decl_macro)]

use mysql::prelude::Queryable;
use mysql::Pool;
use rocket::State;
use rocket::{http::Cookie, response::Body};
use rocket::{http::Cookies, Response};
use std::io::Cursor;

#[macro_use]
extern crate rocket;

#[get("/")]
fn index(dbpool: State<Pool>, mut cookies: Cookies) -> String {
    println!(
        "Getting hello cookie: {}",
        cookies.get_private("hello").unwrap()
    );

    let cookie = Cookie::build("hello", "shhhhh").finish();
    cookies.add_private(cookie);

    let mut conn = dbpool.get_conn().unwrap();
    let users = conn
        .query_map(
            "SELECT id, username, passhash FROM users",
            |(id, username, hashedpass): (u32, String, String)| {
                format!("{} {} {}", id, username, hashedpass)
            },
        )
        .unwrap();

    users.join("\n")
}

fn main() {
    populate_debug_db();

    let url = "mysql://root:iZR3xjHNtAYVLqIU@localhost:3306/rocketapp";
    let dbpool = Pool::new(url).unwrap();

    rocket::ignite()
        .manage(dbpool)
        .mount("/", routes![index])
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
