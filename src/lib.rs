extern crate hyper;
extern crate rustc_serialize;

mod types;
mod util;

pub use types::*;
use util::Params;

use rustc_serialize::{json, Decodable};
use std::io::Read;
use std::fmt;
use hyper::{Client, Url};
use hyper::header::Connection;

const API_URL : &'static str = "https://api.telegram.org/bot";

/// The main type for doing everything.
pub struct Bot {
    offset: Integer,
    url: Url,
    client: Client,
}

impl Bot {
    /// Creates a new bot with the given token. If the token is completely
    /// invalid (resulting in an invalid API-URL), the function will panic.
    /// However, the function will not check if the given token is a valid
    /// Telegram token. You can call `get_me` to do a test request.
    pub fn new(token: String) -> Bot {
        let url = match Url::parse(&*format!("{}{}/dummy", API_URL, token)) {
            Ok(url) => url,
            Err(e) => panic!("Invalid token! ({})", e),
        };
        Bot {
            offset: 0,
            url: url,
            client: Client::new(),
        }
    }

    fn send_request<T: Decodable>(&mut self, method: &str, p: Params)
                   -> Result<T> {
        // Prepare URL for request: Clone and change the last path fragment
        // to the method name and append GET parameters.
        let mut url = self.url.clone();
        url.path_mut().map(|path| {         // if theres a path: Change it
            path.last_mut().map(|last| {    // if its not empty: Change last...
                *last = method.into()       // ... into method name
            })
        });

        // For alle (str, String) pairs: Map to (str, str) and append it to URL
        let it = p.get_params().iter().map(|&(k, ref v)| (k, &**v));
        url.set_query_from_pairs(it);

        // Prepare HTTP Request
        let req = self.client.get(url).header(Connection::close());

        // Send request and check if it failed
        let mut resp = match req.send() {
            Ok(resp) => resp,
            Err(e) => return Err(Error::Http(e)),
        };

        // Read response into String and return error if it failed
        let mut body = String::new();
        if let Err(e) = resp.read_to_string(&mut body) {
            return Err(Error::Io(e));
        }

        // Try to decode response as JSON representing a Response
        match json::decode(&*body) {
            // If decoding JSON fails: Return JSON-Error
            Err(e) => Err(Error::Json(e)),
            // If JSON decoding was ok, but the response says that there was
            // an error: Return API-Error with the given description
            Ok(Response { ok: false, description: Some(desc), ..}) => {
                Err(Error::Api(desc))
            },
            // If JSON decoding and response are "ok": Return the result.
            Ok(Response { ok: true, result: Some(res), ..}) => {
                Ok(res)
            },
            // This should never occur: If "ok"==false, "description" should
            // always be Some. If "ok"==true, then "result" should always be
            // Some. We could also panic in this case.
            _ => Err(Error::InvalidState("Invalid server response".into())),
        }
    }

    /// Corresponds to the "getMe" method of the API.
    pub fn get_me(&mut self) -> Result<User> {
        // Execute request with empty parameter list
        self.send_request("getMe", Params::new())
    }

    /// Corresponds to the "getUpdates" method of the API.
    ///
    /// **Note:**
    /// The method will not set the offset parameter on its own. To receive
    /// updates in a more high level way, see `long_poll`.
    pub fn get_updates(&mut self, offset: Option<Integer>,
                       limit: Option<Integer>, timeout: Option<Integer>)
                       -> Result<Vec<Update>> {
        // Prepare parameters
        let mut params = Params::new();
        params.add_get_opt("offset", offset);
        params.add_get_opt("limit", limit);
        params.add_get_opt("timeout", timeout);

        // Execute request
        self.send_request("getUpdates", params)
    }

    /// Receive and handle updates via "getUpdates".
    ///
    /// This method will repeatedly call `get_updates` to receive new updates.
    /// It will then call the given handler for every update and increase the
    /// update offset accordingly, so the handler will never be called with
    /// the same update twice.
    /// The `timeout` parameter influences how long (in seconds) each poll may
    /// last. Defaults to 30.
    ///
    /// **Note:**
    /// If the bot is restarted, but the last received updates are not yet
    /// confirmed (the last poll was not empty), there will be some duplicate
    /// updates.
    pub fn long_poll<F>(&mut self, timeout: Option<Integer>, handler: F)
                        -> Result<()>
                        where F: Fn(&mut Bot, Update) {
        // Calculate final timeout: Given or default.
        let timeout = Some(if let Some(t) = timeout { t } else { 30 });

        loop {
            // Receive updates with correct offset
            let offset = Some(self.offset);
            let updates = try!(self.get_updates(offset, None, timeout));

            // For every update: Increase the offset and call the handler.
            for u in updates {
                if u.update_id >= self.offset {
                    self.offset = u.update_id + 1;
                }

                handler(self, u);
            }
        }
    }
}

/// Telegram-Bot Result
pub type Result<T> = std::result::Result<T, Error>;

/// Telegram-Bot Error.
#[derive(Debug)]
pub enum Error {
    Http(hyper::error::Error),
    Io(std::io::Error),
    Json(json::DecoderError),
    Api(String),
    InvalidState(String),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Http(ref e) => e.description(),
            Error::Io(ref e) => e.description(),
            Error::Json(ref e) => e.description(),
            Error::Api(ref s) => &*s,
            Error::InvalidState(ref s) => &*s,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Http(ref e) => e.fmt(f),
            Error::Io(ref e) => e.fmt(f),
            Error::Json(ref e) => e.fmt(f),
            Error::Api(ref s) => s.fmt(f),
            Error::InvalidState(ref s) => s.fmt(f),
        }
    }
}


#[test]
fn it_works() {
}
