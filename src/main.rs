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

header! { (XToken, "x-token") => [String] }

include!(concat!(env!("OUT_DIR"), "/serde_types.rs"));

fn main() {
    let port = env::var("DEBITOOR_EXTENSIONS_PORT").or(Ok::<String, std::env::VarError>("8080".to_string())).unwrap().parse::<i32>().unwrap();

    fn hello(serverReq: hyper::server::Request, serverRes: hyper::server::Response) {
        //println!("Incoming request {:?}", req);

        let token = env::args().skip(1).next().unwrap();

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
        serverRes.send(asset_string.as_bytes()).unwrap();

        println!("all done");
    }

    println!("listening on {:?}", port);
    Server::http(&*format!("0.0.0.0:{}", port)).unwrap().handle(hello).unwrap();
}
