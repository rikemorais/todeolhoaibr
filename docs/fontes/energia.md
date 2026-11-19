# Energia

Fonte: **ANEEL + EPE + ONS** · Formato: CSV, ZIP, XLSX · Atualização:
variável · Dados públicos

## O que é ingerido

### ANEEL (3 recursos)

| Recurso | Formato | Descrição |
|---------|---------|-----------|
| `tarifas` | CSV (~83 MB) | Tarifas homologadas das distribuidoras (TE e TUSD), com classe, subgrupo e modalidade tarifária |
| `siga-empreendimentos` | CSV (~8 MB) | Empreendimentos de geração de energia do SIGA (tipo, potência, fonte, localização) |
| `geracao-distribuida` | ZIP (~113 MB) | Micro e minigeração distribuída — unidades consumidoras com geração própria |

### EPE (6 recursos)

| Recurso | Formato | Descrição |
|---------|---------|-----------|
| `consumo-mensal-por-classe` | XLSX (~1,6 MB) | Consumo mensal de energia elétrica por classe (residencial, industrial, comercial, etc.), região e subsistema, desde 2004 |
| `ben-dados-agregados` | XLSX | BEN Cap. 1 — Análise Energética e Dados Agregados (1970–2025) |
| `ben-oferta-demanda` | XLSX | BEN Cap. 2 — Oferta e Demanda de Energia por Fonte (1970–2025) |
| `ben-consumo-por-setor` | XLSX | BEN Cap. 3 — Consumo de Energia por Setor (1970–2025) |
| `ben-dados-estaduais` | XLSX | BEN Cap. 8 — Dados Estaduais (2000–2025) |
| `ben-capacidade-instalada` | XLSX | BEN Anexo I — Capacidade Instalada |

### ONS (27 recursos)

Balanço de energia nos subsistemas (horário), um CSV por ano de 2000 a 2026.

Campos: `id_subsistema`, `nom_subsistema`, `din_instante`,
`val_gerhidraulica`, `val_gertermica`, `val_gereolica`, `val_gersolar`,
`val_carga`, `val_intercambio`.

Subsistemas: Norte, Nordeste, Sudeste/Centro-Oeste, Sul.

## Sobre as fontes

- **[ANEEL](https://dadosabertos.aneel.gov.br/)**: Agência Nacional de
  Energia Elétrica. Portal de dados abertos com 72+ datasets em CSV/XML/Parquet.
- **[EPE](https://www.epe.gov.br/)**: Empresa de Pesquisa Energética.
  Publica o Balanço Energético Nacional (BEN) e estatísticas de consumo.
- **[ONS](https://dados.ons.org.br/)**: Operador Nacional do Sistema
  Elétrico. Portal de dados abertos com 88+ datasets (carga, geração,
  hidrologia) em CSV/XLSX/Parquet no S3.

## Perfil HTTP

- **Host com limite**: `dadosabertos.aneel.gov.br` — 2 req/s
- **Autenticação**: nenhuma (todos os dados são públicos)
- **Redirecionamento**: seguir

Os downloads do ONS (S3 da AWS) e da EPE não possuem restrição especial.

## Padrão de URL

ANEEL (CKAN):
```
https://dadosabertos.aneel.gov.br/dataset/{dataset-id}/resource/{resource-id}/download/{filename}
```

EPE:
```
https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/Documents/{filename}
https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/PublicacoesArquivos/publicacao-145/topico-515/{filename}
```

ONS:
```
https://ons-aws-prod-opendata.s3.amazonaws.com/dataset/balanco_energia_subsistema_ho/BALANCO_ENERGIA_SUBSISTEMA_{YYYY}.csv
```

## Comandos

Para baixar tudo:

```bash
docker compose run --rm --build ingestao executar energia --sim
```

Para baixar só ANEEL:

```bash
docker compose run --rm --build ingestao executar energia --filtro orgao=aneel
```

Para baixar só ONS:

```bash
docker compose run --rm --build ingestao executar energia --filtro orgao=ons
```

Para baixar só EPE:

```bash
docker compose run --rm --build ingestao executar energia --filtro orgao=epe
```

## Observações

- **Sob demanda**: os downloads são pesados (tarifas ~83 MB, geração
  distribuída ~113 MB, ONS 27 anos).
- **Granularidade**: ANEEL por distribuidora/empreendimento; EPE por
  UF/classe/setor; ONS por subsistema/hora.
- **Idempotência**: cada recurso é baixado como arquivo único; a
  pré-checagem reaproveita o que já existe na Raw.
