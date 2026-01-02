cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_identity::{WasiIdentity,  IdentityDefault};
        use wasi_keyvalue::{WasiKeyValue, KeyValueDefault};
        use wasi_messaging::{WasiMessaging, MessagingDefault};
        use wasi_otel::{WasiOtel,  OtelDefault};

        warp::runtime!({
            main: true, 
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiIdentity: IdentityDefault,
                WasiKeyValue: KeyValueDefault,
                WasiMessaging: MessagingDefault,
            },
        });
    } else {
        // HACK: prevent lint error for wasm32 target
        fn main() {}
    }
}
