cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_identity::{WasiIdentity,  IdentityDefault};
        use wasi_keyvalue::{WasiKeyValue, KeyValueDefault};
        use wasi_messaging::{WasiMessaging, MessagingDefault};
        use wasi_otel::{WasiOtel,  OtelDefault};
        use wasi_config::{WasiConfig, ConfigDefault};
        
        warp::runtime!({
            main: true,
            hosts: {
                WasiConfig: ConfigDefault,
                WasiHttp: HttpDefault,
                WasiIdentity: IdentityDefault,
                WasiKeyValue: KeyValueDefault,
                WasiMessaging: MessagingDefault,
                WasiOtel: OtelDefault,
            }
        });
    } else {
        // HACK: prevent lint error for wasm32 target
        fn main() {}
    }
}
