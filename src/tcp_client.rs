use std::io::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc::{self, Sender, Receiver};
use crate::buffer_util::Buffer;
use crate::hide_bytearray::CLIENT_DATA;

pub struct TcpServer {
    listener: TcpListener,
    sender_of_executor: Sender<Buffer>,
    sender_of_connection: Sender<OwnedWriteHalf>
}

impl TcpServer {
    pub async fn new(addr: &str) -> Result<(Self, Receiver<OwnedWriteHalf>, Receiver<Buffer>), Error> {
        let listener = TcpListener::bind(addr).await?;
        let (sender_of_executor, receiver_of_executor) = mpsc::channel(3000); // Creates a new mpsc channel
        let (sender_of_connection, receiver_of_connection) = mpsc::channel(3000); // Creates a new mpsc channel
        Ok((Self { listener, sender_of_executor, sender_of_connection }, receiver_of_connection, receiver_of_executor))
    }

    pub async fn init(self) {
        println!("Server listening on {}", self.listener.local_addr().unwrap());

        while let Ok((stream, _)) = self.listener.accept().await {
            let mut sender_of_executor = self.sender_of_executor.clone();
            // Spawn a new async task to handle each client connection.
            let (reader, writer) = stream.into_split();
            if let Err(e) = self.sender_of_connection.send(writer).await {
                eprintln!("{}", e);
            };
            tokio::spawn(async move { Self::handle_client(reader, &mut sender_of_executor).await });
            // tokio::spawn(async move { Self::write_to_client(writer, receiver_of_sender).await });
        }
    }

    async fn handle_client(mut reader: OwnedReadHalf, sender_of_executor: &mut Sender<Buffer>) {
        let mut buffer = [0; 7000];
        let mut size_buffer = [0; 2];
        let mut hide_buffer = [0u8; CLIENT_DATA.len()];
        let mut buffered = BufReader::new(&mut reader);

        match buffered.read_exact(&mut hide_buffer).await {
            Ok(size_read) => {
                if size_read > 0 {

                } else {
                    println!("size of hidden is 0");
                }
            }
            Err(e) => {
                println!("break of hidden : {:?}", e);
            }
        }

        loop {
            match buffered.read_exact(&mut size_buffer).await {
                Ok(size_read) if size_read > 0 => {
                    let size = u16::from_be_bytes(size_buffer);
                    if size > buffer.len() as u16 {
                        println!("size is too much");
                        continue;
                    }
                    match buffered.read_exact(&mut buffer[0..(size as usize)]).await {
                        Ok(bytes_read) if bytes_read > 0 => {
                            // Sends the stream and the bytes read to the channel
                            if size > 7000 {
                                println!("it is more than 7000");
                                continue;
                            }
                            if let Err(e) = sender_of_executor.send((Buffer::new_from(&buffer[..bytes_read]))).await {
                                eprintln!("Error sending to channel: {:?}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            println!("break of data {:?}", e);
                            break;
                        }
                        _ => {
                            println!("other data");
                            break;
                        }
                    }
                }
                Err(e) => {
                    println!("break of size {:?}", e);
                    break;
                }
                _ => {
                    println!("other size");
                    break;
                }
            }
        }
    }

    // async fn write_to_client(mut writer: OwnedWriteHalf, mut receiver_of_sender: Receiver<Vec<u8>>) {
    //     loop {
    //         match receiver_of_sender.recv().await {
    //             Some(buffer) =>              {
    //                 // Sends the stream and the bytes read to the channel
    //                 if let Err(e) = writer.write_all(&buffer).await {
    //                     eprintln!("Error sending to channel: {:?}", e);
    //                     break;
    //                 }
    //             }
    //             _ => break,
    //         }
    //     }
    // }
}

// Later in your code, you can receive messages like this:
async fn receive_messages(mut receiver: Receiver<(TcpStream, Vec<u8>)>) {
    while let Some((stream, bytes)) = receiver.recv().await {
        println!("Received {:?} from {:?}", bytes, stream);
        // Process the received data...
    }
}