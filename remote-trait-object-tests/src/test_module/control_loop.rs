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

use cbasesandbox::execution::executee;
use cbasesandbox::ipc::intra::Intra;
use cbasesandbox::ipc::unix_socket::DomainSocket;
use cbasesandbox::ipc::Ipc;
use remote_trait_object::{Context, HandleToExchange};
use std::collections::HashMap;

pub fn module_control_loop<IPC: Ipc, Module: Bootstrap>(args: Vec<String>) {
    let ctx = executee::start::<IPC>(args);

    let args: Vec<u8> = recv(&ctx);
    let mut rto_contexts = HashMap::<String, remote_trait_object::Context>::new();
    let mut module = Module::new(args);

    loop {
        let message: String = recv(&ctx);
        debug!("Receved message {}", message);
        if message == "link" {
            let (counter_module_name, ipc_type, ipc_config) = recv(&ctx);
            debug!("Received link({}, {}, {:?})", counter_module_name, ipc_type, ipc_config);
            let rto_context = create_ipc_context(ipc_type, ipc_config);
            let old = rto_contexts.insert(counter_module_name, rto_context);
            // we assert before dropping old to avoid (hard-to-debug) blocking.
            assert!(old.is_none(), "You must unlink first to link an existing remote trait object context");
        } else if message == "handle_export" {
            let (counter_module_name, service_name): (String, String) = recv(&ctx);
            debug!("Received handle_export({}, {})", counter_module_name, service_name);
            let counter_rto_context =
                rto_contexts.get(&counter_module_name).expect("Please link the module before export");
            let handle_to_export = module.export(counter_rto_context, service_name);
            send(&ctx, &handle_to_export);
        } else if message == "handle_import" {
            let (counter_module_name, service_name, exchange): (String, String, HandleToExchange) = recv(&ctx);
            debug!("Received handle_import({}, {}, {:?})", counter_module_name, service_name, exchange);
            let counter_rto_context =
                rto_contexts.get(&counter_module_name).expect("Please link the module before export");
            module.import(counter_rto_context, service_name, exchange);
        } else if message == "start" {
            let result = module.start();
            send(&ctx, &result);
        } else if message == "quit" {
            break
        } else {
            panic!("Unexpected message: {}", message);
        }
        send(&ctx, &"done".to_string());
    }
    for rto_context in rto_contexts.values() {
        rto_context.disable_garbage_collection();
    }
    ctx.terminate();
}

fn recv<IPC: Ipc, T: serde::de::DeserializeOwned>(ctx: &executee::Context<IPC>) -> T {
    let bytes = ctx.ipc.as_ref().unwrap().recv(None).unwrap();
    serde_cbor::from_slice(&bytes).unwrap()
}

fn send<I: Ipc, T: serde::Serialize>(ctx: &executee::Context<I>, data: &T) {
    ctx.ipc.as_ref().unwrap().send(&serde_cbor::to_vec(data).unwrap());
}

fn create_ipc_context(ipc_type: String, ipc_config: Vec<u8>) -> remote_trait_object::Context {
    if ipc_type == "DomainSocket" {
        let ipc = DomainSocket::new(ipc_config);
        let (send, recv) = ipc.split();
        remote_trait_object::Context::new(send, recv)
    } else if ipc_type == "Intra" {
        let ipc = Intra::new(ipc_config);
        let (send, recv) = ipc.split();
        remote_trait_object::Context::new(send, recv)
    } else {
        panic!("Invalid ipc type")
    }
}

pub trait Bootstrap {
    fn new(args: Vec<u8>) -> Self;
    fn export(&mut self, context: &Context, service_name: String) -> HandleToExchange;
    fn import(&mut self, context: &Context, service_name: String, exchange: HandleToExchange);
    fn start(&self) -> String {
        "".to_string()
    }
}
