# FAO — FAOSTAT

Fonte: **Food and Agriculture Organization (FAO)** · Formato: ZIP (CSV) ·
Atualização: trimestral/anual · Licença: CC BY-NC-SA 3.0 IGO

## O que é ingerido

| Recurso | Domínio FAOSTAT | Descrição |
|---------|-----------------|-----------|
| `producao` | Production — Crops, Livestock | Produção agrícola e pecuária por país e ano |
| `valor-producao` | Value of Production | Valor bruto/líquido da produção por país |
| `indices-producao` | Production Indices | Índices de produção (base 2014–2016) |
| `comercio` | Trade — Crops, Livestock | Comércio internacional agrícola por país |
| `balanco-alimentar` | Food Balance Sheets | Balanço alimentar por país (a partir de 2010) |
| `balanco-alimentar-historico` | Food Balance Sheets (Historic) | Balanço alimentar (séries antes de 2010) |
| `uso-do-solo` | Inputs — Land Use | Uso do solo agrícola por país |
| `fertilizantes` | Inputs — Fertilizers (Nutrient) | Consumo de fertilizantes por nutriente |
| `precos` | Prices | Preços de produtor agrícola por país |
| `emissoes` | Emissions Totals | Emissões agrícolas totais por país |
| `populacao` | Population | População anual por país |

Cada ZIP contém um CSV com todos os países e anos do domínio, preservado
opaco na Raw.

## Sobre a fonte

O [FAOSTAT](https://www.fao.org/faostat/) é o banco de dados estatísticos da
FAO/ONU que cobre alimentação e agricultura em mais de 245 países desde 1961.
Os bulk downloads em ZIP/CSV oferecem acesso direto a datasets completos sem
paginação.

## Perfil HTTP

- **Host com limite**: `fenixservices.fao.org` — 1 req/2s
- **Autenticação**: nenhuma
- **Redirecionamento**: seguir (301 → `bulks-faostat.fao.org`)

## Padrão de URL

Bulk download (novembro de 2026):
```
https://fenixservices.fao.org/faostat/static/bulkdownloads/{Nome_Do_Dominio}_E_All_Data.zip
```

O servidor redireciona para:
```
https://bulks-faostat.fao.org/{categoria}/{Nome_Do_Dominio}_E_All_Data.zip
```

## Comandos

```bash
# Todos os domínios
docker compose run --rm --build ingestao executar fao --sim

# Apenas produção
docker compose run --rm --build ingestao executar fao dominio=producao --sim

# Apenas comércio
docker compose run --rm --build ingestao executar fao dominio=comercio --sim
```

## Observações

- **Sob demanda**: os ZIPs somam ~230 MB no total. O maior é `comercio` (~93 MB).
- **Sem API de metadados**: os URLs são estáticos; não há consulta dinâmica ao catálogo.
- **Domínios indisponíveis**: `Trade_LiveAnimals` e `Emissions_Agriculture` retornam HTTP 403 (nov/2026).
