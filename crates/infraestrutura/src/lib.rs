//! Adaptadores que implementam as portas do núcleo com tecnologias concretas.

#![warn(missing_docs)]

mod armazenamento_s3;
mod cliente_ftp;
mod cliente_http;
mod despacho;
mod historico_s3;
mod limite;

pub use armazenamento_s3::{ArmazenamentoS3, ConfiguracaoS3};
pub use cliente_ftp::ClienteFtp;
pub use cliente_http::{ClienteHttpComRetentativas, PoliticaDeRetentativas};
pub use despacho::ClienteDespacho;
pub use historico_s3::HistoricoS3;
