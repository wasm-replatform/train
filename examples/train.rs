cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use qwasr_wasi_http::{WasiHttp, HttpDefault};
        use qwasr_wasi_identity::{WasiIdentity,  IdentityDefault};
        use qwasr_wasi_keyvalue::{WasiKeyValue, KeyValueDefault};
        use qwasr_wasi_messaging::{WasiMessaging, MessagingDefault};
        use qwasr_wasi_otel::{WasiOtel,  OtelDefault};
        use qwasr_wasi_config::{WasiConfig, ConfigDefault};

        qwasr::runtime!({
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
