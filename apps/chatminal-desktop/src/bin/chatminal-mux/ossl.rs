use anyhow::{anyhow, Context, Error};
use async_ossl::AsyncSslStream;
use config::TlsDomainServer;
use engine_mux_server_impl::PKI;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslStream, SslVerifyMode};
use openssl::x509::X509;
use promise::spawn::spawn_into_main_thread;
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;

struct OpenSslNetListener {
    acceptor: Arc<SslAcceptor>,
    listener: TcpListener,
}

impl OpenSslNetListener {
    fn new(listener: TcpListener, acceptor: SslAcceptor) -> Self {
        Self {
            listener,
            acceptor: Arc::new(acceptor),
        }
    }

    fn verify_peer_cert<T>(stream: &SslStream<T>) -> anyhow::Result<()> {
        let cert = stream
            .ssl()
            .peer_certificate()
            .ok_or_else(|| anyhow!("no peer cert"))?;
        let subject = cert.subject_name();
        let cn = subject
            .entries_by_nid(openssl::nid::Nid::COMMONNAME)
            .next()
            .ok_or_else(|| anyhow!("cert has no CN"))?;
        let cn_str = cn.data().as_utf8()?.to_string();
        let wanted_unix_name = std::env::var("USER")?;

        if wanted_unix_name == cn_str || cn_str.starts_with(&format!("user:{wanted_unix_name}/")) {
            Ok(())
        } else {
            anyhow::bail!("CN `{}` did not match $USER `{}`", cn_str, wanted_unix_name);
        }
    }

    fn run(&mut self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    stream.set_nodelay(true).ok();
                    match self.acceptor.clone().accept(stream) {
                        Ok(stream) => {
                            if let Err(err) = Self::verify_peer_cert(&stream) {
                                log::error!("problem with peer cert: {}", err);
                                break;
                            }
                            spawn_into_main_thread(async move {
                                engine_mux_server_impl::dispatch::process(AsyncSslStream::new(
                                    stream,
                                ))
                                .await
                                .map_err(|err| {
                                    log::error!("process: {:?}", err);
                                    err
                                })
                            })
                            .detach();
                        }
                        Err(err) => log::error!("failed TlsAcceptor: {}", err),
                    }
                }
                Err(err) => {
                    log::error!("accept failed: {}", err);
                    return;
                }
            }
        }
    }
}

pub fn spawn_tls_listener(tls_server: &TlsDomainServer) -> Result<(), Error> {
    openssl::init();
    let mut acceptor = SslAcceptor::mozilla_modern(SslMethod::tls())?;

    let cert_file = tls_server
        .pem_cert
        .clone()
        .unwrap_or_else(|| PKI.server_pem());
    acceptor
        .set_certificate_file(&cert_file, SslFiletype::PEM)
        .with_context(|| format!("set_certificate_file to {}", cert_file.display()))?;

    if let Some(chain_file) = tls_server.pem_ca.as_ref() {
        acceptor
            .set_certificate_chain_file(chain_file)
            .with_context(|| format!("set_certificate_chain_file to {}", chain_file.display()))?;
    }

    let key_file = tls_server
        .pem_private_key
        .clone()
        .unwrap_or_else(|| PKI.server_pem());
    acceptor
        .set_private_key_file(&key_file, SslFiletype::PEM)
        .with_context(|| format!("set_private_key_file to {}", key_file.display()))?;

    fn load_cert(path: &Path) -> anyhow::Result<X509> {
        Ok(X509::from_pem(&std::fs::read(path)?)?)
    }

    for name in &tls_server.pem_root_certs {
        if name.is_dir() {
            for entry in std::fs::read_dir(name)? {
                if let Ok(cert) = load_cert(&entry?.path()) {
                    acceptor.cert_store_mut().add_cert(cert).ok();
                }
            }
        } else {
            acceptor.cert_store_mut().add_cert(load_cert(name)?)?;
        }
    }
    acceptor
        .cert_store_mut()
        .add_cert(load_cert(&PKI.ca_pem())?)?;
    acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);

    let mut listener = OpenSslNetListener::new(
        TcpListener::bind(&tls_server.bind_address)
            .with_context(|| format!("error binding to {}", tls_server.bind_address))?,
        acceptor.build(),
    );
    std::thread::spawn(move || listener.run());
    Ok(())
}
