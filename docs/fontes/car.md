# CAR — Cadastro Ambiental Rural

Fonte: **GeoServer do SICAR** (Serviço Florestal Brasileiro) · Formato: SHAPE-ZIP ·
Atualização: contínua · Licença: dados públicos (SFB/MMA)

## O que é ingerido

Perímetros dos imóveis rurais cadastrados no CAR, por UF:

| UF | Imóveis | UF | Imóveis | UF | Imóveis |
|----|---------|----|---------|----|---------|
| AC | 63.349  | MA | 419.436 | RJ | 69.429  |
| AL | 143.945 | MG | 1.184.602 | RN | 117.963 |
| AM | 99.033  | MS | 90.505  | RO | 198.428 |
| AP | 17.833  | MT | 221.724 | RR | 30.319  |
| BA | 1.333.886 | PA | 392.775 | RS | 680.095 |
| CE | 410.531 | PB | 220.965 | SC | 437.084 |
| DF | 21.045  | PE | 436.075 | SE | 125.160 |
| ES | 130.987 | PI | 337.983 | SP | 474.035 |
| GO | 245.396 | PR | 559.308 | TO | 112.678 |

**Total: ~8,5 milhões de imóveis em 27 tarefas (uma por UF), sob demanda.**

## Sobre a fonte

O [SICAR](https://www.car.gov.br/) (Sistema Nacional de Cadastro Ambiental Rural)
registra a situação ambiental dos imóveis rurais brasileiros: perímetro do imóvel,
reserva legal, APP, uso consolidado e vegetação nativa.

O download é feito via WFS (Web Feature Service) público do GeoServer do SICAR,
sem captcha e sem credenciais. Cada UF é uma layer WFS
(`sicar:sicar_imoveis_{uf}`) servida em SHAPE-ZIP.

### Atributos por imóvel

| Campo | Descrição |
|-------|-----------|
| `cod_imovel` | Código do CAR (ex: `DF-5300108-...`) |
| `status_imovel` | Status no SICAR (ativo, pendente, cancelado) |
| `dat_criacao` | Data de criação do cadastro |
| `data_atualizacao` | Última atualização |
| `area` | Área declarada (hectares) |
| `condicao` | Condição do imóvel |
| `uf` | Unidade Federativa |
| `municipio` | Nome do município |
| `cod_municipio_ibge` | Código IBGE do município |
| `m_fiscal` | Módulo fiscal |
| `tipo_imovel` | Tipo do imóvel rural |

Além dos atributos tabulares, cada feature contém a **geometria** (polígono)
do perímetro do imóvel em SIRGAS 2000 (EPSG:4674).

## Perfil HTTP

- **Host**: `geoserver.car.gov.br`
- **Autenticação**: nenhuma (WFS público)
- **Limite**: 1 requisição/segundo (conservador — UFs grandes são pesadas)
- **Redirecionamento**: seguir

## Padrão de URL

```
https://geoserver.car.gov.br/geoserver/sicar/wfs?service=WFS&version=2.0.0&request=GetFeature&typeNames=sicar:sicar_imoveis_{uf}&outputFormat=SHAPE-ZIP
```

## Comandos

Para baixar todas as UFs:

```bash
docker compose run --rm --build ingestao executar car --sim
```

Para baixar uma UF específica:

```bash
docker compose run --rm --build ingestao executar car --filtro uf=DF
```

## Observações

- **Dado geoespacial**: a Raw guarda o SHAPE-ZIP bruto (vetor + atributos),
  imutável. Processamento geoespacial fica nas camadas seguintes.
- **Volume alto**: BA e MG têm mais de 1 milhão de features cada. O download
  completo de uma UF grande pode levar vários minutos.
- O WFS suporta paginação (`startIndex` + `count`) se for necessário
  parcelar downloads no futuro.
