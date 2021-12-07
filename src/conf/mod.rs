use tokio::fs;

#[derive(serde_derive::Deserialize)]
#[derive(Debug)]
pub struct ProxyConfig {
    pub source_ip: String,
    pub source_port: String,
    pub target_ip: String,
    pub target_port: String,
}

#[derive(serde_derive::Deserialize)]
#[derive(Debug)]
pub struct Conf {
    pub proxy: Vec<ProxyConfig>,
}

impl Conf {
    pub async fn parser() -> Result<Conf, String> {
        let file_path = "config.toml";
        let content = fs::read_to_string(file_path).await.unwrap();
        let conf: Conf = toml::from_str(&*content).unwrap();
        return Ok(conf);
    }
}