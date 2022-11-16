use std::io::Cursor;
use std::process::Command;

use thiserror::Error;

#[macro_use]
extern crate rocket;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment, Profile,
};
use rocket::fairing::AdHoc;
use rocket::http::ContentType;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::serde::{Deserialize, Serialize};
use rocket::State;

use git2::Repository;
mod git_helper;
use git_helper::*;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Config {
    remote_name: String,
    remote_branch: String,
    site_directory: String,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            remote_name: String::from("origin"),
            remote_branch: String::from("main"),
            site_directory: String::from("/data/site"),
        }
    }
}

#[derive(Error, Debug)]
enum AppError {
    #[error("Error from git operation")]
    GitError(#[from] git2::Error),
    #[error("IOError when executing command")]
    IOError(#[from] std::io::Error),
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let res = format!("{:?}\n", self.to_string());
        eprintln!("{:?}", self);
        Response::build()
            .header(ContentType::Plain)
            .status(Status::InternalServerError)
            .sized_body(res.len(), Cursor::new(res))
            .ok()
    }
}

#[get("/<_..>")]
fn all(config: &State<Config>) -> Result<String, AppError> {
    let repo = Repository::open(&config.site_directory)?;
    let mut remote = repo.find_remote(&config.remote_name)?;
    let fetch_commit = do_fetch(&repo, &[&config.remote_branch], &mut remote)?;
    do_merge(&repo, &(config.remote_branch), fetch_commit)?;

    let result = Command::new("zola")
        .arg("build")
        .current_dir(&config.site_directory)
        .output()?;

    println!("Hooked");
    Ok(String::from_utf8(result.stdout).unwrap())
}

#[launch]
fn rocket() -> _ {
    let figment = Figment::from(rocket::Config::default())
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file("App.toml").nested())
        .merge(Env::prefixed("APP_").global())
        .select(Profile::from_env_or("APP_PROFILE", "default"));

    rocket::custom(figment)
        .attach(AdHoc::config::<Config>())
        .mount("/", routes![all])
}
