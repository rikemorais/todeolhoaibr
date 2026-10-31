//! Adaptadores que implementam as portas do núcleo com tecnologias concretas.

mod armazenamento_s3;

pub use armazenamento_s3::{ArmazenamentoS3, ConfiguracaoS3};
