# INCRA — SIGEF e Assentamentos

Fonte: **incra** (`FonteIncra`)

Ingere parcelas georreferenciadas do SIGEF pelo Geoportal público do INCRA e a
relação tabular de projetos de assentamento publicada pelo INCRA.

## SIGEF — Geoportal INCRA

O Geoportal publica a camada `geonode:sigef_geo` por WFS. A ingestão filtra a
camada por código IBGE da UF e pelo campo `natureza`, baixando um Shapefile ZIP
por categoria e estado, sem autenticação.

| Recurso Raw | Filtro `natureza` |
|-------------|-------------------|
| `sigef-publico/uf={UF}` | `Gleba Pública` |
| `sigef-privado/uf={UF}` | `Particular` |
| `sigef-assentamento/uf={UF}` | `Assentamento Parcela` |

Os arquivos contêm, entre outros campos, `parcela_codigo`, `natureza`,
`codigo_imovel`, `municipio_id`, `uf_id`, `area_hectares`, `status` e
`geometry` (EPSG:4674 / SIRGAS 2000). A categoria `Assentamento Parcela` contém
parcelas SIGEF classificadas como assentamento; ela não representa os perímetros
dos projetos de assentamento.

## Assentamentos — CSV tabular (SIPRA)

Lista nacional de projetos de assentamento, publicada pelo INCRA em CSV,
separada por `;` e codificada em Windows-1252. O decodificador
`DecodificadorAssentamentosIncra` filtra cabeçalhos ministeriais, separadores de
Superintendência Regional e totalizadores.

| Recurso Raw | Tabela Bronze |
|-------------|---------------|
| `assentamentos-csv/projetos` | `assentamentos` |

Campos documentados no arquivo de origem incluem `codigo_projeto`,
`nome_projeto`, `municipio_sede`, `area_ha`, `familias_assentadas`, `fase`,
`tipo`, `numero`, `data_criacao`, `forma_obtencao` e `data_obtencao`. A chave
de negócio dos projetos é `codigo_projeto`.

## Catálogo

**Total:** 82 tarefas: 27 UFs para cada uma das três categorias SIGEF e uma
tarefa nacional do CSV. A fonte é sob demanda.

## Disponibilidade das camadas legadas

O host antigo `certificacao.incra.gov.br` não resolve em DNS. O serviço oficial
de download do Acervo Fundiário agora exige conta gov.br, portanto a ingestão
automática sem credenciais não pode obter os Shapefiles legados de SNCI nem os
perímetros geográficos de assentamentos. Essas tarefas saíram do catálogo; os
objetos que já tenham sido gravados na Raw não são apagados e deixam de receber
atualizações automáticas. O CSV tabular de assentamentos continua disponível
publicamente.

## Referências

- [Geoportal INCRA](https://geoportal.incra.gov.br/)
- [Serviços WFS do Geoportal](https://geoportal.incra.gov.br/developer/)
- [SIGEF — Governo Digital](https://www.gov.br/governodigital/pt-br/infraestrutura-nacional-de-dados/territoriais/bases-e-sistemas/sigef/sigef/)
- [Serviço oficial de download do Acervo Fundiário](https://www.gov.br/pt-br/servicos/obter-coordenadas-e-baixar-os-arquivos-dos-imoveis-rurais-certificados)
- [Relação de Projetos de Assentamento](https://www.gov.br/incra/pt-br/assuntos/reforma-agraria/assentamentos-relacao-de-projetos)
