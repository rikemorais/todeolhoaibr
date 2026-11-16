# INCRA — Imóveis Rurais e Assentamentos

Fonte: **incra** (`FonteIncra`)

Dados de certificação de imóveis rurais (SIGEF/SNCI) e projetos de
assentamento da reforma agrária, publicados pelo INCRA (Instituto Nacional
de Colonização e Reforma Agrária).

## Camadas

### SIGEF — Parcelas Certificadas (pós-2013)

Shapefiles por UF com parcelas certificadas no Sistema de Gestão Fundiária.
Duas naturezas: público (imóveis de propriedade pública) e privado.

| Campo | Descrição |
|-------|-----------|
| `codigo_parcela` | Identificador único da parcela |
| `rt` | Responsável técnico |
| `art` | Anotação de responsabilidade técnica |
| `situacao` | Situação da certificação |
| `codigo_imovel` | Código do imóvel no SNCR |
| `data_submissao` | Data de submissão ao SIGEF |
| `data_aprovacao` | Data de aprovação |
| `status` | Status da parcela |
| `nome_area` | Nome da área |
| `registro_matricula` | Matrícula no registro de imóveis |
| `registro_data` | Data do registro |
| `cod_municipio` | Código IBGE do município |
| `uf` | UF |
| `natureza` | Público ou privado |
| `geometry` | Polygon Z (EPSG:4674 / SIRGAS 2000) |

### SNCI — Certificações Anteriores

Parcelas certificadas no sistema anterior ao SIGEF (registros até ~2016).
Publicadas em 24 das 27 UFs (sem AC, DF, RR).

| Campo | Descrição |
|-------|-----------|
| `num_processo` | Número do processo |
| `sr` | Superintendência regional |
| `num_certificacao` | Número da certificação |
| `data_certificacao` | Data da certificação |
| `area_peca_tecnica` | Área da peça técnica |
| `cod_profissional` | Código do profissional |
| `cod_imovel_rural` | Código do imóvel rural |
| `nome_imovel` | Nome do imóvel |
| `uf` | UF |
| `geometry` | Polygon (EPSG:4674 / SIRGAS 2000) |

### Assentamentos — Geoespacial

Shapefile nacional com os perímetros dos projetos de assentamento.

| Campo | Descrição |
|-------|-----------|
| `codigo_sipra` | Código SIPRA do projeto |
| `nome_projeto` | Nome do projeto de assentamento |
| `municipio` | Município |
| `uf` | UF |
| `area_ha` | Área em hectares |
| `capacidade` | Capacidade de famílias |
| `num_familias` | Número de famílias assentadas |
| `fase` | Fase de implementação |
| `data_criacao` | Data de criação do projeto |
| `forma_obtencao` | Forma de obtenção (desapropriação, compra, etc.) |
| `data_obtencao` | Data de obtenção |
| `geometry` | Polygon (EPSG:4674 / SIRGAS 2000) |

### Assentamentos — CSV Tabular (SIPRA)

Lista de projetos de assentamento publicada pelo INCRA em CSV. Nacional, ~10 mil
projetos. Fonte: SIPRA (Sistema de Informações de Projetos de Reforma Agrária).

| Coluna | Descrição |
|--------|-----------|
| `codigo_projeto` | Código SIPRA (ex.: PA0006000) |
| `nome_projeto` | Nome do projeto de assentamento |
| `municipio_sede` | Município sede |
| `area_ha` | Área em hectares |
| `familias_assentadas` | Famílias assentadas |
| `familias_assent` | Famílias assentadas (capacidade) |
| `fase` | Fase de implementação (código) |
| `tipo` | Tipo de projeto |
| `numero` | Número do projeto |
| `data_criacao` | Data de criação |
| `forma_obtencao` | Forma de obtenção |
| `data_obtencao` | Data de obtenção |

## Recursos

Padrão: `{camada}/uf={UF}` para shapefiles por estado.

Exemplos:
- `sigef-publico/uf=SP` — parcelas certificadas públicas de São Paulo
- `sigef-privado/uf=MG` — parcelas certificadas privadas de Minas Gerais
- `snci/uf=BA` — certificações SNCI da Bahia
- `assentamentos-geo` — shapefile nacional de assentamentos
- `assentamentos-csv` — CSV tabular de projetos de assentamento

**Total:** 80 tarefas (27 SIGEF público + 27 SIGEF privado + 24 SNCI + 1 geo + 1 CSV). Sob demanda.

## Formato

- **Shapefiles:** ZIP contendo .shp, .dbf, .shx, .prj. CRS EPSG:4674 (SIRGAS 2000).
  Encoding DBF: Latin-1 (cp1252). Geometria: SIGEF → Polygon Z; SNCI/Assentamentos → Polygon.
- **CSV tabular:** delimitador `;`, encoding Latin-1 (Windows-1252), com linhas de metadados
  intercaladas que são filtradas pelo decodificador.

## Bronze

Decodificador apenas para o CSV tabular: `DecodificadorAssentamentosIncra` — filtra
linhas de cabeçalho ministerial, separadores de Superintendência Regional e totalizador.

| Entrada | Prefixo | Tabela Bronze |
|---------|---------|---------------|
| `incra-assentamentos` | `assentamentos-csv/` | `assentamentos` |

Shapefiles: ficam na Raw com manifesto, sem decodificação Bronze nesta versão.
Decodificador de shapefile (extração do .dbf) é follow-up.

## Idempotência

- Shapefiles: por UF — cada UF é um recurso distinto. Re-ingerir substitui o ZIP inteiro.
- CSV de assentamentos: recurso único nacional. Dentro do CSV, cada projeto é único
  por `codigo_projeto`.

## Observações

- O endpoint de shapefiles (`certificacao.incra.gov.br`) tem indisponibilidade intermitente.
  Todas as tarefas de shapefile aceitam ausência (`aceitar_ausencia`).
- O certificado TLS pode usar CA governamental (SERPRO). A fonte ativa
  `aceitar_certificado_invalido` no perfil HTTP.
- O SNCI não tem cobertura em AC, DF e RR (sem arquivo publicado).
- Tabelas auxiliares de códigos e cruzamento com CAR são follow-up.
- Permite cruzamento com CONAB (produção agrícola por município) e CAR (dimensão fundiária).

## Referências

- [Acervo Fundiário — dados.gov.br](https://dados.gov.br/dados/conjuntos-dados/acervo-fundiario)
- [SIGEF — dados.gov.br](https://dados.gov.br/dados/conjuntos-dados/sistema-de-gestao-fundiaria---sigef)
- [Relação de Projetos de Assentamento](https://www.gov.br/incra/pt-br/assuntos/reforma-agraria/assentamentos-relacao-de-projetos)
- [SIGEF — Consulta Pública](https://sigef.incra.gov.br/consultar/parcelas/)
