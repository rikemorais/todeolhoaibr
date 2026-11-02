use std::path::Path;

/// Espaço livre no disco que contém `caminho`, ou `None` se não for possível descobrir.
pub fn espaco_livre_em(caminho: &Path) -> Option<u64> {
    fs4::available_space(caminho).ok()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn caminho_temporario(nome: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "todeolhoaibr-{nome}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn informa_espaco_livre_no_diretorio_existente() {
        let caminho = caminho_temporario("espaco-livre");
        std::fs::create_dir(&caminho).unwrap();

        let disponivel = espaco_livre_em(&caminho);

        std::fs::remove_dir_all(caminho).unwrap();
        assert!(disponivel.is_some());
    }

    #[test]
    fn devolve_none_quando_o_caminho_nao_existe() {
        let caminho = caminho_temporario("inexistente");
        assert_eq!(espaco_livre_em(&caminho), None);
    }
}
