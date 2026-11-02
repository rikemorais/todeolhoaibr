# Fonte `tse` — votos por seção eleitoral

Resultado eleitoral no maior detalhe público: **votos por seção**, de **1998 a 2026**.
Cada linha traz UF, município, zona, seção, cargo, candidato e votos — um único conjunto
atende às consultas por seção, cidade e estado.

Documentação: <https://dadosabertos.tse.jus.br/dataset/?tags=Resultados>.

## O que é ingerido

Um **ZIP por UF e por eleição** (anos pares de 1998 a 2026), baixado como **arquivo
grande** (em fluxo, com [download retomável](camara-votacoes.md)) e gravado **opaco** na
Raw ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)):

- `https://cdn.tse.jus.br/.../votacao_secao_{ano}_{uf}.zip`
- **408 arquivos** no total. Eleições **gerais** (2002, 2006, 2010…) incluem `DF` e `BR`;
  **municipais** (2000, 2004…) não têm `DF`. Em 2010 há também `VT` e `ZZ` (exterior).

## Layout na Raw

```text
raw/tse/votacao_secao/ano=AAAA/uf=UF/_ultima_versao.json
raw/tse/votacao_secao/ano=AAAA/uf=UF/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.zip
raw/tse/votacao_secao/ano=AAAA/uf=UF/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

## Fonte `sob_demanda`

São ~**16 GB** no total, então é `sob_demanda = true`: **não entra no `executar --todas`**.
O menu mostra o tamanho por arquivo; filtre por `ano`/`uf`.

## Como rodar

```bash
# Uma UF de uma eleição
docker compose run --rm --build ingestao executar tse --filtro ano=2024 --filtro uf=AC

# Várias eleições e UFs
docker compose run --rm --build ingestao executar tse --filtro ano=2022,2024 --filtro uf=AC,SP
```

## Idempotência

Pré-checagem por versão/tamanho ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)); uma
carga longa interrompida continua de onde parou (ver o
[download retomável](camara-votacoes.md)).

## Fora de escopo desta camada (fica no refino)

- Os arquivos usam codificação **Latin-1** e **`;`** como separador; descompactados, chegam
  a ~8× o tamanho do ZIP. Decodificar, descompactar e consultar por seção/cidade/estado é do
  refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)) — a Raw guarda o ZIP opaco.
- Para 1994 e 1996 o TSE não publica votos por seção.
