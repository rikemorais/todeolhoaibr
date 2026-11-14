# Pesquisa: dados do Mapa de Empresas

**Escopo:** verificar se a fonte oficial permite ingerir estatísticas completas por UF e quadrimestre, com uma chave de idempotência `quadrimestre + UF`, como pede a [issue #35](https://github.com/rikemorais/todeolhoaibr/issues/35).

**Verificação:** 14/11/2026. Foram consultadas páginas e arquivos publicados pelo Governo Federal. Um XLSX oficial foi baixado e inspecionado localmente; nenhum dado foi adicionado ao repositório.

## Fontes oficiais encontradas

### Boletins quadrimestrais

O MEMP descreve o boletim como relatório de análise dos dados de abertura referentes ao quadrimestre. Há publicações históricas desde 2020 e publicação do 1º quadrimestre de 2025. Os arquivos são PDFs, por exemplo, [1º quadrimestre de 2025](https://www.gov.br/empresas-e-negocios/pt-br/mapa-de-empresas/boletins/mapa-de-empresas-boletim-1o-quadrimestre-2025-pdf.pdf) e [3º quadrimestre de 2024](https://www.gov.br/empresas-e-negocios/pt-br/mapa-de-empresas/boletins/boletim-do-mapa-de-empresas-3o-quad-2024.pdf/@@display-file/file). A página institucional confirma a periodicidade quadrimestral e o propósito analítico dos boletins ([MEMP — Mapa de Empresas](https://www.gov.br/memp/pt-br/acesso-a-informacao/acoes-e-programas/mapa-de-empresas)).

O boletim do 3º quadrimestre de 2024 não publica uma tabela completa com as quatro métricas para as 27 UFs: suas tabelas de abertura e fechamento por estado mostram rankings parciais (maiores e menores), além de comparações percentuais. Portanto, arquivar esses PDFs na Raw preserva a publicação oficial, mas não produz os dados tabulares completos por UF exigidos pela issue. A amostra pode ser conferida no texto do [boletim de 2024](https://www.gov.br/empresas-e-negocios/pt-br/mapa-de-empresas/boletins/boletim-do-mapa-de-empresas-3o-quad-2024.pdf/@@display-file/file).

### Exportação dos painéis

O serviço oficial orienta a selecionar abas e filtros no painel e então baixar os dados de um quadro como planilha. O formato “Dados” reflete os filtros já aplicados. Isso comprova exportação interativa, mas a documentação consultada não descreve uma API pública de dados nem um endpoint estável de exportação em lote ([serviço Mapa de Empresas — etapas 2–4](https://www.gov.br/pt-br/servicos/mapa-de-empresa)). A página dos painéis incorpora visualizações públicas da plataforma Qlik Sense do Serpro; isso, por si só, não constitui contrato de API para ingestão automatizada ([painéis oficiais](https://www.gov.br/empresas-e-negocios/pt-br/mapa-de-empresas/painel-mapa-de-empresas)).

### Planilhas XLSX mensais

O portal do MEMP publica planilhas mensais, por exemplo [ArquivoDados de agosto de 2026](https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos/08_arquivodados_ago26-mapa-de-empresas.xlsx). A página oficial descreve os painéis como atualizados mensalmente e permite exportar dados com os filtros escolhidos ([serviço Mapa de Empresas](https://www.gov.br/pt-br/servicos/mapa-de-empresa)).

Inspeção do XLSX publicado em agosto de 2026:

- As abas são `Abertas, Fechadas e Ativas`, `Inova Simples`, `Empresas Ativas por NJ e Porte` e `Gênero (UF, NJ e Porte)`.
- A primeira aba contém um bloco por município e um bloco agregado por UF, com campos de referência/geração e medidas de empresas abertas, fechadas e ativas.
- O arquivo é um snapshot mensal. Ele contém a dimensão UF, mas sua partição e referência são mensais, não quadrimestrais.
- Foram encontrados XLSX para meses recentes de 2026; os caminhos previsíveis para snapshots de fechamento de quadrimestre em 2023–2025 retornaram 404. Essa verificação pontual não prova inexistência de toda publicação antiga, mas não sustenta uma série histórica quadrimestral em XLSX nesse endpoint.

O portal lista as bases do MEMP e informa que elas são disponibilizadas gradualmente conforme cronograma; o Plano de Dados Abertos classifica o Mapa como conjunto de painéis estatísticos sobre a base CNPJ, com prazo de abertura previsto em junho de 2026 ([Dados Abertos do MEMP](https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos), [Plano de Dados Abertos 2025–2027](https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos/plano-de-dados-abertos-pda/plano-de-dados-abertos-6.pdf)).

## Viabilidade dos critérios da issue

| Critério | Evidência | Resultado |
| --- | --- | --- |
| Dados por UF e quadrimestre na Raw | PDFs cobrem quadrimestres, mas têm rankings estaduais parciais; XLSX tem dados agregados por UF, mas é snapshot mensal. | **Não atendido integralmente por um artefato oficial já publicado.** |
| Manifesto | A ingestão de arquivos existente pode preservar um XLSX ou PDF opaco e criar manifesto para esse artefato. | **Viável para cada arquivo/periodicidade real da fonte.** |
| Idempotência por quadrimestre + UF | O PDF é um arquivo por quadrimestre com vários estados e dados parciais; o XLSX é um arquivo mensal com todas as UFs. Nenhum deles tem um recurso oficial individual por `(quadrimestre, UF)`. | **Não é honesto declarar essa chave na ingestão sem baixar dados por UF, filtrar/transformar o conteúdo ou duplicar o arquivo inteiro em 27 recursos.** |
| Filtros de período e UF | A CLI filtra etiquetas de tarefas. O PDF permite no máximo selecionar o quadrimestre da tarefa, não a UF; o XLSX permite selecionar o mês/snapshot, enquanto UF está dentro do conteúdo. | **Período é viável conforme o artefato; filtro UF requer parsing/refino ou exportação interativa com filtro aplicado.** |

## Conclusão e caminhos coerentes

Os critérios exatos da issue não podem ser implementados simultaneamente com a ingestão opaca atual e os arquivos oficiais verificados. Há duas alternativas coerentes:

1. **Preservar o contrato Raw opaco:** ingerir as planilhas XLSX por mês de referência, uma tarefa idempotente por mês; manter todas as UFs no mesmo arquivo e fazer o recorte por UF no Bronze/Silver. Isso entrega os dados estruturados disponíveis e evita cópias 27 vezes, mas muda a periodicidade e a chave de idempotência pedidas.
2. **Manter quadrimestre:** ingerir o PDF oficial uma vez por quadrimestre, com manifesto, deixando explícito que o conteúdo não traz a tabela completa por UF e que UF não é filtro de tarefa; para cumprir os critérios de dados estruturados completos, seria necessário obter uma exportação oficial filtrada para cada UF/período ou implementar um cliente automatizado do mecanismo de exportação Qlik, cujo contrato não está documentado como API estável pelo MEMP.

Não se recomenda criar 27 tarefas que baixem e armazenem cópias idênticas do mesmo XLSX/PDF para simular idempotência por UF: isso duplicaria conteúdo e faria os filtros aparentarem restringir os dados quando o artefato ainda contém todos os estados.
