# Pesquisa inicial: remuneração municipal

**Issue:** [#76](https://github.com/rikemorais/todeolhoaibr/issues/76)  
**Data da pesquisa:** 2026-11-10  
**Estado:** TCE-CE validado como fonte candidata; ingestão ainda não ativada.

## Resumo

O primeiro candidato a agregador é o Sistema de Informações Municipais do
TCE-CE. A [API de Dados Abertos do SIM](https://api-dados-abertos.tce.ce.gov.br/sim/)
lista recursos de Cadastro de Pessoal e Folha, incluindo agentes municipais,
itens remuneratórios, folhas de pagamento e agentes por folha. A documentação
permite até 1.000 registros por requisição e paginação por `$start_index`; o
acesso é permitido somente a IPs no Brasil.

O TCE-CE publica o [Manual do SIM 2026](https://www.tce.ce.gov.br/municipios/sim/documentacao-e-programas),
atualizado em 19/12/2025. A especificação OpenAPI declara autenticação Basic
global, mas as consultas GET de validação abaixo responderam sem cabeçalho
`Authorization`. Essa diferença entre esquema e comportamento observado deve
ser confirmada antes de depender de acesso anônimo em produção.

## Fontes candidatas

| Fonte | Evidência oficial | Avaliação | Pendências |
| --- | --- | --- | --- |
| TCE-CE SIM | [API oficial](https://api-dados-abertos.tce.ce.gov.br/sim/) lista `folhas_pagamentos` e `agentes_publicos_folha`; [manual SIM](https://www.tce.ce.gov.br/municipios/sim/documentacao-e-programas) publicado por exercício | Melhor primeiro candidato: API estadual com resposta estruturada; prova de 2025-12 encontrou ao menos uma folha nos 184 municípios com código IBGE | Completar a comparação municipal, confirmar conectividade do runtime e divergência de autenticação, e definir tratamento dos CPFs no endpoint nominal |
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

## Decisão proposta

Prosseguir com um adaptador do TCE-CE como primeira fonte municipal. Antes de
publicar ingestão, paginar integralmente uma competência, comparar folhas e
totais com o portal municipal, e confirmar acesso a partir do runtime de
produção. Se esses controles falharem, avaliar portais de capitais como fontes
independentes, sem presumir que compartilham um esquema.

O endpoint nominal retorna CPF e valores por item. Antes de persistir respostas
brutas, a ingestão precisa definir minimização, proteção e retenção desses
identificadores. O manifesto deve registrar município, endpoint, competência,
parâmetros de paginação, instante de coleta, quantidade recebida e resultado da
validação, sem incluir CPFs.

## Próximas verificações

1. Confirmar a autenticação declarada no OpenAPI versus as respostas sem
   credenciais observadas.
2. Paginar integralmente a competência `2025-12` e comparar uma folha e seus
   totais com o portal de Fortaleza.
3. Verificar conectividade do runtime de produção, pois a API limita o acesso
   a IPs no Brasil.
4. Definir minimização e retenção de CPF antes de armazenar dados nominais.
5. Implementar a ingestão e o manifesto após esses controles.
