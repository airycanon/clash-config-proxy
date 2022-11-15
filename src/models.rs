use std::collections::BTreeMap;
use super::schema::configs;
use diesel::prelude::*;
use serde::{Serialize,Deserialize};
use serde_yaml::{Mapping, Number, Value};


#[derive(Clone,Queryable,Serialize,Deserialize)]
pub struct Config {
    #[serde(default)]
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Insertable)]
#[diesel(table_name = configs)]
pub struct NewConfig<'a> {
    pub name: &'a String,
    pub url: &'a String,
}



#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct Manifest {
    pub port: i32,
    pub token: String,
    #[serde(default)]
    pub proxies: Vec<Proxy>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(
        default,
        rename(deserialize = "rule-providers", serialize = "rule-providers")
    )]
    pub rule_providers: BTreeMap<String, RuleProvider>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct Proxy {
    name: String,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    protocol: String,
    username: String,
    password: String,
    server: String,
    port: i32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct RuleProvider {
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
        map.insert(
            Value::String("type".to_string()),
            Value::String(clone.protocol),
        );
        map.insert(
            Value::String("username".to_string()),
            Value::String(clone.username),
        );
        map.insert(
            Value::String("password".to_string()),
            Value::String(clone.password),
        );
        map.insert(
            Value::String("server".to_string()),
            Value::String(clone.server),
        );
        map.insert(
            Value::String("port".to_string()),
            Value::Number(Number::from(clone.port)),
        );

        map
    }
}

impl RuleProvider {
    pub fn to_map(&self) -> Mapping {
        let clone = self.clone();

        let mut map = Mapping::new();
        map.insert(
            Value::String("type".to_string()),
            Value::String(clone.protocol),
        );
        map.insert(
            Value::String("behavior".to_string()),
            Value::String(clone.behavior),
        );
        map.insert(Value::String("url".to_string()), Value::String(clone.url));
        map.insert(Value::String("path".to_string()), Value::String(clone.path));
        map.insert(
            Value::String("interval".to_string()),
            Value::Number(Number::from(clone.interval)),
        );

        map
    }
}
