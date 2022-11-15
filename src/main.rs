use std::io::Cursor;
use std::process::Command;

use thiserror::Error;

#[macro_use]
extern crate rocket;
use rocket::http::ContentType;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use git2::Repository;
mod git_helper;
use git_helper::*;

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
fn all() -> Result<String, AppError> {
    let remote_name = "origin";
    let remote_branch = "master";
    let repo = Repository::open("/tmp/auie")?;
    let mut remote = repo.find_remote(remote_name)?;
    let fetch_commit = do_fetch(&repo, &[remote_branch], &mut remote)?;
    do_merge(&repo, &remote_branch, fetch_commit)?;

    let result = Command::new("zola")
        .arg("build")
        .current_dir("/tmp/auie/myblog")
        .output()?;

    println!("Hooked");
    Ok(String::from_utf8(result.stdout).unwrap())
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![all])
}