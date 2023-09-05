use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashMap;

struct Client {
    identifier: String,
    name: String,
    stream: TcpStream
}

/**
Each chat user will be handled by a thread running this function
*/
fn handle_connection(mut stream: TcpStream, tx: Sender<String>) {
    // Read client information (port and ip)
    let port = stream.peer_addr().expect("Cannot read port").port();
    let ip = stream.peer_addr().expect("Cannot read ip").ip();

    // Buffer to read data to
    let mut buffer = [0;1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read ==0 {
                    println!("{}:{} disconnected", ip, port);
                    break;
                }
                // Send message to receiver handler
                let message = format!("{}:{}:-{}",ip,port, String::from_utf8_lossy(&buffer[0..bytes_read]));
                tx.send(message);
            }
            Err(err) => {
                eprintln!("Error reading from client");
            }
        }
    }

}

/**
    rx: is a receiver of messages from other thread (each thread handles 1 chat user)
    rx_clients: is a receiver with main thread for updating new clients
*/
fn handle_receiver(rx: Receiver<String>, rx_clients: Receiver<Client>) {

    let mut clients = HashMap::new();

    loop {
        // Check if there are new clients
        let client_result = rx_clients.try_recv();
        match client_result {
            Ok(client) => {
                clients.insert(client.identifier.clone(), client);
            },
            Err(err) => {}
        }

        // Check new messages
        let message_result = rx.try_recv();
        match message_result {
            Ok(message_data) => {

                println!("{}", message_data);
                // Process the message
                let parts: Vec<&str> = message_data.as_str().split(":-").collect();
                let sender = parts.first().unwrap().to_string();
                let message = parts.last().unwrap();


                // Get client
                let mut client = clients.get_mut(&sender).unwrap();


                // Process special commands
                let mut is_special_command = false;
                if message.starts_with("username: ") {
                    let mut name = message.split(": ").collect::<Vec<&str>>().last().unwrap().to_string();
                    name = name.trim().to_string();
                    client.name = name;
                    is_special_command = true;
                }

                // If there is new message and the message is not special command,
                // broad cast to all client
                if !is_special_command {
                    let new_message = format!("{}: {}", client.name, message);
                    for (identifier, client) in &mut clients {
                        if let Err(err) = client.stream.write_all(new_message.as_bytes()) {
                            eprintln!("Error writing to client");
                            break;
                        }
                    }
                }

            }
            Err(err) => {}
        }

    }

}


fn main() {
    // Create a Tcp server
    let server = TcpListener::bind("0.0.0.0:8080").expect("Cannot bind to port 8080");
    let mut client_count = 1;
    println!("Server listening on port 8080...");

    // Accept incoming connections
    let (tx, rx) = mpsc::channel();

    let (tx_clients, rx_clients) = mpsc::channel();
    // Create a separete thread to handle message
    thread::spawn(move || {
        handle_receiver(rx, rx_clients);
    });
    for stream in server.incoming() {
        match stream {
            Ok(stream) => {

                let tx_clone = tx.clone();
                let port = stream.peer_addr().expect("Cannot get port").port();
                let ip = stream.peer_addr().expect("Cannot get ip").ip();

                println!("New client {}:{} connected", ip, port);
                let client = Client {
                    identifier: format!("{}:{}", ip, port),
                    stream: stream.try_clone().expect(""),
                    name: format!("client-{}", client_count),
                };
                tx_clients.send(client);
                thread::spawn(move || {
                    handle_connection(stream, tx_clone);

                });
                client_count += 1;


            }
            Err(e) => {
                println!("Error handling stream");
            }
        }

    }

}































