#[cfg(not(target_os = "linux"))]
compile_error!("Gurthang applications support Linux only");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    __GURTHANG_CRATE_NAME__::run().await
}
