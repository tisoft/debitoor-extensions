#![feature(proc_macro)]
#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate hyper;
extern crate chrono;

extern crate rocket;

use std::env;
use hyper::Client;
use hyper::header::Connection;
use std::vec::Vec;
use std::ops::Deref;
use chrono::UTC;
use serde::{Deserialize, Deserializer};
use std::str::FromStr;
use rocket::Outcome;

header! { (XToken, "x-token") => [String] }

static DEBITOOR_TOKEN: &'static str = "DEBITOOR_TOKEN";

// This single-element tuple struct is called a newtype struct.
#[derive(Debug)]
struct Date(chrono::Date<UTC>);

impl Deserialize for Date {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        struct Visitor;

        impl ::serde::de::Visitor for Visitor {
            type Value = Date;

            fn visit_str<E>(&mut self, value: &str) -> Result<Date, E>
                where E: ::serde::de::Error,
            {
                Ok(Date(chrono::Date::from_utc(chrono::naive::date::NaiveDate::from_str(value).unwrap(), UTC)))
            }
        }

        // Deserialize the enum from a string.
        deserializer.deserialize_str(Visitor)
    }
}

// Enable `Deref` coercion.
impl Deref for Date {
    type Target = chrono::Date<UTC>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Deserialize, Debug)]
struct Expense {
    date: String,
    lines: Vec<Line>,
}

#[derive(Deserialize, Debug)]
struct Line {
    #[serde(rename = "categoryType")]
    category_type: Option<String>,
    description: String,
    #[serde(rename = "assetDepreciation")]
    #[serde(default = "Vec::new")]
    asset_depreciation: Vec<AssetDepreciation>,
}

#[derive(Deserialize, Debug)]
struct AssetDepreciation {
    #[serde(rename = "depreciationCost")]
    depreciation_cost: f64,
    #[serde(rename = "depreciationDate")]
    depreciation_date: Date,
    #[serde(rename = "bookValue")]
    book_value: f64,
}

#[derive(Deserialize, Debug)]
struct AccessToken {
    access_token: String
}

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for AccessToken {
    type Error = ();

    fn from_request(request: &'a rocket::request::Request<'r>) -> rocket::request::Outcome<Self, Self::Error> {
        let token = request.cookies().find(DEBITOOR_TOKEN).map(|c| c.value.to_owned());

        println!("Header: {:?}", request.headers());

        match token {
            None => {
                Outcome::Forward(())
            }
            Some(token) => {
                Outcome::Success(AccessToken {
                    access_token: token
                })
            }
        }
    }
}

#[derive(FromForm)]
struct CodeWrapper<'r> {
    code: &'r str
}

#[get("/?<code>", rank = 1)]
fn check_code(cookies: &rocket::http::Cookies, code: CodeWrapper) -> rocket::response::Redirect {
    let client_secret = env::var("CLIENT_SECRET").unwrap();

    println!("got code {:?}", code.code);

    let client = Client::new();

    let body = format!("code={}&client_secret={}&redirect_uri=http://localhost:8080/", code.code, client_secret);

    println!("body {}", body);

    let res = client.
        post("https://app.debitoor.com/login/oauth2/access_token").
        //if we keep the connection open the parsing will wait for a minute in between for a timeout
        //don't know why this is, so just disable keep alive for now
        body(body.as_bytes()).
        header(Connection::close()).
        header(hyper::header::ContentType::form_url_encoded()).
        //the access token to authenticate with
        send().unwrap();

    assert_eq!(res.status, hyper::Ok);

    let access_token: AccessToken = serde_json::from_reader(res).unwrap();

    println!("{:?}", access_token);

    //set cookie and redirect
    cookies.add(rocket::http::Cookie::new(DEBITOOR_TOKEN.to_owned(), access_token.access_token.to_owned()));
    rocket::response::Redirect::temporary("/")
}

#[get("/", rank = 2)]
fn asset_list(token: AccessToken) -> String {
    let client = Client::new();

    println!("send request for token {:?}", token);
    let res = client.
        get("https://api.debitoor.com/api/expenses/v3").
        //if we keep the connection open the parsing will wait for a minute in between for a timeout
        //don't know why this is, so just disable keep alive for now
        header(Connection::close()).
        //the access token to authenticate with
        header(XToken(token.access_token.to_owned())).
        send().unwrap();
    assert_eq!(res.status, hyper::Ok);

    println!("create parser");

    let expenses: Vec<Expense> = serde_json::from_reader(res).unwrap();

    println!("printing value");

    let mut asset_string = "".to_string();

    for expense in expenses {
        for line in expense.lines.iter().filter(|line| line.category_type == Some("asset".to_string())) {
            println!("{:?}", line);
            asset_string = asset_string + &*format!("{:?}\n", line);
        }
    }

    println!("Sending response");

    return asset_string;
}

#[get("/", rank = 3)]
fn redirect_auth() -> rocket::response::Redirect {
    let client_id = env::var("CLIENT_ID").unwrap();

    rocket::response::Redirect::temporary(format!("https://app.debitoor.com/login/oauth2/authorize?client_id={}&response_type=code", client_id).as_str())
}

fn main() {
    rocket::ignite().mount("/", routes![asset_list, check_code, redirect_auth]).launch();
}
