#![warn(rust_2018_idioms)]
#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::net::SocketAddr;
use futures::future::{join_all, try_join};
use tokio;
use tokio::fs;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let mut runners = vec![];
    let mut proxys = vec![];
    let file_path = "config.toml";
    let content = fs::read_to_string(file_path).await.unwrap();
    let conf: Conf = toml::from_str(&*content).unwrap();
    let proxy = conf.proxy.unwrap();
    println!("{:?}", proxy);
    for c in proxy.as_slice() {
        proxys.push( Proxy {
            source_port: c.source_port.unwrap(),
            target_ip: c.target_ip.unwrap(),
            target_port: c.target_port.unwrap(),
            listener: Option::None,
        });

    }
    for proxy in proxys.iter() {
        runners.push(tokio::spawn(async move {
            proxy.run()
        }));
    }
    join_all(runners).await;
}

#[derive(Deserialize)]
#[derive(Debug)]
struct ProxyConfig {
    source_port: Option<String>,
    target_ip: Option<String>,
    target_port: Option<String>,
}

#[derive(Deserialize)]
#[derive(Debug)]
struct Conf {
    proxy: Option<Vec<ProxyConfig>>,
}

struct Proxy {
    source_port: String,
    target_ip: String,
    target_port: String,
    listener: Option<TcpListener>,
}

impl Proxy {
    pub async fn run(&self) {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.source_port)).await.unwrap();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            println!("receive connection");
            transfer_tcp(socket, format!("{}:{}",self.target_ip, self.target_port).parse().unwrap()).await;
        }
    }
}

async fn transfer_tcp(source_socket: TcpStream, remote_add: SocketAddr) -> Result<(), String> {
    source_socket.set_nodelay(true).unwrap();
    match TcpStream::connect(remote_add).await {
        Ok(target_socket) => {
            println!("connected target");
            target_socket.set_nodelay(true).unwrap();
            let (mut source_input, mut source_output) = io::split(source_socket);
            let (mut target_input, mut target_output) = io::split(target_socket);
            println!("copy io");
            let source_to_target = async {
                io::copy(&mut source_input, &mut target_output).await;
                target_output.shutdown().await
            };

            let target_to_source = async {
                io::copy(&mut target_input, &mut source_output).await;
                source_output.shutdown().await
            };
            try_join(source_to_target, target_to_source).await;
        }
        Err(e) => { println!("asdasdasd{}", e) }
    }
    Ok(())
}



