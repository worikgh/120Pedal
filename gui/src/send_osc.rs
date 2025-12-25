//! Send data to mixer about volume of effects
use rosc::{OscMessage, OscPacket, OscType, encoder};
use std::error::Error;
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
#[derive(Debug)]
pub struct OscSender {
    sock: UdpSocket,
    to_addr: SocketAddrV4,
}

impl OscSender {
    pub fn new(host_addr: &str, to_addr: &str) -> Result<Self, Box<dyn Error>> {
        let host_addr = SocketAddrV4::from_str(host_addr)?;
        let to_addr = SocketAddrV4::from_str(to_addr)?;
        let sock = UdpSocket::bind(host_addr)?;
        Ok(Self { sock, to_addr })
    }

    pub fn send(&self, osc_path: &str, volume: f32) -> Result<(), Box<dyn Error>> {
        let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
            addr: osc_path.to_string(),
            args: vec![OscType::Float(volume)],
        }))?;
        self.sock.send_to(&msg_buf, self.to_addr)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Tests if the OSC message is correctly sent and received.
    fn send(osc_sender: &OscSender, osc_path: &str, value: f32) {
        if let Err(err) = osc_sender.send(osc_path, value) {
            eprintln!("DBG Errpr {err}");
        }
    }
    #[test]
    fn test_send_osc_volume() -> Result<(), Box<dyn Error>> {
        let test_port = 5020;
        let test_addr = format!("127.0.0.1:{}", test_port);
        let osc_sender = OscSender::new("127.0.0.1:5200", &test_addr)?;
        // Send test OSC message
        send(&osc_sender, "/v/1", 1.0);
        send(&osc_sender, "/v/2", 0.2002);
        send(&osc_sender, "/v/3", 0.3002);
        send(&osc_sender, "/v/4", 0.999);

        Ok(())
    }
    #[test]
    fn test_send_osc_master() -> Result<(), Box<dyn Error>> {
        let test_port = 5020;
        let test_addr = format!("127.0.0.1:{}", test_port);
        let osc_sender = OscSender::new("127.0.0.1:5200", &test_addr)?;
        send(&osc_sender, "/M", 0.0);
        Ok(())
    }
}
