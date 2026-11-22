//! Turismo — dados abertos do Ministério do Turismo.

use std::time::Duration;
use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "turismo-chegadas";
const DATASET: &str = "184e0ddd-7eaf-488d-ad84-0331219d6e99";
const HOST: &str = "dados.turismo.gov.br";

struct AnoChegadas {
    ano: u16,
    recurso_id: &'static str,
    arquivo: &'static str,
}

const ANOS: &[AnoChegadas] = &[
    AnoChegadas { ano: 1989, recurso_id: "e1a22b7d-fb15-4f66-bd83-9733968ae75f", arquivo: "chegadas_1989.csv" },
    AnoChegadas { ano: 1990, recurso_id: "23882d05-d82b-4537-a5c7-c2cdf93668c0", arquivo: "chegadas_1990.csv" },
    AnoChegadas { ano: 1991, recurso_id: "16d8af69-487e-4f22-a9b5-1b4bfa761e0d", arquivo: "chegadas_1991.csv" },
    AnoChegadas { ano: 1992, recurso_id: "73e509d4-daf5-4b82-836a-e329c0ec77c9", arquivo: "chegadas_1992.csv" },
    AnoChegadas { ano: 1993, recurso_id: "f4ee84f1-e4b4-4183-9357-5f3509f5685c", arquivo: "chegadas_1993.csv" },
    AnoChegadas { ano: 1994, recurso_id: "425b1cce-653b-419f-b4f8-10b4bd51d30c", arquivo: "chegadas_1994.csv" },
    AnoChegadas { ano: 1995, recurso_id: "59a90159-8b32-43e7-82a0-4a59f39732f4", arquivo: "chegadas_1995.csv" },
    AnoChegadas { ano: 1996, recurso_id: "46240329-9ac4-4322-9717-f86b06b78c5c", arquivo: "chegadas_1996.csv" },
    AnoChegadas { ano: 1997, recurso_id: "4c38df5d-968d-4197-b251-c7fdfdddfb09", arquivo: "chegadas_1997.csv" },
    AnoChegadas { ano: 1998, recurso_id: "33268202-2dfa-4604-a919-f419ffb8d448", arquivo: "chegadas_1998.csv" },
    AnoChegadas { ano: 1999, recurso_id: "a472fd00-638e-4ae9-9a7d-e5270be4a5ee", arquivo: "chegadas_1999.csv" },
    AnoChegadas { ano: 2000, recurso_id: "8a879066-8a14-4576-802f-bdc2b65ab9d7", arquivo: "chegadas_2000.csv" },
    AnoChegadas { ano: 2001, recurso_id: "7ff96ec4-80a9-4406-8ca1-61f52b1a94ca", arquivo: "chegadas_2001.csv" },
    AnoChegadas { ano: 2002, recurso_id: "a0236a28-54a5-4b29-8f29-ea6121ed037f", arquivo: "chegadas_2002.csv" },
    AnoChegadas { ano: 2003, recurso_id: "b963ba83-8d54-43a2-a5ca-979912743dfb", arquivo: "chegadas_2003.csv" },
    AnoChegadas { ano: 2004, recurso_id: "482362d3-5af4-4c1a-b74a-a5ef51906897", arquivo: "chegadas_2004.csv" },
    AnoChegadas { ano: 2005, recurso_id: "1935236f-1e1d-4ed9-bb5c-84ff2f955b58", arquivo: "chegadas_2005.csv" },
    AnoChegadas { ano: 2006, recurso_id: "11a1514c-b496-408c-9810-03bce31e1605", arquivo: "chegadas_2006.csv" },
    AnoChegadas { ano: 2007, recurso_id: "24577db6-bc12-4db1-9146-4ffa00e36b17", arquivo: "chegadas_2007.csv" },
    AnoChegadas { ano: 2008, recurso_id: "a4934a01-e12d-4a30-8a90-8f353b036195", arquivo: "chegadas_2008.csv" },
    AnoChegadas { ano: 2009, recurso_id: "942e84f4-93e7-406f-8314-e860cd1d80d3", arquivo: "chegadas_2009.csv" },
    AnoChegadas { ano: 2010, recurso_id: "44cee3ac-a507-49c1-9320-20ecdb7353c1", arquivo: "chegadas_2010.csv" },
    AnoChegadas { ano: 2011, recurso_id: "410f90a1-7287-4487-a8a3-142f8c37d87c", arquivo: "chegadas_2011.csv" },
    AnoChegadas { ano: 2012, recurso_id: "ec9c41a2-6b1e-46b3-a93f-f46e0adfc2d7", arquivo: "chegadas_2012.csv" },
    AnoChegadas { ano: 2013, recurso_id: "97dbba8c-2c8e-4055-baae-6361c3ba2e79", arquivo: "chegadas_2013.csv" },
    AnoChegadas { ano: 2014, recurso_id: "669597f4-217f-4762-ab47-80c17238ce64", arquivo: "chegadas_2014.csv" },
    AnoChegadas { ano: 2015, recurso_id: "e0e4ffa2-fab1-4a06-a5a5-3b8986a1a331", arquivo: "chegadas_2015.csv" },
    AnoChegadas { ano: 2016, recurso_id: "7a04c1d4-482d-4359-b04e-0907b6407c08", arquivo: "chegadas_2016.csv" },
    AnoChegadas { ano: 2017, recurso_id: "86134fdc-fbfe-44d0-ad2e-315b16f70233", arquivo: "chegadas_2017.csv" },
    AnoChegadas { ano: 2018, recurso_id: "7495cab5-6597-4015-95ec-d161d756ee41", arquivo: "chegadas_2018.csv" },
    AnoChegadas { ano: 2019, recurso_id: "24057369-a5dc-45b4-a66f-9a6a8f8e3422", arquivo: "chegadas_2019.csv" },
    AnoChegadas { ano: 2020, recurso_id: "92e6e604-156f-4e69-830b-9c34abcd14cb", arquivo: "chegadas_2020.csv" },
    AnoChegadas { ano: 2021, recurso_id: "21f188c3-5d93-4124-ae5c-135490a26acf", arquivo: "chegadas_2021.csv" },
    AnoChegadas { ano: 2022, recurso_id: "8df14749-5e61-4887-90d0-c9ad7af3a14c", arquivo: "chegadas_2022.csv" },
    AnoChegadas { ano: 2023, recurso_id: "24761490-6668-4a55-84a6-994d0e9cf84d", arquivo: "chegadas_2023.csv" },
    AnoChegadas { ano: 2024, recurso_id: "a1336a87-2458-4034-bc13-fb4e3523db2f", arquivo: "chegadas_2024.csv" },
    AnoChegadas { ano: 2025, recurso_id: "53dd37d0-42fe-4c23-9a9a-8248191d06e8", arquivo: "chegadas_2025.csv" },
    AnoChegadas { ano: 2026, recurso_id: "68342f44-8f7a-472a-a099-0d3a7d7d6a8f", arquivo: "base_chegadas_2026_acum_08_agosto.csv" },
];

