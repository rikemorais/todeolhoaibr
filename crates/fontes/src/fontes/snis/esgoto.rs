use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

use super::api;

/// SINISA — esgotamento sanitário por município (Base Municipal).
pub struct FonteSnisEsgoto;

impl FonteDeDados for FonteSnisEsgoto {
    fn nome(&self) -> &str {
        "snis-esgoto"
    }

    fn descricao(&self) -> &str {
        "SINISA / Ministério das Cidades — esgotamento sanitário por município"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        vec![
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                "ae_2023".into(),
                api::FONTE_RAW,
                "esgoto/ano=2023".into(),
                api::AE_2023.into(),
            ))
            .com_etiquetas([("ano", "2023")]),
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                "esgoto_2024".into(),
                api::FONTE_RAW,
                "esgoto/ano=2024".into(),
                api::ESGOTO_2024.into(),
            ))
            .com_etiquetas([("ano", "2024")]),
        ]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cria_duas_tarefas() {
        let tarefas = FonteSnisEsgoto.criar_tarefas();
        assert_eq!(tarefas.len(), 2);
        assert!(tarefas.iter().all(|t| t.etiquetas.contains_key("ano")));
    }
}
