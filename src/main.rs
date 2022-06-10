use actix_web::{get, web, App, HttpServer, Responder, HttpResponse};
use std::fs;
use serde::{Serialize, Deserialize};
use awc::Client;
use serde_yaml::{Value, Mapping, Number};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
struct Config {
    remote: String,
    port: i32,
    token: String,
    #[serde(default)]
    proxies: Vec<Proxy>,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default, rename(deserialize = "rule-providers", serialize = "rule-providers"))]
    rule_providers: BTreeMap<String, RuleProvider>,
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
struct RuleProvider {
    #[serde(rename(deserialize = "type", serialize = "type"))]
    protocol: String,
    behavior: String,
    url: String,
    path: String,
    interval: i32,
}

impl Proxy {
    pub fn to_map(&self) -> Mapping {
        let clone = self.clone();

        let mut map = Mapping::new();
        map.insert(Value::String("name".to_string()), Value::String(clone.name));
        map.insert(Value::String("type".to_string()), Value::String(clone.protocol));
        map.insert(Value::String("username".to_string()), Value::String(clone.username));
        map.insert(Value::String("password".to_string()), Value::String(clone.password));
        map.insert(Value::String("server".to_string()), Value::String(clone.server));
        map.insert(Value::String("port".to_string()), Value::Number(Number::from(clone.port)));

        map
    }
}

impl RuleProvider {
    pub fn to_map(&self) -> Mapping {
        let clone = self.clone();

        let mut map = Mapping::new();
        map.insert(Value::String("type".to_string()), Value::String(clone.protocol));
        map.insert(Value::String("behavior".to_string()), Value::String(clone.behavior));
        map.insert(Value::String("url".to_string()), Value::String(clone.url));
        map.insert(Value::String("path".to_string()), Value::String(clone.path));
        map.insert(Value::String("interval".to_string()), Value::Number(Number::from(clone.interval)));

        map
    }
}


#[derive(Deserialize)]
pub struct Query {
    token: String,
    disable: Option<bool>,
}

#[get("/config.yaml")]
async fn index(config: web::Data<Config>, web::Query(query): web::Query<Query>) -> impl Responder {
    if query.token != config.token {
        return HttpResponse::NotFound().finish();
    }

   let disabled = query.disable.unwrap_or(false);

    // load remote config
    let client = Client::default();

    let mut resp = client
        .get(config.remote.as_str())
        .send()
        .await
        .unwrap();

    if !resp.status().is_success() {
        return HttpResponse::InternalServerError().finish();
    }

    let body = String::from_utf8(resp.body().await.unwrap().to_vec()).unwrap();
    let mut map: BTreeMap<String, Value> = serde_yaml::from_str(body.as_str()).unwrap();

    // prepend proxies
    let mut proxies = vec![];
    if map.contains_key("proxies") {
        proxies = map["proxies"].as_sequence().unwrap().to_owned();
    }

    let mut new_proxies = vec![];
    if !disabled {
        for proxy in config.proxies.clone().into_iter() {
            new_proxies.push(Value::Mapping(proxy.to_map()));
        }
    }

    for v in proxies.clone().into_iter() {
        new_proxies.push(v)
    }

    map.insert("proxies".to_string(), Value::Sequence(new_proxies));

    // prepend rules
    let mut rules = vec![];
    if map.contains_key("rules") {
        rules = map["rules"].as_sequence().unwrap().to_owned();
    }

    let mut new_rules = vec![];
    if !disabled {
        for rule in config.rules.clone().into_iter() {
            new_rules.push(Value::String(rule));
        }
    }

    for v in rules.clone().into_iter() {
        new_rules.push(v)
    }

    map.insert("rules".to_string(), Value::Sequence(new_rules));

    // prepend rule-providers
    let mut rule_providers = Mapping::new();
    if map.contains_key("rule-providers") {
        rule_providers = map["rule-providers"].as_mapping().unwrap().to_owned();
    }

    let mut new_rule_providers = Mapping::new();
    if !disabled {
        for (k, v) in config.rule_providers.clone().into_iter() {
            new_rule_providers.insert(Value::String(k), Value::Mapping(v.to_map()));
        }
    }

    for (k, v) in rule_providers.clone().into_iter() {
        new_rule_providers.insert(k, v);
    }

    map.insert("rule-providers".to_string(), Value::Mapping(new_rule_providers));

    let yaml = serde_yaml::to_string(&map).unwrap();
    HttpResponse::Ok().content_type("text/plain; charset=utf-8").body(yaml)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let contents = fs::read_to_string("./config.yaml")
        .expect("Something went wrong reading the file");

    let config: Config = serde_yaml::from_str(contents.as_str()).unwrap_or_default();

    let addr = format!("0.0.0.0:{}", config.port);
    println!("start server at {}", addr);

    HttpServer::new(move || App::new()
        .app_data( web::Data::new(config.clone()))
        .service(index))
        .bind(addr)?
        .run()
        .await
}
