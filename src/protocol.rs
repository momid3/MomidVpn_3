use std::io::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc::{self, Sender, Receiver};
use crate::buffer_util::Buffer;
use crate::encryption::xor_decode;
use crate::header::skip;
use crate::hide_bytearray::{CLIENT_DATA, CLIENT_DOWNLOAD_DATA, CLIENT_UPLOAD_DATA, SERVER_DOWNLOAD_DATA};

pub struct ProtocolServer {
    listener: TcpListener,
    sender_of_upload: Sender<Buffer>,
    sender_of_connection: Sender<OwnedWriteHalf>
}

impl ProtocolServer {
    pub async fn new(addr: &str) -> Result<(Self, Receiver<OwnedWriteHalf>, Receiver<Buffer>), Error> {
        let listener = TcpListener::bind(addr).await?;
        let (sender_of_upload, receiver_of_upload) = mpsc::channel(3000); // Creates a new mpsc channel
        let (sender_of_connection, receiver_of_connection) = mpsc::channel(3000); // Creates a new mpsc channel
        return Ok((Self { listener, sender_of_upload, sender_of_connection }, receiver_of_connection, receiver_of_upload))
    }

    pub async fn init(self) {
        println!("Server listening on {}", self.listener.local_addr().unwrap());

        loop {
            if let Ok((stream, _)) = self.listener.accept().await {
                let mut sender_of_upload = self.sender_of_upload.clone();

                let (reader, writer) = stream.into_split();
                println!("upload connected");
                tokio::spawn(async move { Self::handle_upload(reader, &mut sender_of_upload).await });
            }

            if let Ok((stream, _)) = self.listener.accept().await {
                let sender_of_connection = self.sender_of_connection.clone();
                println!("download connected");
                let (reader, writer) = stream.into_split();
                tokio::spawn(async move { Self::handle_download(reader, writer, sender_of_connection).await});
            }
        }
    }

    async fn handle_upload(mut reader: OwnedReadHalf, sender_of_upload: &mut Sender<Buffer>) {
        let mut buffer = [0; 7000];
        let mut size_buffer = [0; 2];
        let mut hide_buffer = [0u8; CLIENT_UPLOAD_DATA.len()];
        let mut buffered = BufReader::new(&mut reader);

        // match buffered.read_exact(&mut hide_buffer).await {
        //     Ok(size_read) => {
        //         if size_read > 0 {
        //
        //         } else {
        //             println!("size of hidden is 0");
        //         }
        //     }
        //     Err(e) => {
        //         println!("break of hidden : {:?}", e);
        //     }
        // }

        if let Err(e) = skip(&mut buffered).await {
            println!("error when skipping for client upload : {:?}", e);
        };

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
                            let mut received_packet = Buffer::new_from(&buffer[..bytes_read]);
                            xor_decode(received_packet.get(), 7);
                            if let Err(e) = sender_of_upload.send(received_packet).await {
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

    async fn handle_download(mut reader: OwnedReadHalf, mut writer: OwnedWriteHalf, sender_of_connection: Sender<OwnedWriteHalf>) {
        let mut hide_buffer = [0u8; CLIENT_DOWNLOAD_DATA.len()];
        let mut buffered = BufReader::new(&mut reader);

        // match buffered.read_exact(&mut hide_buffer).await {
        //     Ok(size_read) => {
        //         if size_read > 0 {
        //
        //         } else {
        //             println!("size of hidden is 0");
        //         }
        //     }
        //     Err(e) => {
        //         println!("break of hidden : {:?}", e);
        //     }
        // }

        if let Err(e) = skip(&mut buffered).await {
            println!("error when skipping for client download : {:?}", e);
        };

        if let Err(e) = writer.write_all(SERVER_DOWNLOAD_DATA).await {
            println!("error writing server download hidden : {:?}", e);
        };

        if let Err(e) = sender_of_connection.send(writer).await {
            eprintln!("Error sending to channel: {:?}", e);
        }

        // this is fake and to keep this task running indefinitely without the connection getting closed and only finishes on connection getting closed
        // loop {
        //     match buffered.read(&mut hide_buffer).await {
        //         Ok(size_read) => {
        //             if size_read > 0 {
        //                 println!("should never reach here in fake download. read {} bytes", size_read);
        //             } else {
        //                 println!("should never reach here, size of it is 0");
        //             }
        //         }
        //         Err(e) => {
        //             println!("should never reach here, completely finished : {:?}", e);
        //         }
        //     }
        // }
    }
}
