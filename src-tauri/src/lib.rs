mod access;
mod auth;
mod control;
pub mod core;
mod db;
mod hive;
mod hive_plan;
mod log;
mod migrate;
mod office;
mod office_sync;
mod online;
mod ops;
mod passwords;
mod provision;
mod sheets;
mod submit;
pub mod testdata;
mod trial;
mod voucher_edit;
mod vouchers;

// NOTE: full body restored in follow-up commit if truncated
pub const APP_NAME: &str = "T Books";
