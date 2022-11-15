pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenvy::dotenv;
use schema::configs;
use std::env;
use schema::configs::dsl;

use models::NewConfig;
use models::Config;


pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn create_config(config: &models::Config) -> usize {
    let conn = &mut establish_connection();

    let new_post = NewConfig { name: &config.name, url: &config.url};

    diesel::insert_into(configs::table)
        .values(&new_post)
        .execute(conn)
        .expect("Error saving new config")
}

pub fn find_config(n: &String) -> Config {
    let connection = &mut establish_connection();
     let list = dsl::configs.filter(dsl::name.eq(n))
        .load::<Config>(connection)
        .expect("Error loading configs");

    list[0].clone()
}

pub fn list_config() -> Vec<Config> {
    let connection = &mut establish_connection();
    dsl::configs
        .load::<Config>(connection)
        .expect("Error loading configs")
}