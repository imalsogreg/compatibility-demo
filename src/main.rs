use serde::{Serialize, Deserialize};
use serde_json;


/// Our "original" version of the types and their serialization formats.
pub mod v0 {
    use serde::{Serialize, Deserialize};
    #[derive(Serialize, Deserialize)]
    pub struct GreetingRequest {
        pub name: String,
        pub favorite_thing: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Greeting {
        pub name: String,
        pub greeting: String,
    }
}

/// Our "updated" version of the types and their serialization formats.
pub mod v1 {
    use serde::{Serialize, Deserialize};
    #[derive(Serialize, Deserialize)]
    pub struct GreetingRequest {
        pub name: String,
        pub favorite_thing: String,
        pub favorite_song: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Greeting {
        pub greeting: String,
    }
}

fn save<T: Serialize>(t: &T) -> String {
    serde_json::to_string(t).expect("Serde will not fail to encode")
}

fn load<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod basic_tests {
    use super::{load, save};
    use crate::v0;
    use super::v1;

    // #[test]
    fn greeting_change_is_backward_compatible() {
        let old_data = save(&v0::Greeting { name: "Greg".to_string(), greeting: "Hi greg".to_string() });
        assert!( load::<v1::Greeting>(&old_data).is_ok() );
    }

    // #[test]
    fn greeting_change_is_not_forward_compatible() {
        let new_data = save(&v1::Greeting { greeting: "Hi greg".to_string() });
        assert!( load::<v0::Greeting>(&new_data).is_err() );
    }


}

#[cfg(test)]
mod database_tests {
    use super::{load, save};
    use crate::v0;
    use super::v1;

    struct Database{ entries: Vec<String> }
    impl Database {
        pub fn new() -> Database {
            Database { entries: vec![] }
        }

        pub fn insert<T: serde::Serialize>(&mut self, t: &T) {
            self.entries.push( save(t) )
        }

        pub fn read_all<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<Vec<T>, serde_json::Error> {
            self.entries.iter().map(|s| load(s) ).collect()
        }
    }

    // #[test]
    fn databases_require_forward_compatibile_changes() {
        let mut database = Database::new();

        // Remember that the diff for Greeting is backward compatible but not
        // forward compatible.

        // Insert original data.
        database.insert(&v0::Greeting { name: "Greg".to_string(), greeting: "Hi greg".to_string() });
        // Insert a new value that is not forward compatible.
        database.insert(&v1::Greeting {greeting: "Hi greg".to_string()});

        // New versions of the server can read the data.
        assert!( database.read_all::<v1::Greeting>().is_ok() );

        // But old versions of the server fail to read.
        assert!( database.read_all::<v0::Greeting>().is_err() );

    }

    // #[test]
    fn databases_require_backward_compatibile_changes() {
        let mut database = Database::new();

        // Remember that the diff for GreetingRequest is forward compatible but not
        // backward compatible.

        // Insert original data.
        database.insert(&v0::GreetingRequest { name: "Greg".to_string(), favorite_thing: "Rust".to_string() });
        // Insert a new value that is not forward compatible.
        database.insert(&v1::GreetingRequest { name: "Greg".to_string(), favorite_thing: "Rust".to_string(), favorite_song: "Never gonna give you up".to_string() });

        // Old versions of the server fail to read.
        assert!( database.read_all::<v0::GreetingRequest>().is_ok() );

        // But new versions of the server can not read the data.
        assert!( database.read_all::<v1::GreetingRequest>().is_err() );


    }

}

#[cfg(test)]
mod server_client_tests {

    use serde::{Serialize, Deserialize};
    use serde_json;
    use super::{load, save};
    use crate::v0;
    use super::v1;
    use std::marker::PhantomData;

    #[derive(Serialize, Deserialize, Default)]
    struct ReqOld {
        name: String,
    }

    #[derive(Serialize, Deserialize, Default, Debug)]
    struct RespOld {
        greeting: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    struct ReqNew {
        name: String, // Deleting this field would not be forward-compatible.
    }

    #[derive(Serialize, Deserialize, Default, Debug)]
    struct RespNew {
        greeting: String,
    }

    struct Client { request: String, handle_response: Box<dyn Fn(String) -> Result<String, serde_json::Error>> }
    struct Server { handle_request: Box<dyn Fn(String) -> Result<String, serde_json::Error>> }

    fn run_network(client: Client, server: Server) -> Result<String, serde_json::Error> {
        let request = client.request;
        let response = (*server.handle_request)(request)?;
        (*client.handle_response)(response)
    }

    fn make_v0_client() -> Client {
        Client {
            request: save(&ReqOld::default()),
            handle_response: Box::new(|resp| {
                let resp = load::<RespOld>(&resp).expect(&format!("response should decode: {resp}"));
                Ok(format!("Response: {resp:?}"))
            })
        }
    }

    fn make_v0_server() -> Server {
        Server {
            handle_request: Box::new( |req| {
                let ReqOld { name } = load(&req).expect("request should decode");
                let resp = RespOld::default();
                Ok(save(&resp))
            } )
        }
    }


    fn make_v1_client() -> Client {
        Client {
            request: save(&ReqNew::default()),
            handle_response: Box::new(|resp| {
                let resp = load::<RespNew>(&resp)?;
                Ok(format!("Response: {resp:?}"))
            })
        }
    }

    fn make_v1_server() -> Server {
        Server {
            handle_request: Box::new( |req| {
                let ReqNew { .. } = load(&req)?;
                let resp = RespNew::default();
                Ok(save(&resp))
            } )
        }
    }

    #[test]
    fn requests_are_backward_compatible() {
        assert!( load::<ReqNew>( &save(&ReqOld::default()) ).is_ok() );
    }

    #[test]
    fn requests_are_forward_compatible() {
        assert!( load::<ReqOld>( &save(&ReqNew::default()) ).is_ok() );
    }

    #[test]
    fn responses_are_backward_compatible() {
        assert!( load::<RespNew>( &save(&RespOld::default()) ).is_ok() );
    }

    #[test]
    fn responses_are_forward_compatible() {
        assert!( load::<RespOld>( &save(&RespNew::default()) ).is_ok() );
    }

    #[test]
    fn server_update_is_ok() {
        assert!( run_network( make_v0_client(), make_v1_server() ).is_ok() );
    }
}


fn main() {
    println!("Hello, world!");
}
