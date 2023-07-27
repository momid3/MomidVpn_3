extern crate pnet;

use std::cell::{Cell, RefCell};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::from_utf8;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::SendError;
use std::thread;
use pnet::datalink;
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket, Ethernet as eth, EtherType, EtherTypes};
use pnet::packet::ipv4::{checksum, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{FromPacket, Packet, tcp};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::{MutableTcpPacket, TcpPacket};
use pnet::packet::udp::{ipv4_checksum, MutableUdpPacket};
use crate::arp_packet;
use crate::buffer_util::Buffer;
use crate::udp_client::UdpClient;

pub fn start() {

    let port = 7073;
    let interface = &datalink::interfaces()[1];
    let recent_udp_socket_address = Arc::new(Mutex::new(None));
    // let recent_ref = recent_udp_socket_address.get_mut();
    let recent_clone = Arc::clone(&recent_udp_socket_address);

    let (sender_of_executor, receiver_of_executor) = mpsc::channel::<Buffer>();


    let (mut sender, mut receiver) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    let (udp_sender_executor, udp_receiver_executor) = mpsc::channel::<(Buffer, SocketAddr)>();
    let mut udp_client = UdpClient::new(SocketAddr::from(SocketAddrV4::new(Ipv4Addr::from([0, 0, 0, 0]), port)), udp_sender_executor);

    udp_client.init();
    udp_client.start_receiving();
    let udp_sender = udp_client.sender_channel.unwrap();

    let (source_mac_address, destination_mac_address) = arp_packet::start();
    if source_mac_address.is_none() || destination_mac_address.is_none() {
        panic!("source mac address or destination mac address is none")
    }



    let thread = thread::spawn(move || {
        let mut ethernet_buffer = [0u8; 3000];
        let mut ip_buffer = [0u8; 3000];
        'aloop: loop {
            match receiver_of_executor.recv() {
                Ok(mut packet) => {

                    let packet_received = packet.get();
                    let packet_size = packet_received.len();

                    ethernet_buffer[0..packet_size].copy_from_slice(packet.get());
                    ip_buffer[0..packet_size - MutableEthernetPacket::minimum_packet_size()].copy_from_slice(&ethernet_buffer[MutableEthernetPacket::minimum_packet_size()..packet_size]);

                    if let Some(ethernet_packet) = MutableEthernetPacket::new(&mut ethernet_buffer[0..packet_size]) {
                        if let Some(mut ip_packet) = MutableIpv4Packet::new(&mut ip_buffer[0..packet_size - MutableEthernetPacket::minimum_packet_size()]) {
                            if ip_packet.get_source() == Ipv4Addr::from([146, 70, 145, 152])  {
                                continue;
                            }
                            match ip_packet.get_next_level_protocol() {
                                IpNextHeaderProtocols::Udp => {
                                    let udp_bytes = &mut ip_packet.payload().to_owned();
                                    let mut udp_packet = MutableUdpPacket::new(udp_bytes).expect("cannot make udp packet");
                                    if udp_packet.get_destination() == 7073 || udp_packet.get_destination() == 22 {
                                        continue 'aloop;
                                    }
                                    ip_packet.set_destination(Ipv4Addr::from([192, 168, 1, 1]));
                                    ip_packet.set_checksum(checksum(&ip_packet.to_immutable()));

                                    udp_packet.set_checksum(ipv4_checksum(&udp_packet.to_immutable(), &ip_packet.get_source(), &ip_packet.get_destination()));

                                    println!("received a packet from internet");
                                    println!("its ip is : {}", ip_packet.get_source());
                                    println!("the packet length is : {}", ip_packet.packet().len());

                                    ip_packet.set_payload(udp_packet.packet());
                                    let ip_packet_buffer = Buffer::new_from(ip_packet.packet());
                                    let recent_value = recent_udp_socket_address.lock().unwrap();
                                    if (*recent_value).is_some() {
                                        match udp_sender.send((ip_packet_buffer, (*recent_value).unwrap())) {
                                            Ok(_) => {
                                                println!("sent udp")
                                            }
                                            Err(e) => {
                                                eprintln!("{}", e);
                                            }
                                        };
                                    } else {
                                        println!("recent is none");
                                    }
                                }
                                IpNextHeaderProtocols::Tcp => {
                                    let tcp_bytes = &mut ip_packet.payload().to_owned();
                                    let mut tcp_packet = MutableTcpPacket::new(tcp_bytes).expect("cannot make tcp packet");
                                    if tcp_packet.get_destination() == 7073 || tcp_packet.get_destination() == 22 {
                                        continue 'aloop;
                                    }
                                    ip_packet.set_destination(Ipv4Addr::from([192, 168, 1, 1]));
                                    ip_packet.set_checksum(checksum(&ip_packet.to_immutable()));

                                    tcp_packet.set_checksum(tcp::ipv4_checksum(&tcp_packet.to_immutable(), &ip_packet.get_source(), &ip_packet.get_destination()));

                                    println!("received a packet from internet");
                                    println!("its ip is : {}", ip_packet.get_source());
                                    println!("the packet length is : {}", ip_packet.packet().len());

                                    ip_packet.set_payload(tcp_packet.packet());
                                    let ip_packet_buffer = Buffer::new_from(ip_packet.packet());
                                    let recent_value = recent_udp_socket_address.lock().unwrap();
                                    if (*recent_value).is_some() {
                                        match udp_sender.send((ip_packet_buffer, (*recent_value).unwrap())) {
                                            Ok(_) => {
                                                println!("sent udp")
                                            }
                                            Err(e) => {
                                                eprintln!("{}", e);
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
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }
    });

    let thread = thread::spawn(move || {
        while let Ok((mut buffer, socket_address)) = udp_receiver_executor.recv() {
            println!("receive a packet from udp of size {} from {}", buffer.current_size, socket_address);
            let _ = recent_clone.lock().unwrap().insert(socket_address);
                // if let Some(ethernet_packet) = MutableEthernetPacket::new(buffer.get()) {
                if let Some(mut ip_packet) = MutableIpv4Packet::new(buffer.get()) {
                    println!("found ip in udp, its source and destination is : {} and {}", ip_packet.get_source(), ip_packet.get_destination());
                    ip_packet.set_source(Ipv4Addr::from([146, 70, 145, 152]));
                    ip_packet.set_checksum(checksum(&ip_packet.to_immutable()));
                    match ip_packet.get_next_level_protocol() {
                        IpNextHeaderProtocols::Udp => {
                            let udp_bytes = &mut ip_packet.payload().to_owned();
                            let mut udp_packet = MutableUdpPacket::new(udp_bytes).expect("cannot make udp packet");
                            udp_packet.set_checksum(ipv4_checksum(&udp_packet.to_immutable(), &ip_packet.get_source(), &ip_packet.get_destination()));
                            ip_packet.set_payload(udp_packet.packet());
                        }
                        IpNextHeaderProtocols::Tcp => {
                            let tcp_bytes = &mut ip_packet.payload().to_owned();
                            let mut tcp_packet = MutableTcpPacket::new(tcp_bytes).expect("cannot make tcp packet");
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
                        sender.send_to(crafted_ethernet_packet.packet(), None).expect("cannot send").expect("cannot send");
                    };
                }
            // }
        }
    });


    loop {
        match receiver.next() {
            Ok(packet) => {
                // let packet = EthernetPacket::new(packet).unwrap();
                let mut buffer = Buffer::new();
                buffer.put(packet);
                sender_of_executor.send(buffer).unwrap();
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}
