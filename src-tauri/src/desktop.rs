use super::*;
use crate::online::is_online;
use crate::sheets::credentials_exist;
use std::sync::Mutex;
use tauri::Manager;

include!("desktop_a.rs");
include!("desktop_b.rs");
