extern crate pnet;


use std::io::stdin;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::usize;
use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet as athernet;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, Arp, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, Ethernet, MutableEthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::{Packet, MutablePacket};
use pnet::util::MacAddr;

pub fn start() -> (Option<MacAddr>, Option<MacAddr>) {
    // Retrieve the default network interface

    let mut interface_index = 0;
    datalink::interfaces()
        .into_iter().for_each(|interface| {
        println!("interface {}, number {}", interface.name, interface_index);
        interface_index += 1;
    });

    let mut input_index = String::new();
    stdin().read_line(&mut input_index).expect("cant read input index");
    let interface = &datalink::interfaces()[1];

    // Create a new channel
    let (mut tx, mut rx) = match datalink::channel(interface, Default::default()) {
        Ok(athernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Failed to create channel: {}", e),
    };

    // // Create a new ARP packet
    // let arp_packet = Arp {
    //     hardware_type: ArpHardwareTypes::Ethernet,
    //     protocol_type: EtherTypes::Ipv4,
    //     hw_addr_len: 6, // MAC address length
    //     proto_addr_len: 4, // IPv4 address length
    //     operation: ArpOperations::Request,
    //     sender_hw_addr: interface.mac.unwrap(), // Sender MAC address
    //     sender_proto_addr: Ipv4Addr::from_str("146.70.145.152").unwrap(), // Sender IPv4 address
    //     target_hw_addr: MacAddr::zero(), // Target MAC address (we'll set this later)
    //     target_proto_addr: Ipv4Addr::from_str("146.70.145.129").unwrap(),
    //     payload: vec![],
    // };
    //
    // // Create a new Ethernet packet
    // let ethernet_packet = Ethernet {
    //     destination : MacAddr::broadcast(), // Destination MAC address (we'll set this later)
    //     source : interface.mac.unwrap(), // Source MAC address
    //     ethertype : EtherTypes::Arp, // Ethernet type
    //     payload : arp_packet.payload, // ARP packet
    // };


    let mut ethernet_buffer = [0u8; 42];
    let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

    ethernet_packet.set_destination(MacAddr::broadcast());
    ethernet_packet.set_source(interface.mac.unwrap());
    ethernet_packet.set_ethertype(EtherTypes::Arp);

    let mut arp_buffer = [0u8; 28];
    let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);
    arp_packet.set_sender_hw_addr(interface.mac.unwrap());
    arp_packet.set_sender_proto_addr(Ipv4Addr::from_str("146.70.145.152").unwrap());
    arp_packet.set_target_hw_addr(MacAddr::zero());
    arp_packet.set_target_proto_addr(Ipv4Addr::from_str("146.70.145.129").unwrap());

    ethernet_packet.set_payload(arp_packet.packet_mut());

    // sender
    //     .send_to(ethernet_packet.packet(), None)
    //     .unwrap()
    //     .unwrap();


    // // Convert the Ethernet packet to a mutable byte buffer
    // let mut ethernet_buffer = vec![0u8; ethernet_packet.payload.len()];
    // ethernet_packet.payload.clone_into(&mut ethernet_buffer);

    // Send the packet
    match tx.send_to(ethernet_packet.packet(), None) {
        Some(_) => println!("ARP packet sent successfully"),
        _ => {}
    }

    // Receive and process packets
    let mut reply_mac: Option<MacAddr> = None;
    let mut received_reply = false;

    while !received_reply {
        match rx.next() {
            Ok(packet) => {
                if let Some(ethernet) = EthernetPacket::new(packet) {
                    if ethernet.get_ethertype() == EtherTypes::Arp {
                        let arp = ArpPacket::new(ethernet.payload()).unwrap();
                        if arp.get_operation() == ArpOperations::Reply {
                            reply_mac = Some(arp.get_sender_hw_addr());
                            received_reply = true;
                        }
                    }
                }
            }
            Err(e) => println!("Failed to read packet: {}", e),
        }
    }

    // Process the received ARP reply
    if let Some(mac) = reply_mac {
        println!("Received ARP reply from MAC address: {}", mac);
        return (interface.mac, Some(mac));
    } else {
        return (None, None)
    }
}
