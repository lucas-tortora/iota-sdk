// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod message_handler;
pub use message_handler::*;
use neon::prelude::*;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

pub static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    // Message handler methods.
    cx.export_function("sendMessage", message_handler::send_message)?;
    cx.export_function("messageHandlerNew", message_handler::message_handler_new)?;
    cx.export_function("destroy", message_handler::destroy)?;

    cx.export_function("listen", message_handler::listen)?;
    cx.export_function("initLogger", message_handler::init_logger)?;

    cx.export_function(
        "migrateStrongholdSnapshotV2ToV3",
        message_handler::migrate_stronghold_snapshot_v2_to_v3,
    )?;

    Ok(())
}
