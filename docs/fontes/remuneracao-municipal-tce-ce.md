# Fonte `remuneracao-municipal-tce-ce` — folhas municipais agregadas

Ingestão dos totais orçamentários das folhas municipais publicados pelo Sistema de
Informações Municipais do Tribunal de Contas do Estado do Ceará (TCE-CE).

Documentação oficial: [API de Dados Abertos do SIM](https://api-dados-abertos.tce.ce.gov.br/sim/)
e [Manual do SIM](https://www.tce.ce.gov.br/municipios/sim/documentacao-e-programas).

## Escopo desta versão

- Competência fixa `2025-12`, usada na pesquisa de cobertura da [issue #76](https://github.com/rikemorais/todeolhoaibr/issues/76).
- Uma tarefa independente para cada um dos 184 códigos SIM com código IBGE validado.
- Consulta apenas `GET /folhas_pagamentos`, que retorna totais por folha e unidade orçamentária: total dos itens remuneratórios, deduções, vantagens extraorçamentárias e subsídio do prefeito.
- Paginação via `links.next`, com até 1.000 registros por página.
- A fonte é `sob_demanda`; ela não entra na execução `--todas`.

A pesquisa encontrou presença de ao menos uma folha para os 184 municípios na
competência. Isso não garante que cada página ou folha esteja completa; o manifesto
de cada município registra as respostas e URLs recebidas para auditoria.

## Dados e privacidade

O endpoint de totais não declara CPF nem nome de servidor no seu schema. A tarefa não
consulta `/agentes_publicos_folha`, que contém dados nominais. Assim, esta integração
armazena apenas o payload agregado fornecido pelo TCE. O refino poderá decidir depois
como apresentar totais por órgão/unidade e competência.

O OpenAPI declara autenticação Basic, embora as consultas GET de leitura da pesquisa
tenham respondido sem credenciais. O runtime deve confirmar acesso; o host também
restringe conexões a IPs no Brasil.

## Layout na Raw

```text
raw/remuneracao-municipal-tce-ce/folhas-pagamentos/municipio=057/competencia=2025-12/...
```

Cada município tem recurso e manifesto próprios, com a resposta original preservada
em bytes. As etiquetas `municipio_sim` e `competencia` permitem filtrar a ingestão.

```bash
docker compose run --rm --build ingestao executar remuneracao-municipal-tce-ce
```

## Limites

- A competência ingerida inicialmente é fixa em `2025-12`; ampliar para novas competências requer validar a cobertura e atualizar o catálogo da fonte.
- Esta fonte cobre o Ceará; não representa os cerca de 5.570 municípios brasileiros.
- Os totais do TCE ainda não foram reconciliados com o CSV da Prefeitura de Fortaleza; escopo de unidades e categorias pode divergir.
- Não há transformação nem exposição no painel neste escopo; os bytes permanecem na Raw conforme [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md).
