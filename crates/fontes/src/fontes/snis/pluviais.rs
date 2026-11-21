use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

use super::api;

/// SINISA — águas pluviais por município (Base Municipal).
pub struct FonteSnisPluviais;

impl FonteDeDados for FonteSnisPluviais {
    fn nome(&self) -> &str {
        "snis-pluviais"
    }

    fn descricao(&self) -> &str {
        "SINISA / Ministério das Cidades — águas pluviais por município"
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
                "pluviais_2024".into(),
                api::FONTE_RAW,
                "pluviais/ano=2024".into(),
                api::PLUVIAIS_2024.into(),
            ))
            .com_etiquetas([("ano", "2024")]),
        ]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cria_uma_tarefa() {
        let tarefas = FonteSnisPluviais.criar_tarefas();
        assert_eq!(tarefas.len(), 1);
        assert_eq!(tarefas[0].etiquetas["ano"], "2024");
    }
}
