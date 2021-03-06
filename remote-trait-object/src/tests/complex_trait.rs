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

use super::TestPort;
use crate as remote_trait_object;
use crate::{ExportService, ImportService, Port, SArc, Service};
use remote_trait_object_macro as rto_macro;
use std::sync::{Arc, Mutex};

#[rto_macro::service]
trait A: Service {
    fn service_object_as_argument(&self, b: SArc<dyn B>);
    fn service_object_as_return(&self) -> SArc<dyn B>;
    fn recursive_service_object(&self) -> SArc<dyn A>;
    fn get_recursion_count(&self) -> u32;
}

#[rto_macro::service]
trait B: Service {
    fn inc(&self);
    fn get(&self) -> i32;
}

struct SimpleA {
    recursion_count: u32,
}

impl SimpleA {
    pub fn new() -> Self {
        Self {
            recursion_count: 0,
        }
    }

    pub fn with_recursion_count(recursion_count: u32) -> Self {
        Self {
            recursion_count,
        }
    }
}

impl A for SimpleA {
    fn service_object_as_argument(&self, b: SArc<dyn B>) {
        let b = b.unwrap();
        assert_eq!(0, b.get());
        b.inc();
        b.inc();
        b.inc();
        assert_eq!(3, b.get());
    }

    fn service_object_as_return(&self) -> SArc<dyn B> {
        let b = Arc::new(SimpleB::new());
        SArc::new(b)
    }

    fn recursive_service_object(&self) -> SArc<dyn A> {
        let a = Arc::new(SimpleA::with_recursion_count(self.recursion_count + 1));
        SArc::new(a)
    }

    fn get_recursion_count(&self) -> u32 {
        self.recursion_count
    }
}

impl Service for SimpleA {}

struct SimpleB {
    count: Mutex<i32>,
}

impl SimpleB {
    pub fn new() -> Self {
        Self {
            count: Mutex::new(0),
        }
    }
}

impl Service for SimpleB {}
impl B for SimpleB {
    fn inc(&self) {
        *self.count.lock().unwrap() += 1
    }
    fn get(&self) -> i32 {
        *self.count.lock().unwrap()
    }
}

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn create_remote_a(port: Arc<dyn Port>) -> Arc<dyn A> {
    let a: Arc<dyn A> = Arc::new(SimpleA::new());
    let handle = <dyn A as ExportService<dyn A>>::export(Arc::downgrade(&port), a);
    <dyn A as ImportService<dyn A>>::import(Arc::downgrade(&port), handle)
}

#[test]
fn service_object_as_return() {
    init_logger();

    let port = Arc::new(TestPort::new());
    let remote_a = create_remote_a(port.clone());

    let remote_b = remote_a.service_object_as_return().unwrap();
    assert_eq!(remote_b.get(), 0);
    remote_b.inc();
    assert_eq!(remote_b.get(), 1);
    remote_b.inc();
    assert_eq!(remote_b.get(), 2);

    drop(remote_a);
    drop(remote_b);
    drop(port)
}

#[test]
fn service_object_as_argument() {
    init_logger();

    let port = Arc::new(TestPort::new());
    let remote_a = create_remote_a(port.clone());

    let service_object_b = Arc::new(SimpleB::new());
    remote_a.service_object_as_argument(SArc::new(service_object_b));

    drop(remote_a);
    drop(port)
}

#[test]
fn recursive_service_object() {
    init_logger();

    let port = Arc::new(TestPort::new());
    let mut remote_a = create_remote_a(port.clone());
    let mut remote_as = Vec::new();
    remote_as.push(Arc::clone(&remote_a));

    for i in 0..10 {
        assert_eq!(remote_a.get_recursion_count(), i);
        remote_a = remote_a.recursive_service_object().unwrap();
        remote_as.push(Arc::clone(&remote_a));
    }
    assert_eq!(remote_a.get_recursion_count(), 10);

    let remote_b = remote_a.service_object_as_return().unwrap();
    remote_b.inc();
    assert_eq!(remote_b.get(), 1);

    // remote_a + remote_b + recursive 10 remote_a = 12
    assert_eq!(port.register_len(), 12);

    drop(remote_as);
    drop(remote_a);
    drop(remote_b);
    drop(port)
}
