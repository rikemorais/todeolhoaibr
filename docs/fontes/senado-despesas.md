# Fonte `senado-despesas` — despesas da CEAPS

Despesas da **CEAPS** (Cota para o Exercício da Atividade Parlamentar dos Senadores): "quanto um senador gastou, com quem e quando". É o equivalente da [CEAP da Câmara](camara-despesas.md) para a casa alta. Implementa a [issue #26](https://github.com/rikemorais/todeolhoaibr/issues/26).

## O que é ingerido

- Um **arquivo CSV por ano**, nacional, publicado pela Transparência do Senado em
  `https://www.senado.leg.br/transparencia/LAI/verba/despesa_ceaps_{ano}.csv`, de **2008** até o ano corrente, calculado ao executar. Se o arquivo do ano corrente ainda não foi publicado (HTTP 404), a tarefa termina sem gravar dados e será tentada novamente na próxima execução; um arquivo histórico ausente continua sendo erro.
- O arquivo é baixado como **arquivo grande** (em fluxo, sem carregar na memória) e gravado
  **opaco** na camada Raw — a ingestão não interpreta o conteúdo (ver
  [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).
- O CSV traz, por despesa: valor reembolsado, tipo, fornecedor (nome/CNPJ ou CPF), documento,
  data, mês/ano e o senador. **As colunas e a granularidade diferem da Câmara**, então o mapeamento
  é próprio e fica no refino; a ingestão preserva o arquivo como veio.

## Layout na Raw

Mesmo sendo uma fonte separada no catálogo, os arquivos ficam sob a fonte `senado`:

```text
raw/senado/despesas/ano=AAAA/_ultima_versao.json                                    # ponteiro do vigente
raw/senado/despesas/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.csv
raw/senado/despesas/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

O `_ultima_versao.json` é o **ponteiro** que aponta para o manifesto da execução **vigente** de cada ano: um leitor a jusante começa por ele para achar o snapshot atual, em vez de adivinhar pelo caminho de execução (ver [ADR 0012](../adr/0012-pre-checagem-idempotente.md)).

## Fonte separada, `sob_demanda`

A CEAPS é uma **fonte própria `senado-despesas` com `sob_demanda = true`**, para **não entrar no
`executar --todas`** (são muitos arquivos anuais em lote). Como `sob_demanda` é por fonte, não por
tarefa, separá-la é a única forma de manter as fontes `senado-*` de API leves fora desse peso.
O dado, ainda assim, mora em `raw/senado/despesas/`.

## Como rodar

```bash
# Todos os anos (a fonte é argumento posicional)
todeolho executar senado-despesas

# Só um ano (filtro por etiqueta)
todeolho executar senado-despesas --filtro ano=2024
```

## Idempotência

Reusa a pré-checagem por versão/tamanho ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)): a
origem envia um `ETag` (conferido no arquivo real), que o cliente usa como **validador
autoritativo** — um arquivo revisado ganha ETag novo e é reingerido; um inalterado é reaproveitado.

!!! warning "O CSV costuma embutir a data da última atualização"
    Os arquivos da CEAPS trazem uma linha de cabeçalho com a data de geração. Quando o Senado
    republica os CSVs (inclusive de anos antigos), o conteúdo muda e o `ETag` também, então aquele
    ano é reingerido mesmo sem despesa nova. É o comportamento esperado da origem; a Raw preserva
    cada versão publicada.

## Fora de escopo desta camada (fica no refino)

- **Filtro por UF:** o arquivo é **nacional por ano**; a UF do senador é derivada no refino a
  partir do mapeamento de parlamentares. Como a Raw é opaca, não há arquivo por UF para filtrar na
  ingestão — o recorte por UF é do refino/consulta.
- **Deduplicação por chave composta:** a CEAPS **não tem ID único** por despesa. A deduplicação por
  registro é trabalho do refino (camada `silver`), não da ingestão — ver
  [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md). A idempotência **da ingestão** é por
  versão do arquivo (acima).
