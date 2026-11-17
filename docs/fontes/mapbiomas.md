# MapBiomas — Uso e Cobertura do Solo

Fonte: [MapBiomas Brasil](https://brasil.mapbiomas.org/) (consórcio multi-institucional, dado aberto).

Mapeamento anual de uso e cobertura do solo do Brasil desde 1985, com
resolução de 30 m (Landsat). Os dados são publicados em **coleções**; a cada
nova coleção toda a série histórica é revisada com metodologia atualizada.

**Coleção ingerida: 11** (publicada em 2026, cobre 1985–2025).

## Produtos

### Cobertura e Uso do Solo

Área (ha) por classe de cobertura, bioma e estado.

| Recorte | Arquivo |
|---------|---------|
| Bioma + Estado | `MAPBIOMAS_BRAZIL-COL.11-BIOME_STATE.xlsx` |

Cobertura por município está disponível apenas via Google Drive (link
instável para automação) — follow-up quando migrar para URL direta.

### Desmatamento Anual

Perda anual de vegetação nativa por classe de uso substituto, com
detalhamento por município.

| Recorte | Arquivo |
|---------|---------|
| Município | `MAPBIOMAS-BRAZIL-DEFORESTATION-ANNUAL-MUNICIPALITY-COLLECTION_11.xlsx` |

### Vegetação Secundária

Área de regeneração de vegetação nativa por município.

| Recorte | Arquivo |
|---------|---------|
| Município | `MAPBIOMAS-BRAZIL-SECONDARY-VEGETATION-MUNICIPALITY-COLLECTION_11.xlsx` |

## Formato

Todos os arquivos são XLSX (Microsoft Excel). Cada planilha contém colunas
por ano (1985–2025) e linhas por unidade territorial (bioma, estado ou
município, conforme o recorte).

## Idempotência

Chave: `coleção + produto + recorte`. Cada arquivo contém a série completa
de anos; re-ingerir substitui o snapshot inteiro daquela combinação.

## Procedência

Os dados são produzidos pelo consórcio MapBiomas (SEEG, IPAM, IMAZON,
entre outros) a partir de classificação supervisionada de imagens Landsat
no Google Earth Engine. A coleção é identificada no nome do arquivo e nos
metadados do manifesto.

**Importante**: ao comparar séries entre coleções, usar sempre a mesma
coleção — a reclassificação pode alterar valores históricos.

## Follow-up

- Cobertura por município (Google Drive → aguardar URL direta)
- Idade da vegetação secundária (idem)
- Irrigação, mineração, agricultura (produtos tangenciais)
- MapBiomas Fogo (coleção separada)
- MapBiomas Água (coleção separada)
- MapBiomas Pastagem (coleção separada)
- Raster GeoTIFF 30 m (> 1 GB/ano, requer ingestão dedicada)
- Recortes territoriais (AMACRO, Amazônia Legal, Terras Indígenas, etc.)

## Referências

- Plataforma de estatísticas: <https://brasil.mapbiomas.org/en/estatisticas/>
- Repositório Dataverse: <https://data.mapbiomas.org/>
- ATBD Collection 9: <https://brasil.mapbiomas.org/wp-content/uploads/sites/4/2025/02/ATBD-Collection-9-versao2-v2.pdf>
