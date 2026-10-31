use std::path::Path;

/// Espaço livre no disco que contém `caminho`, ou `None` se não for possível descobrir.
pub fn espaco_livre_em(caminho: &Path) -> Option<u64> {
    fs4::available_space(caminho).ok()
}
