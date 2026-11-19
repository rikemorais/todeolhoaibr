# GFW — Global Forest Watch

Fonte: **Our World in Data** (mirror do Global Forest Watch) · Formato: CSV ·
Atualização: anual · Licença: CC BY 4.0 (OWID) / dados abertos (GFW/WRI)

## O que é ingerido

Dois datasets consolidados de desmatamento global por país:

| Dataset | Cobertura | Período | Formato | Tarefas |
|---------|-----------|---------|---------|---------|
| **Tree Cover Loss** | todos os países | 2001–2024 | CSV | 1 |
| **Tree Cover Loss from Wildfires** | todos os países | 2001–2024 | CSV | 1 |

**Total: 2 tarefas, sob demanda.**

## Sobre a fonte

O [Global Forest Watch](https://www.globalforestwatch.org/) publica dados de
perda de cobertura florestal baseados em sensoriamento remoto (Landsat).
O dataset principal usa a metodologia de Hansen et al. (2013), com
classificação de drivers por Sims et al. (2025).

Os CSVs são baixados do mirror do [Our World in Data](https://ourworldindata.org/),
que processa e consolida os dados do GFW em formato tabular simples (país, ano,
hectares perdidos, driver de desmatamento).

### Datasets

- **Tree Cover Loss**: perda total de cobertura florestal por país, ano e
  driver (agricultura permanente, commodity, mineração, urbanização, incêndio,
  silvicultura, deslocamento de agricultura). Fonte: Hansen/UMD + WRI.

- **Tree Cover Loss from Wildfires**: subconjunto da perda causada por
  incêndios florestais. Separa a perda "natural" da antrópica.

## Perfil HTTP

- **Host**: `ourworldindata.org`
- **Autenticação**: nenhuma (acesso público)
- **Limite**: 2 requisições/segundo
- **Redirecionamento**: seguir

## Padrão de URL

```
https://ourworldindata.org/grapher/{dataset}.csv?v=1&csvType=full&useColumnShortNames=false
```

## Roadmap

Numa segunda iteração, esta fonte pode ser expandida com:

- **NASA FIRMS** (focos de incêndio NRT): requer MAP_KEY gratuito, endpoint
  `/api/country/csv/{MAP_KEY}/{source}/{country_code}/{day_range}`.
- **GFW Data API** (datasets adicionais): carbono, alertas integrados, VIIRS
  fire alerts — requer API key e queries SQL via POST.
- **Earthdata Login** (arquivos históricos FIRMS): MODIS e VIIRS completos.
