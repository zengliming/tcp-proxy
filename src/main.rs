#![warn(rust_2018_idioms)]
#[macro_use]
extern crate serde_derive;

use std::net::{SocketAddr};

use futures::future::{join_all, try_join};
use log::{error, info};
use log4rs;
use tokio;
use tokio::fs;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let mut runners = vec![];
    let file_path = "config.toml";
    let content = fs::read_to_string(file_path).await.unwrap();
    let conf: Conf = toml::from_str(&*content).unwrap();
    for c in conf.proxy {
        runners.push(tokio::spawn(run(c.source_ip, c.source_port, c.target_ip, c.target_port)))
    }

    join_all(runners).await;
}

#[derive(Deserialize)]
#[derive(Debug)]
struct ProxyConfig {
    source_ip: String,
    source_port: String,
    target_ip: String,
    target_port: String,
}

#[derive(Deserialize)]
#[derive(Debug)]
struct Conf {
    proxy: Vec<ProxyConfig>,
}

async fn run(source_ip: String, source_port: String, target_ip: String, target_port: String) {
    let listener = TcpListener::bind(format!("{}:{}", source_ip, source_port)).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        info!("receive connection with source ip: {} source port: {}",source_ip, source_port);
        tokio::spawn(transfer_tcp(socket, format!("{}:{}", target_ip, target_port).parse().unwrap()));
        // transfer_tcp(socket, format!("{}:{}",target_ip, target_port).parse().unwrap()).await;
    }
}

async fn transfer_tcp(source_socket: TcpStream, remote_add: SocketAddr) -> Result<(), String> {
    source_socket.set_nodelay(true).unwrap();
    match TcpStream::connect(remote_add).await {
        Ok(target_socket) => {
            info!("connected target ip: {}, target port: {}", remote_add.ip(), remote_add.port());
            target_socket.set_nodelay(true).unwrap();
            let (mut source_input, mut source_output) = io::split(source_socket);
            let (mut target_input, mut target_output) = io::split(target_socket);

            let source_to_target = async {
                match io::copy(&mut source_input, &mut target_output).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("copy io error {}", e);
                    }
                };
                target_output.shutdown().await
            };
            let target_to_source = async {
                match io::copy(&mut target_input, &mut source_output).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("copy io error {}", e);
                    }
                };
                source_output.shutdown().await
            };
            match try_join(source_to_target, target_to_source).await {
                Ok(_) => {}
                Err(_) => {}
            };
        }
        Err(e) => { error!("connected target ip: {}, target port: {} error{}",remote_add.ip(), remote_add.port(), e) }
    }
    Ok(())
}