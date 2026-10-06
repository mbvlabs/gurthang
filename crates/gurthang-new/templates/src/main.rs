#[cfg(not(target_os = "linux"))]
compile_error!("Gurthang applications support Linux only");

#[tokio::main]
async fn main() -> gurthang::Result<()> {
    gurthang::start::<{{ crate_name }}::App>().await
}
