//! Send data to mixer about volume of effects
use rosc::encoder;
use rosc::{OscMessage, OscPacket};
use std::error::Error;
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;

#[allow(dead_code)]
fn send_osc() -> Result<(), Box<dyn Error>> {
    let host_addr = SocketAddrV4::from_str("127.0.0.1:5200").unwrap();
    let to_addr = SocketAddrV4::from_str("127.0.0.1:5020").unwrap();
    let sock = UdpSocket::bind(host_addr).unwrap();
    let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/v/1/0.7".to_string(),
        args: vec![],
    }))
    .unwrap();
    sock.send_to(&msg_buf, to_addr).unwrap();
    Ok(())
}
