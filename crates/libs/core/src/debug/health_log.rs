// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------

// Report app log as SF app health for debugging. Do not use in prod

use std::io::Write;

use tracing_subscriber::layer::SubscriberExt;
use windows_core::HSTRING;

use crate::{
    error::FabricError,
    runtime::CodePackageActivationContext,
    types::{HealthInformation, SequenceNumber},
};

/// Write the log as health report every 15 seconds,
/// or the log has reached 1024 bytes.
pub struct LogAsHealth {
    // data buffer and current index
    buff: FixedBuff,
}

const BUFF_SIZE: usize = 256;

pub struct FixedBuff {
    buff: [u8; BUFF_SIZE],
    index: usize,
    flush: Box<dyn Fn(String, SequenceNumber) + Send>, // function to flush the whole buffer
    seqno: SequenceNumber,
}

impl FixedBuff {
    fn new(flush: Box<dyn Fn(String, SequenceNumber) + Send>) -> Self {
        Self {
            buff: [0; BUFF_SIZE],
            index: 0,
            flush,
            seqno: 0,
        }
    }

    // when it returns ok(0) it is full
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // println!("write");
        let mut remain = self.remain();
        match remain.write(buf) {
            Ok(n) => {
                if n == 0 {
                    self.flush();
                    // retry again after flush
                    self.write(buf)
                } else {
                    self.index += n;
                    if self.full() {
                        self.flush();
                    }
                    Ok(n)
                }
            }
            Err(e) => panic!("mem buf error {e}"),
        }
    }

    fn remain(&mut self) -> &mut [u8] {
        &mut self.buff[self.index..]
    }

    fn full(&self) -> bool {
        debug_assert!(self.index <= self.buff.len());
        self.index == self.buff.len()
    }

    fn reset(&mut self) {
        self.buff.fill(0);
        self.index = 0;
    }

    fn flush(&mut self) {
        (self.flush)(String::from_utf8_lossy(&self.buff).into_owned(), self.seqno);
        self.reset();
        self.seqno += 1;
    }
}

impl Write for LogAsHealth {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buff.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buff.flush();
        Ok(())
    }
}

impl LogAsHealth {
    pub fn new(actctx: CodePackageActivationContext) -> LogAsHealth {
        LogAsHealth {
            buff: FixedBuff::new(Self::report_flush_fn(actctx)),
        }
    }

    fn report_flush_fn(
        actctx: CodePackageActivationContext,
    ) -> Box<dyn Fn(String, SequenceNumber) + Send> {
        let f = move |data, sqno| {
            let report = HealthInformation {
                source_id: HSTRING::from("my_log_source"),
                property: HSTRING::from("my_log_property"),
                time_to_live_seconds: 300,
                state: crate::types::HealthState::Ok,
                description: HSTRING::from(data),
                sequence_number: sqno,
                remove_when_expired: true,
            };
            println!("send health for log");
            if let Err(e) = actctx.report_application_health(&report, None) {
                println!("Error send health: {}", FabricError::from(e))
            }
        };
        Box::new(f)
    }
}

pub fn tempf() {
    let log_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("mypath")
        .expect("cannot open log file");
    // TODO: support log rotation, and non blocking.
    let registry = tracing_subscriber::registry();

    registry.with(
        tracing_subscriber::fmt::layer()
            .with_writer(log_file)
            .with_ansi(false),
    );
}
