use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::buffer_util::Buffer;


pub struct UdpClient {


    udp_socket: Option<UdpSocket>,
    pub sender_channel: Option<Sender<(Buffer, SocketAddr)>>,
    local_address: SocketAddr,

    executor : Sender<(Buffer, SocketAddr)>
}

impl UdpClient { 
    
    pub fn new(local_address: SocketAddr, executor : Sender<(Buffer, SocketAddr)>) -> Self {
        return UdpClient {
            udp_socket: None, sender_channel : None, local_address, executor
        }
    }
    
    pub fn init(&mut self) {

        self.udp_socket = Some(UdpSocket::bind(self.local_address).expect("aoi"));
        // self.udp_socket.as_ref().unwrap().set_read_timeout(Some(std::time::Duration::from_secs(5)))
        //     .expect("Failed to set socket read timeout");

        let (sender, receiver) = mpsc::channel::<(Buffer, SocketAddr)>();
        self.sender_channel.insert(sender);
        let udp_socket = self.udp_socket.as_ref().unwrap().try_clone().expect("Failed to clone UDP socket");
        thread::spawn(move || {


            while let Ok((mut packet_to_send, socket_address)) = receiver.recv() {
                udp_socket.send_to(packet_to_send.get(), socket_address).expect("aoi");
            }
        });
    }

    pub fn send(&self, packet: Buffer, socket_address: SocketAddr) {
        self.sender_channel.as_ref().unwrap().send((packet, socket_address)).unwrap();
    }



    pub fn start_receiving(&self) {
        let udp_socket = self.udp_socket.as_ref().unwrap().try_clone().expect("Failed to clone UDP socket");
        let sender_channel_clone = self.executor.clone();

        thread::spawn(move || {
            let mut packet_buffer = [0u8; 7000];


            loop {
                let receive =  udp_socket.recv_from(&mut packet_buffer);
                match receive {
                    Ok((packet_bytes_read, socket_address)) => {
                        let mut buffer = Buffer::new();
                        buffer.put(&packet_buffer);
                        buffer.current_size = packet_bytes_read;
                        sender_channel_clone.send((buffer, socket_address)).expect("aoi");
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
        });
    }
}