fn url_do_recurso(ano: &AnoChegadas) -> String {
    format!(
        "https://{HOST}/dataset/{DATASET}/resource/{}/download/{}",
        ano.recurso_id, ano.arquivo
    )
}

/// Chegada de turistas internacionais ao Brasil (Ministério do Turismo).
pub struct FonteTurismoChegadas;

impl FonteDeDados for FonteTurismoChegadas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Turismo - chegada de turistas internacionais ao Brasil, por país de residência (1989–2026)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: HOST,
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        ANOS.iter()
            .map(|ano| {
                let ano_str = ano.ano.to_string();
                let tarefa = TarefaDeArquivo::nova(
                    ano_str.clone(),
                    NOME,
                    format!("ano={}", ano.ano),
                    url_do_recurso(ano),
                );
                TarefaPlanejada::nova(tarefa).com_etiquetas([
                    ("dataset", "chegadas-internacionais"),
                    ("ano", &ano_str),
                ])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use todeolho_nucleo::FiltroDeTarefas;

    #[test]
    fn total_de_tarefas() {
        assert_eq!(FonteTurismoChegadas.criar_tarefas().len(), ANOS.len());
        assert_eq!(ANOS.len(), 38);
    }

    #[test]
    fn nomes_sao_unicos() {
        let tarefas = FonteTurismoChegadas.criar_tarefas();
        let nomes: std::collections::HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn url_segue_padrao_ckan() {
        let url = url_do_recurso(&ANOS[0]);
        assert!(url.starts_with("https://dados.turismo.gov.br/dataset/"));
        assert!(url.contains("/resource/"));
        assert!(url.ends_with(".csv"));
    }

    #[test]
    fn filtra_por_ano() {
        let tarefas = FonteTurismoChegadas.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["ano=2019".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "2019");
    }

    #[test]
    fn etiquetas_presentes() {
        let tarefas = FonteTurismoChegadas.criar_tarefas();
        for tarefa in &tarefas {
            assert_eq!(tarefa.etiquetas.get("dataset").unwrap(), "chegadas-internacionais");
            assert!(tarefa.etiquetas.contains_key("ano"));
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteTurismoChegadas.sob_demanda());
    }

    #[test]
    fn sem_credencial_e_segue_redirecionamento() {
        let perfil = FonteTurismoChegadas.perfil_http();
        assert!(perfil.credencial.is_none());
        assert!(matches!(
            perfil.redirecionamento,
            todeolho_nucleo::RedirecionamentoHttp::Seguir
        ));
    }

    #[test]
    fn ano_2026_tem_nome_especial() {
        let ultimo = ANOS.last().unwrap();
        assert_eq!(ultimo.ano, 2026);
        assert!(ultimo.arquivo.contains("acum"));
    }
}
