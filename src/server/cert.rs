use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use tokio::{fs, io};

pub async fn get_self_signed_cert() -> Result<(rustls::Certificate, rustls::PrivateKey)> {
  let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let cert_path = base_path.join("cert").join("cert.der");
  let key_path = base_path.join("cert").join("key.der");

  match fs::read(&cert_path).await {
    Ok(cert) => {
      println!("Reading self-signed certificate from local");

      let key = fs::read(&key_path)
        .await
        .context("Failed to read private key")?;

      Ok((rustls::Certificate(cert), rustls::PrivateKey(key)))
    }

    Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
      println!("Generating self-signed certificate");

      let gen_cert = rcgen::generate_simple_self_signed(vec!["qft-server".into()]).unwrap();

      let cert = gen_cert
        .serialize_der()
        .context("Failed to serialize certificate")?;
      let key = gen_cert.serialize_private_key_der();

      fs::create_dir_all(base_path)
        .await
        .context("Failed to create cert directory")?;
      fs::write(&cert_path, &cert)
        .await
        .context("Failed to write certificate")?;
      fs::write(&key_path, &key)
        .await
        .context("Failed to write private key")?;

      Ok((rustls::Certificate(cert), rustls::PrivateKey(key)))
    }

    Err(e) => Err(anyhow!(e).context("Failed to read certificate")),
  }
}
