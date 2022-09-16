use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use awc::Client;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Number, Value};
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
struct Config {
    #[serde(default)]
    sources: Vec<String>,
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

    let mut proxies = vec![];
    let mut rules = vec![];
    let mut rule_providers = Mapping::new();

    let mut all:BTreeMap<String, Value>  = BTreeMap::new();

    // prepend local config
    if !disabled {
        for proxy in config.proxies.clone().into_iter() {
            proxies.push(Value::Mapping(proxy.to_map()));
        }
        for rule in config.rules.clone().into_iter() {
            rules.push(Value::String(rule));
        }
        for (k, v) in config.rule_providers.clone().into_iter() {
            rule_providers.insert(Value::String(k), Value::Mapping(v.to_map()));
        }
    }

    // load remote config
    let client = Client::default();

    let mut sources = config.sources.clone();
    sources.dedup();

    for remote in sources.into_iter() {
        let mut resp = client.get(remote.as_str()).send().await.unwrap();

        if !resp.status().is_success() {
            return HttpResponse::InternalServerError().finish();
        }

        let body = String::from_utf8(resp.body().await.unwrap().to_vec()).unwrap();
        let resp: BTreeMap<String, Value> = serde_yaml::from_str(body.as_str()).unwrap();

        // same config in next config yml will be ignored 
        for (k, v) in resp.clone().into_iter() {
            all.entry(k).or_insert(v);
        }

        // prepend remote proxies
        let mut remote_proxies = vec![];
        if resp.contains_key("proxies") {
            remote_proxies = resp["proxies"].as_sequence().unwrap().to_owned();
        }
        for v in remote_proxies.clone().into_iter() {
            proxies.push(v)
        }

        // prepend remote rules
        let mut remote_rules = vec![];
        if resp.contains_key("rules") {
            remote_rules = resp["rules"].as_sequence().unwrap().to_owned();
        }
        for v in remote_rules.clone().into_iter() {
            rules.push(v)
        }

        // prepend remote rule-providers
        let mut rule_providers = Mapping::new();
        if resp.contains_key("rule-providers") {
            rule_providers = resp["rule-providers"].as_mapping().unwrap().to_owned();
        }
        for (k, v) in rule_providers.clone().into_iter() {
            rule_providers.insert(k, v);
        }
    }

    all.insert("proxies".to_string(), Value::Sequence(proxies));
    all.insert("rules".to_string(), Value::Sequence(rules));
    all.insert("rule-providers".to_string(), Value::Mapping(rule_providers));

    let yaml = serde_yaml::to_string(&all).unwrap();
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
