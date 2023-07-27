use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub struct UdpClient {
    udp_socket: Option<UdpSocket>,
    sender_channel: Option<Sender<([u8; 700], usize, SocketAddr)>>,
    local_address: SocketAddr,
    executor: Sender<([u8; 700], usize, SocketAddr)>,
}

impl UdpClient {
    pub fn new(local_address: SocketAddr, executor: Sender<([u8; 700], usize, SocketAddr)>) -> Self {
        UdpClient {
            udp_socket: None,
            sender_channel: None,
            local_address,
            executor,
        }
    }

    pub fn init(&mut self) {
        self.udp_socket = Some(UdpSocket::bind(self.local_address).expect("Failed to bind UDP socket"));
        self.udp_socket
            .as_ref()
            .unwrap()
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .expect("Failed to set socket read timeout");

        let (sender, receiver) = mpsc::channel::<([u8; 700], usize, SocketAddr)>();
        self.sender_channel = Some(sender);

        let udp_socket = self.udp_socket.as_ref().unwrap().try_clone().expect("Failed to clone UDP socket");
        thread::spawn(move || {
            for (packet_to_send, data_size, socket_address) in receiver {
                udp_socket
                    .send_to(&packet_to_send[..data_size], socket_address)
                    .expect("Failed to send UDP packet");
            }
        });
    }

    pub fn start_receiving(&self) {
        let udp_socket = self.udp_socket.as_ref().expect("UDP socket not initialized").try_clone().expect("Failed to clone UDP socket");
        
        let sender_channel_clone = self.sender_channel.as_ref().unwrap().clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 65535];
            loop {
                match udp_socket.recv_from(&mut buffer) {
                    Ok((packet_bytes_read, _socket_address)) => {
                        let mut packet = [0u8; 700];
                        packet.copy_from_slice(&buffer[..packet_bytes_read]);
                        sender_channel_clone.send((packet, packet_bytes_read, _socket_address)).expect("aoi");
                        // Process the received packet as needed
                    }
                    Err(err) => {
                        eprintln!("Failed to receive UDP packet: {}", err);
                        break;
                    }
                }
            }
        });
    }
}
