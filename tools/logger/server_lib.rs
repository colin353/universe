use logger_grpc_rust::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone)]
pub struct LoggerServiceHandler {
    writers: Arc<Mutex<HashMap<Log, recordio::RecordIOWriterOwned<EventMessage>>>>,
    data: Arc<RwLock<HashMap<Log, RwLock<Vec<EventMessage>>>>>,
}
