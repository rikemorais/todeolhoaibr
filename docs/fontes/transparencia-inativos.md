# Portal da Transparência — Inativos, Pensionistas e Reserva/Reforma

Fonte: **CGU / Portal da Transparência** · Formato: ZIP (CSV) ·
Atualização: mensal · Dados públicos (Lei de Acesso)

## O que é ingerido

Planilhas mensais de 6 categorias do Portal da Transparência:

| Tipo | Arquivo no Portal | Categoria |
|------|-------------------|-----------|
| Aposentados (SIAPE) | `Aposentados_SIAPE` | aposentados |
| Aposentados (BACEN) | `Aposentados_BACEN` | aposentados |
| Pensionistas civis (SIAPE) | `Pensionistas_SIAPE` | pensionistas-civis |
| Pensionistas civis (BACEN) | `Pensionistas_BACEN` | pensionistas-civis |
| Pensionistas militares (Defesa) | `Pensionistas_DEFESA` | pensionistas-defesa |
| Reserva e reforma militar | `Reserva_Reforma_Militares` | reserva-reforma |

**Série: jan/2020 em diante, sob demanda.**

Cada ZIP mensal contém 3 CSVs: `{YYYYMM}_Cadastro.csv`,
`{YYYYMM}_Remuneracao.csv` e `{YYYYMM}_Observacoes.csv`.

## Sobre a fonte

A CGU publica no [Portal da Transparência](https://portaldatransparencia.gov.br/download-de-dados/servidores)
os dados de remuneração e cadastro de todos os servidores, inativos e
pensionistas do Executivo Federal.

### Aposentados

Servidores civis aposentados (SIAPE e BACEN): dados cadastrais, cargo,
órgão, tipo de aposentadoria e remuneração mensal.

### Pensionistas civis

Pensionistas civis (SIAPE e BACEN): dados do pensionista, do servidor
instituidor, tipo de pensão e valor mensal.

### Pensionistas militares (Defesa)

Pensionistas de militares das Forças Armadas (`Pensionistas_DEFESA`).
Publicados após determinação do TCU (2019) e com cobertura variável
nos primeiros anos. Pode haver atraso de 1 mês em relação aos civis.

### Reserva e reforma militar

Militares da reserva remunerada e reformados (`Reserva_Reforma_Militares`):
Exército, Marinha e Aeronáutica. Contém SITUACAO_VINCULO "MILITAR DA
RESERVA" e "MILITAR REFORMADO".

## Perfil HTTP

- **Host**: `portaldatransparencia.gov.br`
- **Autenticação**: nenhuma (download público)
- **Limite**: 2 requisições/segundo
- **User-Agent**: navegador (o CDN AWS WAF da CGU bloqueia bots)
- **Redirecionamento**: seguir

## Padrão de URL

```
https://portaldatransparencia.gov.br/download-de-dados/servidores/{YYYYMM}_{TipoArquivo}
```

## Comandos

Para baixar tudo:

```bash
docker compose run --rm --build ingestao executar transparencia-inativos --sim
```

Para baixar só pensão militar:

```bash
docker compose run --rm --build ingestao executar transparencia-inativos --filtro categoria=pensionistas-defesa
```

Para baixar uma competência:

```bash
docker compose run --rm --build ingestao executar transparencia-inativos --filtro competencia=2024-01
```

Para baixar aposentados e pensionistas civis:

```bash
docker compose run --rm --build ingestao executar transparencia-inativos --filtro "categoria=aposentados"
docker compose run --rm --build ingestao executar transparencia-inativos --filtro "categoria=pensionistas-civis"
```

## Observações

- **Privacidade**: dados pessoais publicados por lei (nome, CPF parcial,
  cargo, remuneração). A Raw preserva o dado bruto; cuidado ao expor em
  painéis.
- **Pensão militar**: ficou anos fora da transparência ativa. A completude
  dos primeiros meses (2020) pode ser inferior à dos civis.
- **WAF**: o CDN da CGU bloqueia User-Agents de bots. A fonte usa um
  User-Agent de navegador.
- **Complementar à fonte `transparencia-servidores-ativos`**, que cobre
  servidores civis ativos (SIAPE/BACEN).
