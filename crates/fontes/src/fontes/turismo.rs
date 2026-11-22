//! Turismo — dados abertos do Ministério do Turismo.

use std::time::Duration;
use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const HOST: &str = "dados.turismo.gov.br";

fn perfil() -> PerfilHttp {
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

fn url_ckan(dataset: &str, recurso_id: &str, arquivo: &str) -> String {
    format!("https://{HOST}/dataset/{dataset}/resource/{recurso_id}/download/{arquivo}")
}

// ── Chegadas internacionais ─────────────────────────────────────────

const NOME_CHEGADAS: &str = "turismo-chegadas";
const DATASET_CHEGADAS: &str = "184e0ddd-7eaf-488d-ad84-0331219d6e99";

struct AnoChegadas {
    ano: u16,
    recurso_id: &'static str,
    arquivo: &'static str,
}

const ANOS: &[AnoChegadas] = &[
    AnoChegadas {
        ano: 1989,
        recurso_id: "e1a22b7d-fb15-4f66-bd83-9733968ae75f",
        arquivo: "chegadas_1989.csv",
    },
    AnoChegadas {
        ano: 1990,
        recurso_id: "23882d05-d82b-4537-a5c7-c2cdf93668c0",
        arquivo: "chegadas_1990.csv",
    },
    AnoChegadas {
        ano: 1991,
        recurso_id: "16d8af69-487e-4f22-a9b5-1b4bfa761e0d",
        arquivo: "chegadas_1991.csv",
    },
    AnoChegadas {
        ano: 1992,
        recurso_id: "73e509d4-daf5-4b82-836a-e329c0ec77c9",
        arquivo: "chegadas_1992.csv",
    },
    AnoChegadas {
        ano: 1993,
        recurso_id: "f4ee84f1-e4b4-4183-9357-5f3509f5685c",
        arquivo: "chegadas_1993.csv",
    },
    AnoChegadas {
        ano: 1994,
        recurso_id: "425b1cce-653b-419f-b4f8-10b4bd51d30c",
        arquivo: "chegadas_1994.csv",
    },
    AnoChegadas {
        ano: 1995,
        recurso_id: "59a90159-8b32-43e7-82a0-4a59f39732f4",
        arquivo: "chegadas_1995.csv",
    },
    AnoChegadas {
        ano: 1996,
        recurso_id: "46240329-9ac4-4322-9717-f86b06b78c5c",
        arquivo: "chegadas_1996.csv",
    },
    AnoChegadas {
        ano: 1997,
        recurso_id: "4c38df5d-968d-4197-b251-c7fdfdddfb09",
        arquivo: "chegadas_1997.csv",
    },
    AnoChegadas {
        ano: 1998,
        recurso_id: "33268202-2dfa-4604-a919-f419ffb8d448",
        arquivo: "chegadas_1998.csv",
    },
    AnoChegadas {
        ano: 1999,
        recurso_id: "a472fd00-638e-4ae9-9a7d-e5270be4a5ee",
        arquivo: "chegadas_1999.csv",
    },
    AnoChegadas {
        ano: 2000,
        recurso_id: "8a879066-8a14-4576-802f-bdc2b65ab9d7",
        arquivo: "chegadas_2000.csv",
    },
    AnoChegadas {
        ano: 2001,
        recurso_id: "7ff96ec4-80a9-4406-8ca1-61f52b1a94ca",
        arquivo: "chegadas_2001.csv",
    },
    AnoChegadas {
        ano: 2002,
        recurso_id: "a0236a28-54a5-4b29-8f29-ea6121ed037f",
        arquivo: "chegadas_2002.csv",
    },
    AnoChegadas {
        ano: 2003,
        recurso_id: "b963ba83-8d54-43a2-a5ca-979912743dfb",
        arquivo: "chegadas_2003.csv",
    },
    AnoChegadas {
        ano: 2004,
        recurso_id: "482362d3-5af4-4c1a-b74a-a5ef51906897",
        arquivo: "chegadas_2004.csv",
    },
    AnoChegadas {
        ano: 2005,
        recurso_id: "1935236f-1e1d-4ed9-bb5c-84ff2f955b58",
        arquivo: "chegadas_2005.csv",
    },
    AnoChegadas {
        ano: 2006,
        recurso_id: "11a1514c-b496-408c-9810-03bce31e1605",
        arquivo: "chegadas_2006.csv",
    },
    AnoChegadas {
        ano: 2007,
        recurso_id: "24577db6-bc12-4db1-9146-4ffa00e36b17",
        arquivo: "chegadas_2007.csv",
    },
    AnoChegadas {
        ano: 2008,
        recurso_id: "a4934a01-e12d-4a30-8a90-8f353b036195",
        arquivo: "chegadas_2008.csv",
    },
    AnoChegadas {
        ano: 2009,
        recurso_id: "942e84f4-93e7-406f-8314-e860cd1d80d3",
        arquivo: "chegadas_2009.csv",
    },
    AnoChegadas {
        ano: 2010,
        recurso_id: "44cee3ac-a507-49c1-9320-20ecdb7353c1",
        arquivo: "chegadas_2010.csv",
    },
    AnoChegadas {
        ano: 2011,
        recurso_id: "410f90a1-7287-4487-a8a3-142f8c37d87c",
        arquivo: "chegadas_2011.csv",
    },
    AnoChegadas {
        ano: 2012,
        recurso_id: "ec9c41a2-6b1e-46b3-a93f-f46e0adfc2d7",
        arquivo: "chegadas_2012.csv",
    },
    AnoChegadas {
        ano: 2013,
        recurso_id: "97dbba8c-2c8e-4055-baae-6361c3ba2e79",
        arquivo: "chegadas_2013.csv",
    },
    AnoChegadas {
        ano: 2014,
        recurso_id: "669597f4-217f-4762-ab47-80c17238ce64",
        arquivo: "chegadas_2014.csv",
    },
    AnoChegadas {
        ano: 2015,
        recurso_id: "e0e4ffa2-fab1-4a06-a5a5-3b8986a1a331",
        arquivo: "chegadas_2015.csv",
    },
    AnoChegadas {
        ano: 2016,
        recurso_id: "7a04c1d4-482d-4359-b04e-0907b6407c08",
        arquivo: "chegadas_2016.csv",
    },
    AnoChegadas {
        ano: 2017,
        recurso_id: "86134fdc-fbfe-44d0-ad2e-315b16f70233",
        arquivo: "chegadas_2017.csv",
    },
    AnoChegadas {
        ano: 2018,
        recurso_id: "7495cab5-6597-4015-95ec-d161d756ee41",
        arquivo: "chegadas_2018.csv",
    },
    AnoChegadas {
        ano: 2019,
        recurso_id: "24057369-a5dc-45b4-a66f-9a6a8f8e3422",
        arquivo: "chegadas_2019.csv",
    },
    AnoChegadas {
        ano: 2020,
        recurso_id: "92e6e604-156f-4e69-830b-9c34abcd14cb",
        arquivo: "chegadas_2020.csv",
    },
    AnoChegadas {
        ano: 2021,
        recurso_id: "21f188c3-5d93-4124-ae5c-135490a26acf",
        arquivo: "chegadas_2021.csv",
    },
    AnoChegadas {
        ano: 2022,
        recurso_id: "8df14749-5e61-4887-90d0-c9ad7af3a14c",
        arquivo: "chegadas_2022.csv",
    },
    AnoChegadas {
        ano: 2023,
        recurso_id: "24761490-6668-4a55-84a6-994d0e9cf84d",
        arquivo: "chegadas_2023.csv",
    },
    AnoChegadas {
        ano: 2024,
        recurso_id: "a1336a87-2458-4034-bc13-fb4e3523db2f",
        arquivo: "chegadas_2024.csv",
    },
    AnoChegadas {
        ano: 2025,
        recurso_id: "53dd37d0-42fe-4c23-9a9a-8248191d06e8",
        arquivo: "chegadas_2025.csv",
    },
    AnoChegadas {
        ano: 2026,
        recurso_id: "68342f44-8f7a-472a-a099-0d3a7d7d6a8f",
        arquivo: "base_chegadas_2026_acum_08_agosto.csv",
    },
];

/// Chegada de turistas internacionais ao Brasil (Ministério do Turismo).
pub struct FonteTurismoChegadas;

impl FonteDeDados for FonteTurismoChegadas {
    fn nome(&self) -> &str {
        NOME_CHEGADAS
    }

    fn descricao(&self) -> &str {
        "Turismo - chegada de turistas internacionais ao Brasil, por país de residência (1989–2026)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        perfil()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        ANOS.iter()
            .map(|ano| {
                let ano_str = ano.ano.to_string();
                let mut tarefa = TarefaDeArquivo::nova(
                    ano_str.clone(),
                    NOME_CHEGADAS,
                    format!("ano={}", ano.ano),
                    url_ckan(DATASET_CHEGADAS, ano.recurso_id, ano.arquivo),
                );
                if ano.ano == 2026 {
                    tarefa = tarefa.aceitar_ausencia();
                }
                TarefaPlanejada::nova(tarefa)
                    .com_etiquetas([("dataset", "chegadas-internacionais"), ("ano", &ano_str)])
            })
            .collect()
    }
}

// ── Meios de hospedagem ─────────────────────────────────────────────

const NOME_HOSPEDAGEM: &str = "turismo-hospedagem";
const DATASET_HOSPEDAGEM: &str = "d2333d1b-db1e-438b-955a-028db80a031e";

struct RecursoHospedagem {
    ano: u16,
    trimestre: Option<u8>,
    recurso_id: &'static str,
    arquivo: &'static str,
}

const HOSPEDAGEM: &[RecursoHospedagem] = &[
    // 2006–2015: anuais
    RecursoHospedagem {
        ano: 2006,
        trimestre: None,
        recurso_id: "fb7eeca9-fc3c-4e40-a3f8-bfe666755780",
        arquivo: "meiodehospedagem2006cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2007,
        trimestre: None,
        recurso_id: "12222735-b609-4f6c-9c8b-a83c2b6a4a02",
        arquivo: "meiodehospedagem2007cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2008,
        trimestre: None,
        recurso_id: "8a7c534a-ab65-474c-8bdf-d27fe25638d9",
        arquivo: "meiodehospedagem2008cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2009,
        trimestre: None,
        recurso_id: "dd6c0c19-cadf-47cf-94b0-845df8801902",
        arquivo: "meiodehospedagem2009cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2010,
        trimestre: None,
        recurso_id: "bdb3f2a9-be7f-4b86-a87b-5e3d2bca3365",
        arquivo: "meiodehospedagem2010cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2011,
        trimestre: None,
        recurso_id: "03fd1c5b-6c6d-47ee-ba87-d9354844f802",
        arquivo: "meiodehospedagem2011cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2012,
        trimestre: None,
        recurso_id: "c89efc8a-f61c-4ac4-8e27-8b87002dc175",
        arquivo: "meiodehospedagem2012cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2013,
        trimestre: None,
        recurso_id: "e6b6f05a-14c3-4dc0-bcdf-9fb5b6429526",
        arquivo: "meiodehospedagem2013cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2014,
        trimestre: None,
        recurso_id: "ae539eb4-059b-4aee-bc9f-74e8cb98b403",
        arquivo: "meiodehospedagem2014cadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2015,
        trimestre: None,
        recurso_id: "6977faf0-9066-49b1-bf30-32bee9027570",
        arquivo: "meiodehospedagem2015cadasturpj.csv",
    },
    // 2016: trimestral
    RecursoHospedagem {
        ano: 2016,
        trimestre: Some(1),
        recurso_id: "04babccb-7daf-4694-8624-cd12caf385c7",
        arquivo: "meiodehospedagem201601trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2016,
        trimestre: Some(2),
        recurso_id: "d370eb0f-d6f8-4718-95d0-6c5f8cb2556b",
        arquivo: "meiodehospedagem201602trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2016,
        trimestre: Some(3),
        recurso_id: "5e4e9859-ec4e-4067-9bed-1881d2f2f924",
        arquivo: "meiodehospedagem201603trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2016,
        trimestre: Some(4),
        recurso_id: "a57d648e-ffaf-40fa-8272-b180ce01c5c6",
        arquivo: "meiodehospedagem201604trimestrecadasturpj.csv",
    },
    // 2017
    RecursoHospedagem {
        ano: 2017,
        trimestre: Some(1),
        recurso_id: "fe2ca6f1-8f55-4ec4-b73f-8e631c95ed13",
        arquivo: "meiodehospedagem201701trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2017,
        trimestre: Some(2),
        recurso_id: "cbdab625-2d15-4c18-8247-23fdc8682ba9",
        arquivo: "meiodehospedagem201702trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2017,
        trimestre: Some(3),
        recurso_id: "0607dfde-67ae-4b33-944c-53022cc2ff03",
        arquivo: "meiodehospedagem201703trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2017,
        trimestre: Some(4),
        recurso_id: "1d8cf427-abaf-4f00-881a-b52fcd53f46c",
        arquivo: "meiodehospedagem201704trimestrecadasturpj.csv",
    },
    // 2018
    RecursoHospedagem {
        ano: 2018,
        trimestre: Some(1),
        recurso_id: "68fbaf2d-d7bb-445f-926f-a7ffe94895fe",
        arquivo: "meiodehospedagem201801trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2018,
        trimestre: Some(2),
        recurso_id: "31533c45-9ef9-48cf-8001-f3a59741c676",
        arquivo: "meiodehospedagem201802trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2018,
        trimestre: Some(3),
        recurso_id: "a12e8f63-82ad-496c-a290-a829e5e2bd98",
        arquivo: "meiodehospedagem201803trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2018,
        trimestre: Some(4),
        recurso_id: "71a466b7-aa2d-4e5d-b084-817b984626e1",
        arquivo: "meiodehospedagem201804trimestrecadasturpj.csv",
    },
    // 2019
    RecursoHospedagem {
        ano: 2019,
        trimestre: Some(1),
        recurso_id: "971ceb98-7066-479b-8a42-f764ebcf4795",
        arquivo: "meiodehospedagem201901trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2019,
        trimestre: Some(2),
        recurso_id: "569cf182-2f64-4ffb-9a62-26f167b90fa8",
        arquivo: "meiodehospedagem201902trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2019,
        trimestre: Some(3),
        recurso_id: "6ccb906e-4b18-4de7-b9fb-a9593f916e22",
        arquivo: "meiodehospedagem201903trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2019,
        trimestre: Some(4),
        recurso_id: "23286999-972d-4f64-8108-190956001e5d",
        arquivo: "meiodehospedagem201904trimestrecadasturpj.csv",
    },
    // 2020
    RecursoHospedagem {
        ano: 2020,
        trimestre: Some(1),
        recurso_id: "3a889034-b544-493d-b1ee-866a470b763e",
        arquivo: "meiodehospedagem202001trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2020,
        trimestre: Some(2),
        recurso_id: "7bca2b83-303d-4bc7-b28f-2638527230dd",
        arquivo: "meiodehospedagem202002trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2020,
        trimestre: Some(3),
        recurso_id: "fbbb8947-fabd-4bdb-a166-41230cd66ca3",
        arquivo: "meiodehospedagem202003trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2020,
        trimestre: Some(4),
        recurso_id: "1bd1346d-950d-4f5f-b5dd-8214a2c8e5db",
        arquivo: "meiodehospedagem202004trimestrecadasturpj.csv",
    },
    // 2021
    RecursoHospedagem {
        ano: 2021,
        trimestre: Some(1),
        recurso_id: "bce30034-d2dc-4b2b-bc07-ed7b3f03437c",
        arquivo: "meiodehospedagem202101trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2021,
        trimestre: Some(2),
        recurso_id: "bd4640b6-d98b-46f1-9ce9-0219f4dc8dba",
        arquivo: "meiodehospedagem202102trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2021,
        trimestre: Some(3),
        recurso_id: "a59bb412-6ee8-4309-bc27-56e205602dd7",
        arquivo: "meiodehospedagem202103trimestrecadasturpj.csv",
    },
    RecursoHospedagem {
        ano: 2021,
        trimestre: Some(4),
        recurso_id: "5edb741b-502c-4ded-be8e-8dcff9d8d157",
        arquivo: "meio-de-hospedagem-.csv",
    },
    // 2022
    RecursoHospedagem {
        ano: 2022,
        trimestre: Some(1),
        recurso_id: "0ab4dbc3-5f55-43b0-b3a5-b37239a9b23f",
        arquivo: "meio-de-hospedagem-.csv",
    },
    RecursoHospedagem {
        ano: 2022,
        trimestre: Some(2),
        recurso_id: "7a9e1605-11af-4fe2-a0ce-994dafa83e29",
        arquivo: "meio-de-hospedagem-.xlsx",
    },
    RecursoHospedagem {
        ano: 2022,
        trimestre: Some(3),
        recurso_id: "4794e774-66e7-42b9-8bbf-9cf19a1d7694",
        arquivo: "meio-de-hospedagem-.xlsx",
    },
    RecursoHospedagem {
        ano: 2022,
        trimestre: Some(4),
        recurso_id: "28ea0906-6681-4059-9a3f-a532913f91b3",
        arquivo: "meio-de-hospedagem.xls",
    },
    // 2023
    RecursoHospedagem {
        ano: 2023,
        trimestre: Some(1),
        recurso_id: "4f4edad8-2670-429e-b438-47bc6ce9f724",
        arquivo: "meio-de-hospedagem.xls",
    },
    RecursoHospedagem {
        ano: 2023,
        trimestre: Some(2),
        recurso_id: "4f8e3cba-8070-4715-8420-cdb7424f2340",
        arquivo: "meio-de-hospedagem.xls",
    },
    RecursoHospedagem {
        ano: 2023,
        trimestre: Some(3),
        recurso_id: "6d5cb937-d66c-404d-9a02-7b7e59702d20",
        arquivo: "meio-de-hospedagem.xls",
    },
    RecursoHospedagem {
        ano: 2023,
        trimestre: Some(4),
        recurso_id: "26e0f639-97eb-4682-8ebd-9856a7ff0610",
        arquivo: "meio-de-hospedagem.xls",
    },
    // 2024
    RecursoHospedagem {
        ano: 2024,
        trimestre: Some(1),
        recurso_id: "8e8d5e6e-dadf-47fc-9e41-15915422020f",
        arquivo: "meio-de-hospedagem-.xls",
    },
    RecursoHospedagem {
        ano: 2024,
        trimestre: Some(2),
        recurso_id: "357776b4-4da1-4238-9b3a-3c3599a08a22",
        arquivo: "meio-de-hospedagem.xls",
    },
    RecursoHospedagem {
        ano: 2024,
        trimestre: Some(3),
        recurso_id: "6d5aa8ff-9ed8-4bca-8704-17fb89f71eb7",
        arquivo: "meio-de-hospedagem.xlsx",
    },
    RecursoHospedagem {
        ano: 2024,
        trimestre: Some(4),
        recurso_id: "ea5f4986-c468-4e2c-a873-12f35b7d04f1",
        arquivo: "relatorio-meios-de-hospedagem_-situacao_4-trimestre.xlsx",
    },
    // 2025
    RecursoHospedagem {
        ano: 2025,
        trimestre: Some(1),
        recurso_id: "c733c3c1-2451-4473-916b-7e436b77a40d",
        arquivo: "meios-de-hospedagem-data-situacao_-31_03_2025.xlsx",
    },
    RecursoHospedagem {
        ano: 2025,
        trimestre: Some(2),
        recurso_id: "7c35e6e0-b261-4f8f-ab51-b6adb6db0900",
        arquivo: "meios-de-hospedagem-data-situacao_-30_06_2025.xlsx",
    },
    RecursoHospedagem {
        ano: 2025,
        trimestre: Some(3),
        recurso_id: "6a9643e6-6372-4d11-9b3d-51f8fd4cfa92",
        arquivo: "meios-de-hospedagem-data-situacao_-30_09_2025.xlsx",
    },
    RecursoHospedagem {
        ano: 2025,
        trimestre: Some(4),
        recurso_id: "efeb6b18-dd5a-48e7-83c3-2f53fd422476",
        arquivo: "meio-de-hospedagem.xlsx",
    },
    // 2026
    RecursoHospedagem {
        ano: 2026,
        trimestre: Some(1),
        recurso_id: "938cb620-7252-4cd0-9def-443dd2fe3f3b",
        arquivo: "meio-de-hospedagem-1-trimestre-2026.xlsx",
    },
    RecursoHospedagem {
        ano: 2026,
        trimestre: Some(2),
        recurso_id: "7e385ec2-57d8-446c-9cc0-f5f7dadd1fdf",
        arquivo: "meios-de-hospedagem-2-trimestre-2026.xlsx",
    },
];

/// Meios de hospedagem cadastrados no Cadastur (Ministério do Turismo).
pub struct FonteTurismoHospedagem;

impl FonteDeDados for FonteTurismoHospedagem {
    fn nome(&self) -> &str {
        NOME_HOSPEDAGEM
    }

    fn descricao(&self) -> &str {
        "Turismo - meios de hospedagem cadastrados no Cadastur (2006–2026, anual/trimestral)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        perfil()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        HOSPEDAGEM
            .iter()
            .map(|r| {
                let ano_str = r.ano.to_string();
                let (nome, recurso) = match r.trimestre {
                    Some(q) => (
                        format!("{}-Q{q}", r.ano),
                        format!("ano={}/trimestre={q}", r.ano),
                    ),
                    None => (ano_str.clone(), format!("ano={}", r.ano)),
                };
                let tarefa = TarefaDeArquivo::nova(
                    nome,
                    NOME_HOSPEDAGEM,
                    recurso,
                    url_ckan(DATASET_HOSPEDAGEM, r.recurso_id, r.arquivo),
                );
                TarefaPlanejada::nova(tarefa)
                    .com_etiquetas([("dataset", "meios-de-hospedagem"), ("ano", &ano_str)])
            })
            .collect()
    }
}

// ── Mapa do Turismo, Categorização e Competitividade ────────────────

const NOME_MAPA: &str = "turismo-mapa";
const DATASET_MAPA: &str = "569d8d6f-78df-46e9-88eb-009b4d033ada";
const DATASET_CATEGORIZACAO: &str = "categorizacao";
const DATASET_COMPETITIVIDADE: &str = "indice-de-competitividade";

struct RecursoCkan {
    ano: &'static str,
    recurso_id: &'static str,
    arquivo: &'static str,
}

const MAPA: &[RecursoCkan] = &[
    RecursoCkan { ano: "2004", recurso_id: "835ad7d3-41e4-4c38-8d54-07aa3886f943", arquivo: "2004-mapa-turismo.csv" },
    RecursoCkan { ano: "2006", recurso_id: "d5e0168d-d752-49c0-857e-435840ce1584", arquivo: "2006-mapa-turismo.csv" },
    RecursoCkan { ano: "2009", recurso_id: "cc93abe2-cf3d-401d-87d7-81dc85e9a043", arquivo: "2009-mapa-turismo.csv" },
    RecursoCkan { ano: "2013", recurso_id: "1f122bd3-1a65-46db-9707-4a9b4b7d0c1b", arquivo: "2013-mapa-turismo.csv" },
    RecursoCkan { ano: "2016", recurso_id: "2363118f-0528-42e1-80c3-3bc161a37434", arquivo: "2016-mapa-turismo.csv" },
    RecursoCkan { ano: "2017", recurso_id: "ce93dd47-aea9-44c2-b1ec-faf933bfb892", arquivo: "2017-mapa-turismo.csv" },
    RecursoCkan { ano: "2019", recurso_id: "dd05d926-ceaf-4259-87bc-541e159151e7", arquivo: "relatorio_mapa_2019_layout_mkt.xls" },
];

const CATEGORIZACAO: &[RecursoCkan] = &[
    RecursoCkan { ano: "2015", recurso_id: "0a960534-a1f0-4035-8b82-34bc724333e6", arquivo: "2016-categorizacao.csv" },
    RecursoCkan { ano: "2018", recurso_id: "07c4f97a-888f-4ab7-ad2e-d5f040d627a0", arquivo: "2017-categorizacao-v2.csv" },
    RecursoCkan { ano: "2019", recurso_id: "13e5cbd9-e18d-4c3f-aa47-8cf9c04ab32f", arquivo: "relatorio_categorizacao_2019-portal.xls" },
];

const COMPETITIVIDADE: &[RecursoCkan] = &[
    RecursoCkan { ano: "2008", recurso_id: "d62565da-d38f-4380-aa61-d1f53a9e28c4", arquivo: "2008-icompetitividade.csv" },
    RecursoCkan { ano: "2009", recurso_id: "7da6a0d9-6734-49b3-81d8-088b81b2b64a", arquivo: "2009-icompetitividade.csv" },
    RecursoCkan { ano: "2010", recurso_id: "d91c7032-aaa2-47e6-aa5c-58637e04e02c", arquivo: "2010-icompetitividade.csv" },
    RecursoCkan { ano: "2011", recurso_id: "ebc43787-080f-4bd1-8a6c-cf8eed1e22f2", arquivo: "2011-icompetitividade.csv" },
    RecursoCkan { ano: "2013", recurso_id: "f28f4622-0177-4093-8bba-21a6e75cf5a9", arquivo: "2013-icompetitividade.csv" },
    RecursoCkan { ano: "2014", recurso_id: "f85a8e93-393b-455f-9325-9e90ae3e03d0", arquivo: "2014-icompetitividade.csv" },
    RecursoCkan { ano: "2015", recurso_id: "4f1a693e-ef1d-41bc-ad53-3ef96459b6ca", arquivo: "2015-icompetitividade.csv" },
];

fn tarefas_subdataset<'a>(
    dataset_id: &'a str,
    subdataset: &'a str,
    recursos: &'a [RecursoCkan],
) -> impl Iterator<Item = TarefaPlanejada> + 'a {
    recursos.iter().map(move |r| {
        let tarefa = TarefaDeArquivo::nova(
            format!("{subdataset}/{}", r.ano),
            NOME_MAPA,
            format!("{subdataset}/ano={}", r.ano),
            url_ckan(dataset_id, r.recurso_id, r.arquivo),
        );
        TarefaPlanejada::nova(tarefa).com_etiquetas([
            ("dataset", subdataset),
            ("ano", r.ano),
        ])
    })
}

