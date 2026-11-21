//! SINISA (ex-SNIS) — dados de saneamento por município.

pub mod agua;
mod api;
pub mod esgoto;
pub mod pluviais;
pub mod residuos;

pub use agua::FonteSnisAgua;
pub use esgoto::FonteSnisEsgoto;
pub use pluviais::FonteSnisPluviais;
pub use residuos::FonteSnisResiduos;
