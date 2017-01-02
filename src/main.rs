extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate hyper;
extern crate chrono;

use std::env;
use hyper::Client;
use hyper::Server;
use hyper::header::Connection;
use std::vec::Vec;
use hyper::client::IntoUrl;

header! { (XToken, "x-token") => [String] }

include!(concat!(env!("OUT_DIR"), "/serde_types.rs"));

static DEBITOOR_TOKEN: &'static str = "DEBITOOR_TOKEN";

fn main() {
    let port = env::var("DEBITOOR_EXTENSIONS_PORT").or(Ok::<String, std::env::VarError>("8080".to_string())).unwrap().parse::<i32>().unwrap();

    fn hello(server_req: hyper::server::Request, mut server_res: hyper::server::Response) {
        let client_id = env::var("CLIENT_ID").unwrap();
        let client_secret = env::var("CLIENT_SECRET").unwrap();

        println!("Incoming request for {:?}", server_req.uri);
        let token = server_req.headers.get::<hyper::header::Cookie>().
            and_then(|c| c.0.iter().find(|c| c.name == DEBITOOR_TOKEN)).map(|c| c.value.to_owned());

        match token {
            None => {
                //No token, do we have a code?
                match server_req.uri {
                    hyper::uri::RequestUri::AbsolutePath(ref uri) if uri.contains("code=") => {
                        //we have a code, get a token, set the cookie and redirect back to same page without code
                        let url = format!("http://localhost/{}", uri).into_url().unwrap();
                        let code = url.query_pairs().find(|q| q.0 == "code").unwrap().1;
                        println!("got code {:?}", code);

                        let client = Client::new();

                        let body = format!("code={}&client_secret={}&redirect_uri=http://localhost:8080/", code, client_secret);

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
                        server_res.headers_mut().set(hyper::header::SetCookie(vec![
                        hyper::header::CookiePair::new(DEBITOOR_TOKEN.to_owned(), access_token.access_token.to_owned())
                        ]));
                        server_res.headers_mut().set(hyper::header::Location("/".to_owned()));
                        *server_res.status_mut() = hyper::status::StatusCode::TemporaryRedirect;
                    }
                    _ => {
                        //redirect to debitoor
                        println!("not authenticated, redirecting to debitoor");
                        server_res.headers_mut().set(hyper::header::Location(format!("https://app.debitoor.com/login/oauth2/authorize?client_id={}&response_type=code", client_id).to_owned()));
                        *server_res.status_mut() = hyper::status::StatusCode::TemporaryRedirect;
                    }
                }
            }
            Some(token) => {
                //already have a token, proceed
                println!("Incoming token {:?}", token);

                let client = Client::new();

                println!("send request for token {}", token);
                let res = client.
                    get("https://api.debitoor.com/api/expenses/v3").
                    //if we keep the connection open the parsing will wait for a minute in between for a timeout
                    //don't know why this is, so just disable keep alive for now
                    header(Connection::close()).
                    //the access token to authenticate with
                    header(XToken(token.to_owned())).
                    send().unwrap();
                assert_eq!(res.status, hyper::Ok);

                println!("create parser");

                let expenses: Vec<Expense> = serde_json::from_reader(res).unwrap();

                println!("printing value");

                let mut asset_string = "".to_string();

                for expense in expenses {
                    for line in expense.lines.iter().filter(|line| line.category_type == "asset") {
                        println!("{:?}", line);
                        asset_string = asset_string + &*format!("{:?}\n", line);
                    }
                }

                println!("Sending response");
                server_res.send(asset_string.as_bytes()).unwrap();
            }
        }

        println!("all done");
    }

    println!("listening on {:?}", port);
    Server::http(&*format!("0.0.0.0:{}", port)).unwrap().handle(hello).unwrap();
}
