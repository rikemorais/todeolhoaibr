# INPE — Desmatamento e Focos de Calor

Fonte: **inpe** (`FonteInpe`)

Dados de monitoramento de vegetação do INPE (Instituto Nacional de Pesquisas
Espaciais): desmatamento anual (PRODES), alertas de desmatamento quase em
tempo real (DETER) e focos de calor por satélite (Queimadas).

## Produtos

### PRODES — Desmatamento Anual por Corte Raso

Mapeamento anual de desmatamento por corte raso em todos os biomas brasileiros.
O ano PRODES vai de 1.º de agosto a 31 de julho. É a medida oficial de
desmatamento do governo federal.

Download via WFS do GeoServer TerraBrasilis (SHAPE-ZIP).

| Bioma | Workspace | Layer | Cobertura |
|-------|-----------|-------|-----------|
| Amazônia (bioma) | `prodes-amazon-nb` | `yearly_deforestation_biome` | Desde 2008 |
| Amazônia Legal | `prodes-legal-amz` | `yearly_deforestation` | Desde 2008 |
| Cerrado | `prodes-cerrado-nb` | `yearly_deforestation` | Desde ~2018 |
| Caatinga | `prodes-caatinga-nb` | `yearly_deforestation` | Desde 2004 |
| Mata Atlântica | `prodes-mata-atlantica-nb` | `yearly_deforestation` | Desde 2004 |
| Pampa | `prodes-pampa-nb` | `yearly_deforestation` | Desde 2004 |
| Pantanal | `prodes-pantanal-nb` | `yearly_deforestation` | Desde 2004 |

Geometria: Polygon (EPSG:4674 / SIRGAS 2000).

### DETER — Alertas Diários de Desmatamento

Alertas de desmatamento e degradação florestal gerados quase em tempo real
(resolução diária). Dados disponíveis para camada pública anônima.

| Bioma | Workspace | Layer | Filtro |
|-------|-----------|-------|--------|
| Amazônia | `deter-amz` | `deter_amz` | ≥ 0,0625 km² |
| Cerrado | `deter-cerrado-nb` | `deter_cerrado` | ≥ 0,03 km² |

Geometria: Polygon (EPSG:4674 / SIRGAS 2000).

### Queimadas — Focos de Calor (Satélite de Referência)

Focos de calor detectados por sensoriamento remoto, agregados em arquivos
anuais. Utiliza o satélite de referência para garantir comparabilidade temporal
ao longo da série histórica (2003–presente).

| Coluna típica | Descrição |
|---------------|-----------|
| `lat` / `lon` | Coordenadas do foco |
| `data_hora_gmt` | Data e hora UTC da detecção |
| `satelite` | Satélite detector |
| `municipio` | Município |
| `estado` | UF |
| `bioma` | Bioma |
| `dias_sem_chuva` | Dias sem chuva |
| `precipitacao` | Precipitação (mm) |
| `risco_fogo` | Índice de risco de fogo |
| `frp` | Fire Radiative Power (MW) |

Formato: CSV com `;`, encoding UTF-8, dentro de ZIP.

## Recursos

Padrão: `{produto}/bioma={bioma}` para geodados, `{produto}/ano={YYYY}` para CSV.

Exemplos:
- `prodes/bioma=amazonia` — desmatamento anual no bioma Amazônia
- `prodes/bioma=cerrado` — desmatamento anual no Cerrado
- `deter/bioma=amazonia` — alertas de desmatamento na Amazônia
- `queimadas/ano=2024` — focos de calor do satélite de referência em 2024

**Total:** 32 tarefas (7 PRODES + 2 DETER + 23 Queimadas). Sob demanda.

## Formato

- **PRODES / DETER:** Shapefile (SHAPE-ZIP) via WFS. CRS EPSG:4674 (SIRGAS 2000).
- **Queimadas:** CSV dentro de ZIP, delimitador `;`, encoding UTF-8.

## Idempotência

- PRODES: por bioma — re-ingerir substitui o shapefile inteiro daquele bioma.
- DETER: por bioma — idem.
- Queimadas: por ano — cada ano é um recurso distinto.

## Observações

- **Não comparar DETER pré-2015 com pós-2015** (mudança metodológica no sensor
  e na resolução mínima). PRODES é a medida oficial anual.
- Desde 2024, todos os biomas PRODES usam imagens Sentinel-2.
- O GeoServer TerraBrasilis tem limite padrão de 600K features por requisição.
  Se o layer exceder esse limite, o shapefile será truncado — paginação via
  WFS é follow-up.
- DETER Pantanal: o workspace existe mas não tem layers WFS públicas confirmadas.
  Follow-up quando disponível.
- Queimadas com todos os satélites (1998–presente, até 112 MB/ano): follow-up.
- Queimadas mensal e diário: follow-up (dados diários têm retenção de ~30 dias).
- Permite cruzamento com INCRA (parcelas certificadas) e CAR (dimensão fundiária).

## Referências

- [TerraBrasilis — Downloads](https://terrabrasilis.dpi.inpe.br/downloads/)
- [PRODES — BiomasBR](https://data.inpe.br/biomasbr/en/prodes-annual-monitoring-of-native-vegetation-suppression/)
- [DETER — BiomasBR](https://data.inpe.br/biomasbr/en/deter-daily-monitoring-of-native-vegetation-suppression-and-degradation/)
- [Queimadas — Dados Abertos](https://terrabrasilis.dpi.inpe.br/queimadas/portal/dados-abertos/)
- [Data Server INPE — Queimadas](https://dataserver-coids.inpe.br/queimadas/queimadas/)
