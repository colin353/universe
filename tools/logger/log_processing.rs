/*
 *
 * Two types of generic log processing functions: filters and
 * extractors.
 *
 * Filters eliminate records, and extractors convert the records
 * into strings which can be rendered on the frontend.
 */

#[macro_use]
extern crate lazy_static;

use log_types_proto_rust::*;
use logger_grpc_rust::*;
use std::collections::HashMap;

type FilterFn = fn(&HashMap<String, String>, &EventMessage) -> bool;
type ExtractorFn = fn(&HashMap<String, String>, &EventMessage) -> String;

pub fn string_to_log(name: &str) -> Log {
    match name {
        "LARGETABLE_READS" => Log::LARGETABLE_READS,
        _ => Log::UNKNOWN,
    }
}

lazy_static! {
    pub static ref FILTERS: HashMap<String, Vec<(&'static str, FilterFn)>> = {
        let mut h = HashMap::new();
        h
    };
    pub static ref EXTRACTORS: HashMap<String, Vec<(&'static str, ExtractorFn)>> = {
        let mut h = HashMap::new();
        let e: ExtractorFn = textExtractor::<LargetablePerfLog>;
        h.insert(format!("{:?}", Log::LARGETABLE_READS), vec![("text", e)]);
        h
    };
}

pub fn textExtractor<T: protobuf::Message>(
    _: &HashMap<String, String>,
    log: &EventMessage,
) -> String {
    let mut handled_msg = T::new();
    handled_msg.merge_from_bytes(log.get_msg()).unwrap();
    let msg = format!("{:?}", handled_msg);
    format!(
        "<tr><td class='timestamp' data-timestamp={}>{}</td><td>{:?}</td></tr>\n",
        log.get_event_id().get_timestamp(),
        log.get_event_id().get_timestamp(),
        msg
    )
}
