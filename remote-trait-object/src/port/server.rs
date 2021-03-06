// Copyright 2020 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::types::Handler;
use crate::packet::Packet;
use crate::queue::{PopError, Queue};
use crossbeam::channel::RecvTimeoutError::{Disconnected, Timeout};
use crossbeam::channel::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time;

pub struct Server {
    receiver_thread: Option<thread::JoinHandle<()>>,
    joined_event_receiver: Receiver<()>,
}

impl Server {
    pub fn new<H>(handler: Arc<H>, ipc_send: Sender<Packet>, ipc_recv: Receiver<Packet>) -> Self
    where
        H: Handler + Send + 'static, {
        let (joined_event_sender, joined_event_receiver) = channel::bounded(1);
        let receiver_thread = thread::Builder::new()
            .name("port server receiver".into())
            .spawn(move || {
                receiver(handler, ipc_send, ipc_recv);
                joined_event_sender.send(()).expect("Server will be dropped after thread is joined");
            })
            .unwrap();

        Server {
            receiver_thread: Some(receiver_thread),
            joined_event_receiver,
        }
    }

    pub fn shutdown(mut self) {
        match self.joined_event_receiver.recv_timeout(time::Duration::from_millis(500)) {
            Err(Timeout) => {
                panic!("There may be a deadlock or misuse of Server. Call Server::shutdown when ipc_recv is closed");
            }
            Err(Disconnected) => {
                panic!("Maybe receiver thread panics");
            }
            Ok(_) => {}
        }

        self.receiver_thread.take().unwrap().join().unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        assert!(self.receiver_thread.is_none(), "Please call shutdown")
    }
}

fn receiver<H>(handler: Arc<H>, ipc_send: Sender<Packet>, ipc_recv: Receiver<Packet>)
where
    H: Handler + 'static, {
    let received_packets = Arc::new(Queue::new(100));
    let joiners = create_handler_threads(handler, ipc_send, Arc::clone(&received_packets));

    while let Ok(request) = ipc_recv.recv() {
        received_packets.push(request).expect("Queue will close after this loop");
    }
    // ipc_recv is closed.

    received_packets.close();
    for joiner in joiners {
        joiner.join().unwrap();
    }
}

fn create_handler_threads<H>(
    handler: Arc<H>,
    ipc_send: Sender<Packet>,
    received_packets: Arc<Queue<Packet>>,
) -> Vec<thread::JoinHandle<()>>
where
    H: Handler + 'static, {
    let mut joins = Vec::new();

    fn handler_loop<H: Handler>(handler: Arc<H>, ipc_send: Sender<Packet>, received_packets: Arc<Queue<Packet>>) {
        loop {
            let request = match received_packets.pop(None) {
                Ok(packet) => packet,
                Err(PopError::Timeout) => unreachable!(),
                Err(PopError::QueueClosed) => break,
            };

            trace!("Packet received in Port Server {}", request);
            let response = handler.handle(request.view());
            trace!("Handler result in Port Server {:?}", response);
            let mut response_packet = Packet::new_response_from_request(request.view());
            response_packet.append_data(&response);
            if let Err(err) = ipc_send.send(response_packet) {
                trace!("Multiplexer is dropped while sending a packet {:?}", err.into_inner());
                break
            };
        }
    }

    // FIXME: get thread count from config
    for i in 0..4 {
        let packet_queue_ = Arc::clone(&received_packets);
        let ipc_send_ = ipc_send.clone();
        let handler_ = Arc::clone(&handler);

        let join_handle = thread::Builder::new()
            .name(format!("port server send {}", i))
            .spawn(move || handler_loop(handler_, ipc_send_, packet_queue_))
            .unwrap();
        joins.push(join_handle);
    }

    joins
}
