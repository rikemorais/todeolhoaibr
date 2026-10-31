//! Adaptadores que implementam as portas do núcleo com tecnologias concretas.

mod armazenamento_s3;
mod cliente_http;

pub use armazenamento_s3::{ArmazenamentoS3, ConfiguracaoS3};
pub use cliente_http::{ClienteHttpComRetentativas, PoliticaDeRetentativas};
