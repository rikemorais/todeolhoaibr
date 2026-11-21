//! Tesouro Nacional / SICONFI: dados fiscais de estados e municípios.

mod api;
pub mod dca;
pub mod entes;
mod paginacao;
pub mod rgf;
pub mod rreo;

pub use dca::FonteSiconfiDca;
pub use entes::FonteSiconfiEntes;
pub use rgf::FonteSiconfiRgf;
pub use rreo::FonteSiconfiRreo;
