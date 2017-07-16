#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate hyper;
extern crate hyper_native_tls;
extern crate chrono;

extern crate rocket;
extern crate rocket_contrib;

use std::env;
use hyper::Client;
use hyper::header::Connection;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use std::vec::Vec;
use std::collections::BTreeSet;
use chrono::{NaiveDate as Date, Utc};
use rocket::Outcome;
use chrono::Datelike;
use rocket_contrib::Template;

header! { (XToken, "x-token") => [String] }

static DEBITOOR_TOKEN: &'static str = "DEBITOOR_TOKEN";

#[derive(Serialize, Deserialize, Debug)]
struct Expense {
    date: String,
    lines: Vec<Line>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Line {
    #[serde(rename = "categoryType")]
    category_type: Option<String>,
    #[serde(rename = "netAmount")]
    net_amount: f64,
    description: String,
    #[serde(rename = "assetDepreciation")]
    #[serde(default = "Vec::new")]
    asset_depreciation: Vec<AssetDepreciation>,
}

#[derive(Serialize, Deserialize, Debug)]
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
        let token = request.cookies().get(DEBITOOR_TOKEN).map(|c| c.value().to_owned());

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

#[derive(Deserialize, Debug)]
struct BaseURL {
    base_url: String
}

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for BaseURL {
    type Error = ();

    fn from_request(request: &'a rocket::request::Request<'r>) -> rocket::request::Outcome<Self, Self::Error> {
        println!("Header: {:?}", request.headers());

        let schema = request.headers().get_one("X-Forwarded-Proto").unwrap_or("http");
        let host = request.headers().get_one("Host").unwrap();

        Outcome::Success(BaseURL {
            base_url: format!("{}://{}", schema, host)
        })
    }
}

#[derive(FromForm)]
struct CodeWrapper {
    code: String
}

fn create_ssl_client() -> Client {
    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    Client::with_connector(connector)
}

#[get("/?<code>", rank = 1)]
fn check_code(mut cookies: rocket::http::Cookies, code: CodeWrapper, base_url: BaseURL) -> rocket::response::Redirect {
    let client_secret = env::var("CLIENT_SECRET").unwrap();

    println!("got code {:?}", code.code);

    let client = create_ssl_client();

    let body = format!("code={}&client_secret={}&redirect_uri={}", code.code, client_secret, base_url.base_url);

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

#[allow(unused_variables)]
#[get("/", rank = 2)]
fn index(token: AccessToken) -> rocket::response::Redirect {
    rocket::response::Redirect::temporary(format!("/assets/{}", Utc::now().year()).as_str())
}

#[get("/", rank = 3)]
fn redirect_auth() -> rocket::response::Redirect {
    let client_id = env::var("CLIENT_ID").unwrap();

    rocket::response::Redirect::temporary(format!("https://app.debitoor.com/login/oauth2/authorize?client_id={}&response_type=code", client_id).as_str())
}

#[get("/assets/<year>")]
fn asset_list(token: AccessToken, year: i32) -> Template {
    let client = create_ssl_client();

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

    #[derive(Serialize, Deserialize, Debug)]
    struct AssetInformation {
        description: String,
        #[serde(rename = "netAmount")]
        net_amount: f64,
        #[serde(rename = "depreciationCost")]
        depreciation_cost: f64,
        #[serde(rename = "depreciationDate")]
        depreciation_date: Date,
        #[serde(rename = "bookValue")]
        book_value: f64,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Context {
        year: i32,
        asset_information: Vec<AssetInformation>,
        available_years: BTreeSet<i32>,
        total_depreciation_cost: f64,
        total_book_value: f64,
    }

    let mut asset_information: Vec<AssetInformation> = Vec::new();
    let mut available_years: BTreeSet<i32> = BTreeSet::new();

    println!("printing value");

    let mut total_depreciation_cost = 0.0f64;
    let mut total_book_value = 0.0f64;

    for expense in expenses {
        for line in expense.lines {
            let description = line.description.as_str();
            for asset_depreciation in line.asset_depreciation {
                available_years.insert(asset_depreciation.depreciation_date.year());
                if asset_depreciation.depreciation_date.year() == year {
                    total_depreciation_cost += asset_depreciation.depreciation_cost;
                    total_book_value += asset_depreciation.book_value;
                    asset_information.push(AssetInformation {
                        description: description.to_string(),
                        net_amount: line.net_amount,
                        depreciation_cost: asset_depreciation.depreciation_cost,
                        depreciation_date: asset_depreciation.depreciation_date,
                        book_value: asset_depreciation.book_value,
                    });
                }
            }
        }
    }

    println!("Rendering tenplate");

    Template::render("asset_list", Context {
        year: year,
        asset_information: asset_information,
        available_years: available_years,
        total_depreciation_cost: total_depreciation_cost,
        total_book_value: total_book_value
    })
}

fn main() {
    rocket::ignite().attach(Template::fairing()).mount("/", routes![index, asset_list, check_code, redirect_auth]).launch();
}
