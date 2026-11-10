# Pesquisa inicial: remuneração municipal

**Issue:** [#76](https://github.com/rikemorais/todeolhoaibr/issues/76)  
**Data da pesquisa:** 2026-11-10  
**Estado:** cobertura do TCE-CE validada e ingestão agregada da competência 2025-12 implementada para 184 municípios; comparação com fontes municipais ainda pendente.

## Resumo

O melhor primeiro agregador é o Sistema de Informações Municipais do TCE-CE:
na competência 2025-12, retornou ao menos uma folha para todos os 184 municípios
com código IBGE. Fortaleza também publica arquivos CSV mensais de servidores no
[catálogo oficial de dados abertos](https://dados.fortaleza.ce.gov.br/dataset/servidores),
uma fonte municipal independente que permite validar a cobertura e os totais do
TCE.

A [API de Dados Abertos do SIM](https://api-dados-abertos.tce.ce.gov.br/sim/)
lista recursos de Cadastro de Pessoal e Folha, incluindo agentes municipais,
itens remuneratórios, folhas de pagamento e agentes por folha. A API permite até
1.000 registros por requisição e paginação por `$start_index`; aceita conexões
somente de IPs no Brasil.

O TCE-CE publica o [Manual do SIM 2026](https://www.tce.ce.gov.br/municipios/sim/documentacao-e-programas),
atualizado em 19/12/2025. A especificação OpenAPI declara autenticação Basic
global, mas as consultas GET de validação abaixo responderam sem cabeçalho
`Authorization`. Essa diferença entre esquema e comportamento observado deve
ser confirmada antes de depender de acesso anônimo em produção.

## Fontes candidatas

| Fonte | Evidência oficial | Avaliação | Pendências |
| --- | --- | --- | --- |
| TCE-CE SIM | [API oficial](https://api-dados-abertos.tce.ce.gov.br/sim/) lista `folhas_pagamentos` e `agentes_publicos_folha`; [manual SIM](https://www.tce.ce.gov.br/municipios/sim/documentacao-e-programas) publicado por exercício | Melhor primeiro candidato: API estadual com resposta estruturada; prova de 2025-12 encontrou ao menos uma folha nos 184 municípios com código IBGE | Completar a comparação municipal, confirmar conectividade do runtime e divergência de autenticação, e definir tratamento dos CPFs no endpoint nominal |
| Prefeitura de Fortaleza | [Catálogo oficial de dados abertos](https://dados.fortaleza.ce.gov.br/dataset/servidores) publica CSVs mensais da folha; o arquivo 2025-12 tem 73.393 linhas e 20 colunas, incluindo órgão, servidor, vínculo, proventos, descontos e líquido | Fonte municipal reutilizável e comparador independente; arquivo CSV de 2025-12 tem 26.184.481 bytes | Alinhar escopo e categorias com o SIM antes de comparar totais; metadado CKAN do conjunto não declara licença; arquivos incluem nome e valores de remuneração |
| Prefeitura de Porto Alegre | [FAQ oficial](https://transparencia.portoalegre.rs.gov.br/perguntas-frequentes) descreve pesquisa de remuneração por órgão, tipo de folha e competência; afirma atualização mensal até dia 5 | Fonte útil para comparação independente, orientada a consulta web | A página de remuneração usa iframe; não foi localizada nesta triagem uma API ou exportação em lote estável |
| Capitais e demais TCEs | Não avaliados em profundidade nesta etapa | Expandir por adaptador após confirmar formatos estruturados e termos de uso | Fazer inventário por região e priorizar API ou arquivo oficial reaproveitável |

## Prova de viabilidade do TCE-CE

Em 10/11/2026, foram feitas consultas GET somente de leitura a partir de um IP
brasileiro. `GET /municipios` retornou 185 códigos; 184 tinham código IBGE
municipal. Fortaleza foi identificada pelo código SIM `057` e código IBGE
`2304400`.

O OpenAPI confirma que `GET /folhas_pagamentos` exige `codigo_municipio`,
`exercicio_orcamento` e `data_referencia_doc`, aceita `$count` até 1.000 e
`$start_index` para paginação. O exercício usa ano terminado em `00` (por
exemplo, `202500`); a competência e a referência documental usam `YYYYMM` (por
exemplo, `202512`). Com esses parâmetros, Fortaleza retornou folhas para a
competência de dezembro de 2025.

Foi feita uma varredura de presença da competência `2025-12`, com `$count=1`
para cada um dos 184 códigos municipais com IBGE. Todos retornaram ao menos uma
linha e nenhuma chamada falhou. Isso comprova presença de dados na competência,
mas não a entrega de todas as folhas ou de todos os agentes. O endpoint
`GET /agentes_publicos_folha` também respondeu; seu esquema inclui CPF do agente
e valor por item remuneratório. Nenhuma linha nominal foi guardada na pesquisa.

## Fonte municipal: Fortaleza

O [catálogo de dados abertos da Prefeitura de Fortaleza](https://dados.fortaleza.ce.gov.br/dataset/servidores)
publica um CSV por mês. O recurso [relacao_202512.csv](https://dados.fortaleza.ce.gov.br/dataset/servidores/resource/3527c08d-abea-454d-8968-ace527fba91b)
foi atualizado em 22/12/2025, tem 26.184.481 bytes e contém 73.393 linhas em 20 colunas. Os campos
incluem órgão/entidade, nome do servidor, tipo de vínculo, cargo, proventos,
descontos e líquido; não há coluna de CPF nesse arquivo. O conjunto mais recente
no catálogo foi atualizado em 31/07/2026 e inclui recursos até julho de 2026.

O metadado CKAN não declara uma licença para o conjunto. A [página municipal de
Dados Abertos](https://transparencia.fortaleza.ce.gov.br/index.php/dadosAbertos)
descreve os dados publicados como não sigilosos e sem restrições de uso; a
ingestão deve preservar essa procedência e registrar a data de coleta.

Uma soma inicial dos proventos no CSV de dezembro foi comparada com a soma do
campo `valor_total_item_orc` das folhas do SIM para Fortaleza. Os totais não
coincidiram. Ainda não foi comprovada equivalência entre os escopos por
órgão/unidade, nem entre as categorias remuneratórias; portanto, essa comparação
não é validação de completude. O CSV contém nomes e remuneração, enquanto o
endpoint nominal do SIM inclui CPF, então a política de minimização deve ser
definida antes de persistir essas respostas na Raw.

## Decisão e próximos passos

O adaptador de folhas agregadas do TCE-CE foi implementado para a competência
2025-12, particionado por município e com manifesto próprio; consulte a
[documentação da fonte](remuneracao-municipal-tce-ce.md). A execução em produção
precisa confirmar conectividade e completude das páginas. O CSV de Fortaleza
continua sendo uma fonte independente para comparação, mas os totais não devem
ser reconciliados até alinhar órgão/unidade e categorias. Depois desses controles,
avaliar outras capitais sem presumir que compartilham um esquema.

As fontes contêm dados pessoais em contexto de remuneração pública; o endpoint
nominal do TCE retorna CPF e o CSV municipal retorna nome. Antes de persistir
respostas brutas, a ingestão precisa definir minimização, proteção e retenção
desses identificadores. O manifesto deve registrar município, endpoint ou URL
do arquivo, competência, parâmetros de paginação quando aplicáveis, instante de
coleta, quantidade recebida e resultado da validação, sem incluir nomes nem
CPFs.

## Próximas verificações

1. Confirmar a autenticação declarada no OpenAPI versus as respostas sem
   credenciais observadas.
2. Executar e auditar os manifestos da competência `2025-12`; alinhar unidades e
   categorias com o arquivo municipal antes de comparar totais.
3. Verificar conectividade do runtime de produção, pois a API limita o acesso
   a IPs no Brasil.
4. Confirmar a licença específica do CSV municipal e definir minimização,
   proteção e retenção dos dados nominais antes de armazená-los.
5. Validar a primeira execução da Raw antes de ampliar para outras competências;
   definir minimização, proteção e retenção antes de ingerir dados nominais.
