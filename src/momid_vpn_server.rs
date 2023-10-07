extern crate pnet;

use std::cell::{Cell, RefCell};
use std::io::Error;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::from_utf8;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::atomic::Ordering;
use std::sync::mpsc::SendError;
use std::thread;
use std::time::Duration;
use pnet::datalink;
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket, Ethernet as eth, EtherType, EtherTypes};
use pnet::packet::ipv4::{checksum, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{FromPacket, MutablePacket, Packet, tcp};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::{MutableTcpPacket, TcpPacket};
use pnet::packet::udp::{ipv4_checksum, MutableUdpPacket};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::time::sleep;
use crate::arp_packet;
use crate::buffer_util::Buffer;
use crate::encryption::xor_encode;
use crate::tcp_client::TcpServer;
use crate::hide_bytearray;
use crate::hide_bytearray::{DATA, Hide, IS_NEW_RECEIVE, IS_NEW_SEND};
use crate::protocol::ProtocolServer;
use crate::udp_client::UdpClient;

pub async fn start() -> Result<(), Error> {

    let user_ports = Arc::new(tokio::sync::Mutex::new(vec![false; 65536]));
    let user_ports_clone = Arc::clone(&user_ports);

    let port = 80;
    let interface = &datalink::interfaces()[1];

    let (sender_of_executor, mut receiver_of_executor) = tokio::sync::mpsc::channel::<Buffer>(3000);


    let (mut sender, mut receiver) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    let (tcp_server, mut receiver_of_connection, mut receiver_of_udp_executor) = ProtocolServer::new(format!("0.0.0.0:{}", port).as_str()).await?;
    tokio::spawn(async move {
        tcp_server.init().await
    });
    let mut current_tcp_client: Arc<tokio::sync::Mutex<Option<OwnedWriteHalf>>> = Arc::new(tokio::sync::Mutex::new(None));
    let current_tcp_client_clone = Arc::clone(&current_tcp_client);

    let (source_mac_address, destination_mac_address) = arp_packet::start();
    if source_mac_address.is_none() || destination_mac_address.is_none() {
        panic!("source mac address or destination mac address is none")
    }

    let mut final_packet_buffer = Buffer::new();

    let mut hider_buffer = Buffer::new_from(&[0u8; 3700]);



    let thread = tokio::spawn(async move {
        'aloop: loop {
            match receiver_of_executor.recv().await {
                Some(mut packet) => {
                    if let Some(ethernet_packet) = MutableEthernetPacket::new(packet.get()) {
                        if let Some(mut ip_packet) = MutableIpv4Packet::new(&mut ethernet_packet.payload().to_owned()) {
                            if ip_packet.get_source() == Ipv4Addr::from([146, 70, 145, 152])  {
                                continue;
                            }
                            match ip_packet.get_next_level_protocol() {
                                IpNextHeaderProtocols::Udp => {
                                    let udp_bytes = &mut ip_packet.payload().to_owned();
                                    let mut udp_packet = MutableUdpPacket::new(udp_bytes).expect("cannot make udp packet");
                                    if udp_packet.get_destination() == port || udp_packet.get_destination() == 22 {
                                        continue 'aloop;
                                    }

                                    let packet_port: u16 = udp_packet.get_destination();
                                    let mut user_ports_lock = user_ports.lock().await;
                                    let user_ports_ref: &mut Vec<bool> = user_ports_lock.as_mut();
                                    if user_ports_ref[packet_port as usize] == false {
                                        println!("not for user");
                                        continue 'aloop;
                                    }

                                    ip_packet.set_destination(Ipv4Addr::from([192, 168, 3, 7]));
                                    ip_packet.set_checksum(checksum(&ip_packet.to_immutable()));

                                    udp_packet.set_checksum(ipv4_checksum(&udp_packet.to_immutable(), &ip_packet.get_source(), &ip_packet.get_destination()));

                                    println!("received a packet from internet");
                                    println!("its ip is : {}", ip_packet.get_source());
                                    println!("the packet length is : {}", ip_packet.packet().len());

                                    ip_packet.set_payload(udp_packet.packet());
                                    // let ip_packet_buffer = Buffer::new_from(ip_packet.packet());
                                    let mut recent_value = current_tcp_client_clone.lock().await;
                                    if ip_packet.packet().len() > 3000 {
                                        println!("internet is more than allowed");
                                        continue 'aloop
                                    }
                                    
                                    if let Some(tcp_stream) = recent_value.as_mut() {
                                        let ip_packet_buffer = ip_packet.packet_mut();
                                        xor_encode(ip_packet_buffer, 7);
                                        let ip_packet_size: u16 = ip_packet_buffer.len() as u16;
                                        final_packet_buffer.put(&ip_packet_size.to_be_bytes());
                                        final_packet_buffer.append(ip_packet_buffer);
                                        let final_packet = final_packet_buffer.get();
                                        // let hidden_packet = Ordering::Relaxed) { IS_NEW_SEND.store(false, Ordering::Relaxed); final_packet.hide(&mut hider_buffer) } else { final_packet };
                                        match tcp_stream.write_all(final_packet).await {
                                            Ok(_) => {
                                                println!("sent udp")
                                            },
                                            Err(e) => {
                                                println!("error sending udp {:?}", e);
                                            }
                                        };
                                    } else {
                                        println!("recent is none");
                                    }
                                }
                                IpNextHeaderProtocols::Tcp => {
                                    let tcp_bytes = &mut ip_packet.payload().to_owned();
                                    let mut tcp_packet = MutableTcpPacket::new(tcp_bytes).expect("cannot make tcp packet");
                                    if tcp_packet.get_destination() == port || tcp_packet.get_destination() == 22 {
                                        continue 'aloop;
                                    }

                                    let packet_port: u16 = tcp_packet.get_destination();
                                    let mut user_ports_lock = user_ports.lock().await;
                                    let user_ports_ref: &mut Vec<bool> = user_ports_lock.as_mut();
                                    if user_ports_ref[packet_port as usize] == false {
                                        println!("not for user");
                                        continue 'aloop;
                                    }

                                    ip_packet.set_destination(Ipv4Addr::from([192, 168, 3, 7]));
                                    ip_packet.set_checksum(checksum(&ip_packet.to_immutable()));

                                    tcp_packet.set_checksum(tcp::ipv4_checksum(&tcp_packet.to_immutable(), &ip_packet.get_source(), &ip_packet.get_destination()));

                                    println!("received a packet from internet");
                                    println!("its ip is : {}", ip_packet.get_source());
                                    println!("the packet length is : {}", ip_packet.packet().len());

                                    ip_packet.set_payload(tcp_packet.packet());
                                    // let ip_packet_buffer = Buffer::new_from(ip_packet.packet());
                                    let mut recent_value = current_tcp_client_clone.lock().await;
                                    if ip_packet.packet().len() > 3000 {
                                        println!("internet is more than allowed");
                                        continue 'aloop
                                    }
                                    
                                    if let Some(tcp_stream) = recent_value.as_mut() {
                                        let ip_packet_buffer = ip_packet.packet_mut();
                                        xor_encode(ip_packet_buffer, 7);
                                        let ip_packet_size: u16 = ip_packet_buffer.len() as u16;
                                        final_packet_buffer.put(&ip_packet_size.to_be_bytes());
                                        final_packet_buffer.append(ip_packet_buffer);
                                        let final_packet = final_packet_buffer.get();
                                        // let hidden_packet = Ordering::Relaxed) { IS_NEW_SEND.store(false, Ordering::Relaxed); final_packet.hide(&mut hider_buffer) } else { final_packet };
                                        match tcp_stream.write_all(final_packet).await {
                                            Ok(_) => {
                                                println!("sent tcp")
                                            },
                                            Err(e) => {
                                                println!("error sending udp {:?}", e);
                                            }
                                        };
                                    } else {
                                        println!("recent is none");
                                    }
                                }
                                other => {
                                    // println!("no next level protocol, it is {} ip is : {}",other, ip_packet.get_source());
                                }
                            }
                        }
                    }
                }
                _ => {
                    // eprintln!("{}", e);
                }
            }
        }
    });

    let thread = tokio::spawn(async move {
        while let Some(mut buffer) = receiver_of_udp_executor.recv().await {
            let actual = buffer.get();
            // let mut actual_data = if IS_NEW_RECEIVE.load(Ordering::Relaxed) { IS_NEW_RECEIVE.store(false, Ordering::Relaxed); actual.un_hide() } else { actual };
            let mut actual_data = actual;
            println!("receive a packet from udp of size {}", actual_data.len());
                if let Some(mut ip_packet) = MutableIpv4Packet::new(&mut actual_data) {
                    println!("found ip in udp, its source and destination is : {} and {}", ip_packet.get_source(), ip_packet.get_destination());
                    ip_packet.set_source(Ipv4Addr::from([146, 70, 145, 152]));
                    ip_packet.set_checksum(checksum(&ip_packet.to_immutable()));
                    match ip_packet.get_next_level_protocol() {
                        IpNextHeaderProtocols::Udp => {
                            let udp_bytes = &mut ip_packet.payload().to_owned();
                            let mut udp_packet = MutableUdpPacket::new(udp_bytes).expect("cannot make udp packet");

                            let user_port: u16 = udp_packet.get_source();
                            let mut user_ports_lock = user_ports_clone.lock().await;
                            let user_ports_ref: &mut Vec<bool> = user_ports_lock.as_mut();
                            user_ports_ref[user_port as usize] = true;
                            // if user_ports.iter().filter(|port| **port).count() > 300 {
                            //     clear_ports(&mut user_ports);
                            // }

                            udp_packet.set_checksum(ipv4_checksum(&udp_packet.to_immutable(), &ip_packet.get_source(), &ip_packet.get_destination()));
                            ip_packet.set_payload(udp_packet.packet());
                        }
                        IpNextHeaderProtocols::Tcp => {
                            let tcp_bytes = &mut ip_packet.payload().to_owned();
                            let mut tcp_packet = MutableTcpPacket::new(tcp_bytes).expect("cannot make tcp packet");

                            let user_port: u16 = tcp_packet.get_source();
                            let mut user_ports_lock = user_ports_clone.lock().await;
                            let user_ports_ref: &mut Vec<bool> = user_ports_lock.as_mut();
                            user_ports_ref[user_port as usize] = true;
                            // if user_ports.iter().filter(|port| **port).count() > 300 {
                            //     clear_ports(&mut user_ports);
                            // }

                            tcp_packet.set_checksum(tcp::ipv4_checksum(&tcp_packet.to_immutable(), &ip_packet.get_source(), &ip_packet.get_destination()));
                            ip_packet.set_payload(tcp_packet.packet());
                        }
                        _ => {
                            println!("received udp contains nothing");
                        }
                    }
                    let mut buf = [0u8; 7000];

                    if let Some(mut crafted_ethernet_packet) =
                        MutableEthernetPacket::new(
                            &mut buf[
                                0..MutableEthernetPacket::minimum_packet_size()
                                    + ip_packet.packet().len()
                                ]
                        ) {

                        crafted_ethernet_packet.set_source(source_mac_address.unwrap());
                        crafted_ethernet_packet.set_destination(destination_mac_address.unwrap());
                        crafted_ethernet_packet.set_ethertype(EtherTypes::Ipv4);
                        crafted_ethernet_packet.set_payload(ip_packet.packet());

                        // println!("received packet : \n {:#?}", ethernet_packet);
                        if let Some(Err(e)) = sender.send_to(crafted_ethernet_packet.packet(), None) {
                            println!("cannot send : {}", e)
                        }
                    };
                }
            // }
        }
    });

    let thread = tokio::spawn(async move {
        while let Some(writer) = receiver_of_connection.recv().await {
            IS_NEW_SEND.store(true, Ordering::Relaxed);
            IS_NEW_RECEIVE.store(true, Ordering::Relaxed);
            // sleep(Duration::from_millis(300)).await;
            let _ = current_tcp_client.lock().await.insert(writer);
            // send_hidden(&mut current_tcp_client).await;
        }
    });


    loop {
        match receiver.next() {
            Ok(packet) => {
                // let packet = EthernetPacket::new(packet).unwrap();
                let mut buffer = Buffer::new();
                buffer.put(packet);
                if let Err(e) = sender_of_executor.send(buffer).await {

                };
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }


}

async fn send_hidden(tcp_writer_mutex: &mut Arc<tokio::sync::Mutex<Option<OwnedWriteHalf>>>) {
    send_packet_to_client_tcp(DATA, tcp_writer_mutex).await;
}

async fn send_packet_to_client_tcp(packet: &[u8], tcp_writer_mutex: &mut Arc<tokio::sync::Mutex<Option<OwnedWriteHalf>>>) {
    let mut recent_value = tcp_writer_mutex.lock().await;
    if let Some(tcp_stream) = recent_value.as_mut() {
        match tcp_stream.write_all(packet).await {
            Ok(_) => {
                println!("sent udp")
            },
            Err(e) => {
                println!("error sending udp {:?}", e);
            }
        };
    } else {
        println!("recent is none");
    }
}

fn clear_ports(user_ports: &mut [bool]) {
    for port in user_ports {
        *port = false;
    }
}
