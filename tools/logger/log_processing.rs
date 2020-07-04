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
use protobuf::Message;
use std::collections::HashMap;

type FilterFn = fn(&HashMap<String, String>, &EventMessage) -> bool;
type ExtractorFn = fn(&HashMap<String, String>, &EventMessage) -> (u64, String);

pub fn string_to_log(name: &str) -> Log {
    match name {
        "LARGETABLE_READS" => Log::LARGETABLE_READS,
        _ => Log::UNKNOWN,
    }
}

lazy_static! {
    pub static ref FILTERS: HashMap<String, Vec<(&'static str, FilterFn)>> = {
        let mut h = HashMap::new();
        let f: FilterFn = latencyFilter;
        let k: FilterFn = kindFilter;
        h.insert(
            format!("{:?}", Log::LARGETABLE_READS),
            vec![("latency", f), ("kind", k)],
        );
        h
    };
    pub static ref EXTRACTORS: HashMap<String, Vec<(&'static str, ExtractorFn)>> = {
        let mut h = HashMap::new();
        let l: ExtractorFn = latencyExtractor;
        let e: ExtractorFn = textExtractor::<LargetablePerfLog>;
        h.insert(
            format!("{:?}", Log::LARGETABLE_READS),
            vec![("text", e), ("latency", l)],
        );
        h
    };
}

pub fn kindFilter(s: &HashMap<String, String>, log: &EventMessage) -> bool {
    if let Some(kind) = s.get("kind") {
        let mut m = LargetablePerfLog::new();
        m.merge_from_bytes(log.get_msg()).unwrap();
        return kind == "read" && m.get_kind() == ReadKind::READ
            || kind == "read_range" && m.get_kind() == ReadKind::READ_RANGE;
    }

    return false;
}

pub fn latencyFilter(s: &HashMap<String, String>, log: &EventMessage) -> bool {
    let mut m = LargetablePerfLog::new();
    m.merge_from_bytes(log.get_msg()).unwrap();

    if let Some(min) = s.get("minLatency") {
        if let Ok(s) = min.parse::<u64>() {
            if m.get_request_duration_micros() < s {
                return false;
            }
        }
    }
    if let Some(max) = s.get("maxLatency") {
        if let Ok(s) = max.parse::<u64>() {
            if m.get_request_duration_micros() > s {
                return false;
            }
        }
    }

    true
}

pub fn latencyExtractor(_: &HashMap<String, String>, log: &EventMessage) -> (u64, String) {
    let mut extracted_msg = LargetablePerfLog::new();
    extracted_msg.merge_from_bytes(log.get_msg()).unwrap();
    (
        log.get_event_id().get_timestamp(),
        format!("{}", extracted_msg.get_request_duration_micros()),
    )
}

pub fn textExtractor<T: protobuf::Message>(
    _: &HashMap<String, String>,
    log: &EventMessage,
) -> (u64, String) {
    let mut handled_msg = T::new();
    handled_msg.merge_from_bytes(log.get_msg()).unwrap();
    let msg = format!("{:?}", handled_msg);
    (
        log.get_event_id().get_timestamp(),
        format!(
            "<tr><td class='timestamp' data-timestamp={}>{}</td><td>{:?}</td></tr>\n",
            log.get_event_id().get_timestamp(),
            log.get_event_id().get_timestamp(),
            msg
        ),
    )
}
