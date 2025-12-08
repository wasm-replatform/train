#![feature(prelude_import)]
#[macro_use]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_identity::{WasiIdentity, WasiIdentityCtxImpl as IdentityDefault};
use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxImpl as KeyValueDefault};
use wasi_messaging::{WasiMessaging, WasiMessagingCtxImpl as MessagingDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
