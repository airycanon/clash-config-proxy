use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use awc::Client;
use clash_config_proxy::{create_config,find_config, models::{self, Manifest,Config}};
use serde::{Deserialize};
use serde_yaml::{Mapping, Value};
use std::{collections::BTreeMap, fs};

#[post("/configs")]
async fn post_config(config: web::Json<Config>) -> impl Responder {
    let mut config = config.clone();
    config.id = create_config(&config) as i32;

    HttpResponse::Ok().json(config)
}

#[derive(Deserialize)]
pub struct Query {
    #[serde(default)]
    token: String,
    disable: Option<bool>,
}

#[get("/configs/{name}")]
async fn get_config(
    name: web::Path<String>,
    manifest: web::Data<Manifest>,
    web::Query(query): web::Query<Query>,
) -> impl Responder {
    if query.token != manifest.token {
        return HttpResponse::Forbidden().finish();
    }

    let disabled = query.disable.unwrap_or(false);

    let mut proxies = vec![];
    let mut rules = vec![];
    let mut rule_providers = Mapping::new();

    let mut all: BTreeMap<String, Value> = BTreeMap::new();

    // prepend local config
    if !disabled {
        for proxy in manifest.proxies.clone().into_iter() {
            proxies.push(Value::Mapping(proxy.to_map()));
        }
        for rule in manifest.rules.clone().into_iter() {
            rules.push(Value::String(rule));
        }
        for (k, v) in manifest.rule_providers.clone().into_iter() {
            rule_providers.insert(Value::String(k), Value::Mapping(v.to_map()));
        }
    }

    // load remote config
    let config = find_config(&name);
    let client = Client::default();
    let mut resp = client.get(config.url.as_str()).send().await.unwrap();

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
    let mut remote_rule_providers = Mapping::new();
    if resp.contains_key("rule-providers") {
        remote_rule_providers = resp["rule-providers"].as_mapping().unwrap().to_owned();
    }
    for (k, v) in remote_rule_providers.clone().into_iter() {
        rule_providers.insert(k, v);
    }

    all.insert("proxies".to_string(), Value::Sequence(proxies));
    all.insert("rules".to_string(), Value::Sequence(rules));
    all.insert("rule-providers".to_string(), Value::Mapping(rule_providers));

    let yaml = serde_yaml::to_string(&all).unwrap();
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(yaml)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let contents = fs::read_to_string("./config.yaml").expect("Something went wrong reading the file");

    let manifest: models::Manifest = serde_yaml::from_str(contents.as_str()).unwrap_or_default();

    let addr = format!("0.0.0.0:{}", manifest.port);
    println!("start server at {}", addr);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(manifest.clone()))
            .service(get_config)
            .service(post_config)
    })
    .bind(addr)?
    .run()
    .await
}
