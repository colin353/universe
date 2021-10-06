use std::collections::HashMap;
use std::net::Ipv4Addr;

use dns::{DnsHeader, DnsPacket, DnsQuestion, DnsRecord, DnsServer};

struct ExampleDNS {
    records: HashMap<String, (Ipv4Addr, u16)>,
}

impl DnsServer for ExampleDNS {
    fn handle_query(&self, header: DnsHeader, question: DnsQuestion) -> std::io::Result<DnsPacket> {
        let mut response = DnsPacket::new();
        response.header.id = header.id;
        if let Some((ip, port)) = self.records.get(&question.name) {
            response.answers.push(DnsRecord::A {
                domain: question.name.clone(),
                addr: *ip,
                ttl: 32,
            });
        }
        response.questions.push(question);
        Ok(response)
    }
}

fn main() {
    let mut records = HashMap::new();
    records.insert(
        String::from("desktop.colinmerkel.xyz"),
        (Ipv4Addr::new(192, 168, 86, 34), 8080),
    );
    records.insert(
        String::from("12.task.colin.desktop.colinmerkel.xyz"),
        (Ipv4Addr::new(192, 168, 86, 34), 8084),
    );
    println!("serving DNS...");
    let e = ExampleDNS { records };
    e.serve(53).unwrap();
    println!("done!");
}
