use std::net::SocketAddr;
use futures::future::try_join;
use log::{error, info};
use tokio::io;
use tokio::io::AsyncWriteExt;

use tokio::net::{TcpListener, TcpStream};

pub struct Proxy {
    local_address: String,
    target_addr: SocketAddr,
    listener: Option<TcpListener>,
    remote_sockets: Vec<TcpStream>,
    status: ProxyStatus,
}

pub enum ProxyStatus {
    Connected,
    Disconnect,
}

impl Proxy {
    pub async fn new(source_ip: String, source_port: String, target_ip: String, target_port: String) -> Result<Proxy, String> {

        match  TcpListener::bind(format!("{}:{}", source_ip, source_port)).await {
            Ok(tcp_listener) => {
                Ok(Proxy {
                    local_address: format!("{}:{}", source_ip, source_port),
                    target_addr: format!("{}:{}", target_ip, target_port).parse().unwrap(),
                    listener: Some(tcp_listener),
                    remote_sockets: vec![],
                    status: ProxyStatus::Disconnect,
                })
            }
            Err(err) => {Err("".parse().unwrap())}
        }

    }

    pub async fn run(&mut self) {
        loop {
            match self.listener.take() {
                None => {}
                Some(listener) => {
                    let (socket, _) = listener.accept().await.unwrap();
                    info!("receive connection with source: {}",self.local_address);
                    // tokio::spawn(self.transfer_tcp(socket));
                    self.transfer_tcp(socket).await.unwrap();
                }
            }

        }
    }

    async fn transfer_tcp(&mut self, source_socket: TcpStream) -> Result<(), String> {
        source_socket.set_nodelay(true).unwrap();
        match TcpStream::connect(self.target_addr).await {
            Ok(target_socket) => {
                info!("connected target ip: {}", self.target_addr);
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
            Err(e) => { error!("connected target: {} error{}", self.target_addr, e) }
        }
        Ok(())
    }
}