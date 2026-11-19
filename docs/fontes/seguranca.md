# Segurança Pública

Fonte: **IPEA (Atlas da Violência) + FBSP (Anuário)** · Formato: JSON
(OData) + XLSX · Atualização: anual · Dados públicos

## O que é ingerido

### Atlas da Violência (IPEA/Ipeadata)

11 séries do Atlas da Violência via API OData do Ipeadata, cobrindo
estados, regiões e municípios, com série anual desde 1979:

| Série | Código Ipeadata | Descrição |
|-------|----------------|-----------|
| homicidios | `AVIOL12_HOMIC` | Número de homicídios |
| homicidios-feminino | `AVIOL12_HOMICF` | Número de homicídios do sexo feminino |
| homicidios-jovens | `AVIOL12_HOMICJ` | Número de homicídios de jovens de 15 a 29 anos |
| homicidios-masculino | `AVIOL12_HOMICM` | Número de homicídios do sexo masculino |
| taxa-homicidios | `AVIOL12_THOMIC` | Taxa de homicídios (100.000 hab.) |
| suicidios | `AVIOL12_SUICID` | Número de suicídios |
| taxa-suicidios | `AVIOL12_TSUICID` | Taxa de suicídios (100.000 hab.) |
| acidentes-transito | `AVIOL12_ACIDT` | Vítimas fatais em acidente de trânsito |
| acidentes-transito-jovens | `AVIOL12_ACIDTJ` | Vítimas fatais em acidente de trânsito (jovens 15–29) |
| taxa-acidentes-transito | `AVIOL12_TACIDT` | Taxa de acidentes de trânsito fatais (100.000 hab.) |
| populacao | `AVIOL12_VIOLPOP` | População residente — Estimativa/TCU (ajustada) |

Fonte primária: DATASUS (SIM — Sistema de Informações sobre Mortalidade)
e IPEA.

### Anuário Brasileiro de Segurança Pública (FBSP)

5 edições do Anuário em XLSX, publicadas anualmente pelo Fórum
Brasileiro de Segurança Pública:

| Edição | Ano de publicação | Dados de referência |
|--------|-------------------|---------------------|
| 16ª | 2022 | 2021 |
| 17ª | 2023 | 2022 |
| 18ª | 2024 | 2023 |
| 19ª | 2025 | 2024 |
| 20ª | 2026 | 2025 |

## Sobre a fonte

O [Atlas da Violência](https://www.ipea.gov.br/atlasviolencia/) é uma
publicação conjunta do IPEA e do FBSP que consolida estatísticas de
mortes violentas no Brasil. Os microdados vêm do SIM/DATASUS e são
disponibilizados no [Ipeadata](http://www.ipeadata.gov.br) como séries
OData.

O [Anuário Brasileiro de Segurança Pública](https://forumseguranca.org.br/anuario-brasileiro-de-seguranca-publica/)
é a principal publicação do FBSP, com indicadores de ocorrências
criminais, efetivo policial, sistema penitenciário, armas e letalidade
policial, coletados junto às secretarias de segurança estaduais.

## Perfil HTTP

- **Host**: `www.ipeadata.gov.br` (séries OData)
- **Autenticação**: nenhuma
- **Limite**: 2 requisições/segundo no Ipeadata
- **Redirecionamento**: seguir

Os downloads do FBSP (`forumseguranca.org.br`) não possuem restrição
especial.

## Padrão de URL

Séries OData:
```
https://www.ipeadata.gov.br/api/odata4/Metadados('{SERCODIGO}')/Valores
```

Anuários FBSP:
```
https://forumseguranca.org.br/wp-content/uploads/{YYYY}/{MM}/anuario-{YYYY}.xlsx
```

## Comandos

Para baixar tudo:

```bash
docker compose run --rm --build ingestao executar seguranca --sim
```

Para baixar só as séries do IPEA:

```bash
docker compose run --rm --build ingestao executar seguranca --filtro origem=ipea
```

Para baixar só os anuários do FBSP:

```bash
docker compose run --rm --build ingestao executar seguranca --filtro origem=fbsp
```

## Observações

- **Sob demanda**: as séries OData são pesadas (~50 MB cada, 445k+
  valores por série).
- **Cobertura temporal**: as séries AVIOL cobrem desde 1979 (varia
  por série). Os anuários FBSP cobrem 2022–2026.
- **Nível territorial**: as séries incluem dados por Estado, Região
  e possivelmente Município.
- **Fonte primária**: SIM/DATASUS (mortalidade) e secretarias de
  segurança estaduais (ocorrências).
