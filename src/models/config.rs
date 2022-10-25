use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub token: String,
    pub sql_server_ip: String,
    pub sql_server_port: u16,
    pub sql_server_username: String,
    pub sql_server_password: String,
    pub cmd_prefix: String,
    pub lavalink_ip: String,
    pub lavalink_password: String,
    pub danbooru_login: String,
    pub danbooru_api_key: String
}