use std::{borrow::Cow, collections::HashMap, str::FromStr};

use icalendar::{
    parser::{read_calendar, unfold, ParseString},
    Calendar, CalendarComponent, Component, Event,
};
use serde_json::json;
use worker::*;

use regex;
mod utils;
use phf::phf_map;
use serde::{Deserialize, Serialize};

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

trait UrlExt {
    // Get first value of a query parameter with a key
    fn get_qs(&self, key: &str) -> Option<Cow<str>>;
}
impl UrlExt for Url {
    fn get_qs(&self, key: &str) -> Option<Cow<str>> {
        self.query_pairs()
            .filter(|(k, _)| k == key)
            .next()
            .map(|x| x.1)
    }
}

#[derive(Serialize, Deserialize)]
struct CalendarSettings{
    url: String,
    renames: HashMap<String, String>,
}

const BASE_URL: &str = "https://calendar.lim.fi";

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get_async("/:token", |mut req, ctx| async move {
            let kv = ctx.kv("MYCO_CAL_FILTERS")?;
            let token = match ctx.param("token") {
                Some(t) => t,
                None => { return Response::error("No token provided", 404) }
            };

            let cal_settings: CalendarSettings = match kv.get(token).bytes().await? {
                None => { return Response::error("No settings found for requested url", 404); }
                Some(bytes) => serde_cbor::from_slice(&bytes).expect("Got unexpected serialized data from KV store")
            };


            let req = Request::new(&cal_settings.url, Method::Get)?;
            let mut resp = Fetch::Request(
                req
            ).send().await?;
            let text = resp.text().await?;
            let mut cal = Calendar::from_str(&text)?.done();

            for component in &mut cal.components {
                match component {
                    CalendarComponent::Event(e) => {
                        println!("summary");
                        let summary = e.get_summary().unwrap()
                            .replace("\\,", ",")
                            .replace("\\;", ";")
                            .replace("\\n", "\n")
                            .replace("\\N", "\n")
                            .replace("\\\\", "\\");
                        let (event_name, rest) = summary.split_once(" / ").unwrap();
                        let (course_code_and_name, rest) = rest.split_once(", ").unwrap();
                        let (course_code, course_name) = course_code_and_name.split_once(" - ").unwrap();

                        let new_name = cal_settings.renames.get(course_name).map(|x| x.as_ref()).unwrap_or(course_name);
                        e.add_property("SUMMARY", format!("{} {}", new_name, event_name).as_ref());
                    },
                    _ => {}
                }
            }
            Response::ok(cal.to_string())
        })
        .post_async("/", |mut req, ctx| async move {
            let cal: CalendarSettings = match serde_json::from_str(req.text().await?.as_str()) {
                Ok(c) => c,
                Err(_) => { return Response::error("Invalid payload", 403); }
            };
            let url = match Url::parse(&cal.url) {
                Ok(u) => u,
                Err(_) => { return Response::error("Could not parse payload URL", 403); }
            };
            //let maybe_token =
            let token = match url.get_qs("authtoken") {
                None => { return Response::error("Calendar URL does not contain authtoken", 403); }
                Some(token) => token
            };
            let kv = ctx.kv("MYCO_CAL_FILTERS")?;
            kv.put_bytes(&token, &serde_cbor::to_vec(&cal).unwrap())?.execute().await?;

            Response::ok(
                serde_json::to_string(&json!(
                        {
                            "filtered_url": format!("{}/{}", BASE_URL, token)
                        }
                    ))?
            )
        })
        .run(req, env)
        .await
}
