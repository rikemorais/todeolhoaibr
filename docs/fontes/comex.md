# Comex Stat (MDIC) — Comércio Exterior

Fonte: **comex** (`FonteComex`)

Dados de exportação e importação do Brasil publicados pelo Ministério do
Desenvolvimento, Indústria, Comércio e Serviços (MDIC) via Comex Stat.
Série mensal desde 1997, com dois níveis de granularidade.

## Datasets

### NCM (Nomenclatura Comum do Mercosul)

Detalhe máximo por produto (8 dígitos), com UF de origem, via de transporte
e unidade aduaneira.

| Coluna | Descrição |
|--------|-----------|
| `CO_ANO` | Ano |
| `CO_MES` | Mês (01–12) |
| `CO_NCM` | Código NCM (8 dígitos) |
| `CO_UNID` | Unidade estatística |
| `CO_PAIS` | Código do país |
| `SG_UF_NCM` | UF de origem (exp) ou destino (imp) |
| `CO_VIA` | Via de transporte |
| `CO_URF` | Unidade da Receita Federal |
| `QT_ESTAT` | Quantidade estatística |
| `KG_LIQUIDO` | Peso líquido (kg) |
| `VL_FOB` | Valor FOB (US$) |
| `VL_FRETE` | Frete (US$, só importação) |
| `VL_SEGURO` | Seguro (US$, só importação) |

### MUN (Município / SH4)

Detalhe por município do declarante, produto a nível SH4 (4 dígitos).

| Coluna | Descrição |
|--------|-----------|
| `CO_ANO` | Ano |
| `CO_MES` | Mês (01–12) |
| `SH4` | Código do Sistema Harmonizado (4 dígitos) |
| `CO_PAIS` | Código do país |
| `SG_UF_MUN` | UF do declarante |
| `CO_MUN` | Código IBGE do município (7 dígitos) |
| `KG_LIQUIDO` | Peso líquido (kg) |
| `VL_FOB` | Valor FOB (US$) |

## Recursos

Padrão: `{tipo}/fluxo={exp|imp}/ano={YYYY}`

Exemplos:
- `ncm/fluxo=exp/ano=2024` — exportações NCM de 2024
- `mun/fluxo=imp/ano=2023` — importações por município de 2023

**Total:** 120 tarefas (30 anos × 2 fluxos × 2 tipos). Sob demanda.

## Formato

CSV com delimitador `;`, valores entre aspas, encoding Latin-1 (Windows-1252).
Arquivos grandes (dezenas a centenas de MB por ano).

## Bronze

Decodificador: `DecodificadorCsv::novo_windows1252` — reutilizado, sem decoder
customizado. Dois registros no catálogo de refino:

| Entrada | Prefixo | Tabela Bronze |
|---------|---------|---------------|
| `comex-ncm` | `ncm/` | `comercio` |
| `comex-mun` | `mun/` | `comercio_mun` |

## Observações

- O certificado TLS de `balanca.mdic.gov.br` usa CA governamental (SERPRO) fora
  dos bundles padrão. A fonte ativa `aceitar_certificado_invalido` no perfil HTTP.
- Tabelas auxiliares (PAIS.csv, NCM.csv, UF_MUN.csv, etc.) ficam como follow-up
  para enriquecimento Silver.
- Permite ligar produção agrícola → exportação por região via join com CONAB.

## Referências

- [Comex Stat — Dados Abertos](https://www.gov.br/mdic/pt-br/assuntos/comercio-exterior/estatisticas/base-de-dados-bruta)
- [API Comex Stat](https://api-comexstat.mdic.gov.br/docs)