/// Mapa do Turismo, Categorização e Competitividade (Ministério do Turismo).
pub struct FonteTurismoMapa;

impl FonteDeDados for FonteTurismoMapa {
    fn nome(&self) -> &str {
        NOME_MAPA
    }

    fn descricao(&self) -> &str {
        "Turismo - mapa do turismo brasileiro, categorização de municípios e índice de competitividade"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        perfil()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas_subdataset(DATASET_MAPA, "mapa", MAPA)
            .chain(tarefas_subdataset(DATASET_CATEGORIZACAO, "categorizacao", CATEGORIZACAO))
            .chain(tarefas_subdataset(DATASET_COMPETITIVIDADE, "competitividade", COMPETITIVIDADE))
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use todeolho_nucleo::FiltroDeTarefas;

    // ── Chegadas ────────────────────────────────────────────────────

    #[test]
    fn chegadas_total_de_tarefas() {
        assert_eq!(FonteTurismoChegadas.criar_tarefas().len(), ANOS.len());
        assert_eq!(ANOS.len(), 38);
    }

    #[test]
    fn chegadas_nomes_sao_unicos() {
        let tarefas = FonteTurismoChegadas.criar_tarefas();
        let nomes: std::collections::HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn chegadas_url_segue_padrao_ckan() {
        let url = url_ckan(DATASET_CHEGADAS, ANOS[0].recurso_id, ANOS[0].arquivo);
        assert!(url.starts_with("https://dados.turismo.gov.br/dataset/"));
        assert!(url.contains("/resource/"));
        assert!(url.ends_with(".csv"));
    }

    #[test]
    fn chegadas_filtra_por_ano() {
        let tarefas = FonteTurismoChegadas.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["ano=2019".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "2019");
    }

    #[test]
    fn chegadas_etiquetas_presentes() {
        let tarefas = FonteTurismoChegadas.criar_tarefas();
        for tarefa in &tarefas {
            assert_eq!(
                tarefa.etiquetas.get("dataset").unwrap(),
                "chegadas-internacionais"
            );
            assert!(tarefa.etiquetas.contains_key("ano"));
        }
    }

    #[test]
    fn chegadas_e_sob_demanda() {
        assert!(FonteTurismoChegadas.sob_demanda());
    }

    #[test]
    fn chegadas_sem_credencial_e_segue_redirecionamento() {
        let perfil = FonteTurismoChegadas.perfil_http();
        assert!(perfil.credencial.is_none());
        assert!(matches!(
            perfil.redirecionamento,
            todeolho_nucleo::RedirecionamentoHttp::Seguir
        ));
    }

    #[test]
    fn chegadas_ano_2026_tem_nome_especial() {
        let ultimo = ANOS.last().unwrap();
        assert_eq!(ultimo.ano, 2026);
        assert!(ultimo.arquivo.contains("acum"));
    }

    // ── Hospedagem ──────────────────────────────────────────────────

    #[test]
    fn hospedagem_total_de_tarefas() {
        assert_eq!(
            FonteTurismoHospedagem.criar_tarefas().len(),
            HOSPEDAGEM.len()
        );
        assert_eq!(HOSPEDAGEM.len(), 52);
    }

    #[test]
    fn hospedagem_nomes_sao_unicos() {
        let tarefas = FonteTurismoHospedagem.criar_tarefas();
        let nomes: std::collections::HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn hospedagem_filtra_por_ano() {
        let tarefas = FonteTurismoHospedagem.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["ano=2020".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), 4);
        assert!(
            filtradas
                .iter()
                .all(|t| t.tarefa.nome().starts_with("2020"))
        );
    }

    #[test]
    fn hospedagem_anuais_sem_trimestre_no_nome() {
        let tarefas = FonteTurismoHospedagem.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["ano=2010".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "2010");
    }

    #[test]
    fn hospedagem_etiquetas_presentes() {
        let tarefas = FonteTurismoHospedagem.criar_tarefas();
        for tarefa in &tarefas {
            assert_eq!(
                tarefa.etiquetas.get("dataset").unwrap(),
                "meios-de-hospedagem"
            );
            assert!(tarefa.etiquetas.contains_key("ano"));
        }
    }

    #[test]
    fn hospedagem_e_sob_demanda() {
        assert!(FonteTurismoHospedagem.sob_demanda());
    }

    // ── Mapa / Categorização / Competitividade ──────────────────────

    #[test]
    fn mapa_total_de_tarefas() {
        let tarefas = FonteTurismoMapa.criar_tarefas();
        assert_eq!(tarefas.len(), 17);
    }

    #[test]
    fn mapa_nomes_sao_unicos() {
        let tarefas = FonteTurismoMapa.criar_tarefas();
        let nomes: std::collections::HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn mapa_filtra_por_dataset() {
        let tarefas = FonteTurismoMapa.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["dataset=competitividade".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), COMPETITIVIDADE.len());
    }

    #[test]
    fn mapa_etiquetas_presentes() {
        let tarefas = FonteTurismoMapa.criar_tarefas();
        for tarefa in &tarefas {
            assert!(tarefa.etiquetas.contains_key("dataset"));
            assert!(tarefa.etiquetas.contains_key("ano"));
        }
    }

    #[test]
    fn mapa_e_sob_demanda() {
        assert!(FonteTurismoMapa.sob_demanda());
    }
}
