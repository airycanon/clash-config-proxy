use actix_web::{get, web, App, HttpServer, Responder, Error, HttpResponse};
use std::fs;
use serde::{Serialize, Deserialize};
use actix_web::client::Client;
use serde_yaml::{Value, Mapping, Number};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
struct Config {
    remote: String,
    port: i32,
    token: String,
    proxies: Vec<Proxy>,
    rules: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
struct Proxy {
    name: String,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    protocol: String,
    username: String,
    password: String,
    server: String,
    port: i32,
}


#[get("/config.yaml")]
async fn index(config: web::Data<Config>) -> impl Responder {

    // load remote config
    let client = Client::default();

    let mut resp = client
        .get(config.remote.as_str())
        .send()
        .await
        .map_err(Error::from)
        .unwrap();

    let body = String::from_utf8(resp.body().await.unwrap().to_vec()).unwrap();
    let mut map: BTreeMap<String, Value> = serde_yaml::from_str(body.as_str()).unwrap();

    // prepend proxies
    let proxies = map["proxies"].as_sequence().unwrap();
    let mut new_proxies = vec![];

    for proxy in config.proxies.clone().into_iter() {
        let mut map = Mapping::new();
        map.insert(Value::String("name".to_string()), Value::String(proxy.name));
        map.insert(Value::String("type".to_string()), Value::String(proxy.protocol));
        map.insert(Value::String("username".to_string()), Value::String(proxy.username));
        map.insert(Value::String("password".to_string()), Value::String(proxy.password));
        map.insert(Value::String("server".to_string()), Value::String(proxy.server));
        map.insert(Value::String("port".to_string()), Value::Number(Number::from(proxy.port)));

        new_proxies.push(Value::Mapping(map));

        for v in proxies.clone().into_iter() {
            new_proxies.push(v)
        }
    }
    map.insert("proxies".to_string(), Value::Sequence(new_proxies));

    // prepend rules
    let rules = map["proxies"].as_sequence().unwrap();
    let mut new_rules = vec![];
    for rule in config.rules.clone().into_iter() {
        new_rules.push(Value::String(rule));

        for v in rules.clone().into_iter() {
            new_rules.push(v)
        }
    }
    map.insert("rules".to_string(), Value::Sequence(new_rules));

    // todo prepend rule providers
    let yaml = serde_yaml::to_string(&map).unwrap();
    HttpResponse::Ok().content_type("text/plain; charset=utf-8").body(yaml)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let contents = fs::read_to_string("./config.yaml")
        .expect("Something went wrong reading the file");

    let config: Config = serde_yaml::from_str(contents.as_str()).unwrap_or_default();

    let addr = format!("127.0.0.1:{}", config.port);

    println!("start server at {}", addr);

    HttpServer::new(move || App::new()
        .data(config.clone())
        .service(index))
        .bind(addr)?
        .run()
        .await
}
