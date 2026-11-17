# IBAMA — Fiscalização Ambiental

Fonte: **dadosabertos.ibama.gov.br** · Formato: CSV · Atualização: diária ·
Licença: dados abertos ([Lei de Acesso à Informação](https://www.gov.br/acessoainformacao))

## O que é ingerido

Dois datasets do sistema de fiscalização ambiental do IBAMA:

### Autos de infração (8 tabelas)

| Tabela | Descrição | Tamanho |
|--------|-----------|--------:|
| `autos` | Atributos principais de cada auto de infração | 118 MB |
| `enquadramento` | Dispositivos legais que fundamentam o auto (SIFISC) | 8 MB |
| `aie-enquadramento` | Dispositivos legais — sistema AIe/AIe Mob | 33 MB |
| `aie-enquadramento-complementar` | Dispositivos complementares — AIe/AIe Mob | 2 MB |
| `especimes` | Organismos envolvidos na infração | 5 MB |
| `coordenadas` | Ponto geográfico de referência da infração | 9 MB |
| `biomas` | Bioma brasileiro onde a infração ocorreu | 14 MB |
| `anexos` | Metadados de documentos, mapas e fotos anexados | 19 MB |

### Termos de embargo (8 tabelas)

| Tabela | Descrição | Tamanho |
|--------|-----------|--------:|
| `termos` | Atributos principais de cada termo de embargo | 204 MB |
| `itens` | Itens embargados (tipo e objeto do embargo) | 5 MB |
| `coordenadas` | Coordenadas do local ou vértices da área embargada | 19 MB |
| `anexos` | Metadados de documentos anexados | 2 MB |
| `decisoes` | Decisões judiciais que suspendem ou restabelecem embargos | 0,1 MB |
| `enquadramento` | Dispositivos legais que fundamentam a medida | 11 MB |
| `enquadramento-complementar` | Dispositivos legais complementares | 2 MB |
| `historico` | Histórico de alterações dos atributos dos termos | 702 MB |

**Total: 16 tabelas, ~1,1 GB (CSV comprimido e aberto).**

## Sistemas de origem

Os autos de infração vêm de dois sistemas:

- **SIFISC** (série numérica): autos lavrados até 07/10/2019.
- **AIe / AIe Mob** (série alfanumérica): autos eletrônicos a partir de 08/10/2019.

Ambos são publicados no mesmo dataset, mas as tabelas de enquadramento são
separadas: SIFISC usa `enquadramento` e AIe usa `aie-enquadramento`.

## Procedência

Todos os arquivos estão hospedados no Azure Blob Storage do IBAMA:

```
https://stibamadadosabertosprd.blob.core.windows.net/dados-abertos/dados/
├── SIFISC/auto_infracao/      ← autos (SIFISC)
├── AIE_MOB/auto_infracao/     ← autos (AIe)
└── TERMOS_DE_EMBARGO/         ← embargos
```

Não há autenticação. As URLs são estáveis e os arquivos são dumps completos
atualizados diariamente.

## Idempotência

Cada arquivo é um snapshot completo. Re-ingerir substitui o snapshot anterior
no Raw. A idempotência por número do auto ou termo de embargo (chave de
negócio) é responsabilidade da camada Silver.

## Filtros

Os CSVs não suportam filtros na URL. Para filtrar por UF, período ou tipo de
infração, use as colunas correspondentes após a ingestão.

## Follow-up

- **Julgamentos de auto de infração** — dataset separado no portal IBAMA.
- **Outros termos** — apreensão, suspensão, destruição, doação, demolição.
- **Geometrias de áreas embargadas** — SHP via GeoServer do IBAMA.
- **Decodificador Bronze** — decodificar CSV em Arrow para consultas Bronze.
- **Silver** — chave de negócio por número do auto/termo, deduplicação e tipagem.
