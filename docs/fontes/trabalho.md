# Trabalho — Emprego Formal (RAIS e Novo CAGED)

Fonte: **ftp.mtps.gov.br** · Formato: 7z (TXT delimitado) · Atualização:
RAIS anual, CAGED mensal · Licença: dados abertos (PDET/MTE)

## O que é ingerido

### RAIS — Relação Anual de Informações Sociais

Microdados de **vínculos empregatícios** (declaração obrigatória de todo
empregador) e de **estabelecimentos**.

| Período | Formato | Arquivos por ano | Total |
|---------|---------|------------------|------:|
| 1985–2017 | per-UF (`{UF}{ANO}.7z`) | 27 UFs + 1 ESTB | 924 |
| 2018–2025 | per-região (`RAIS_VINC_PUB_{REG}.7z`) | 6 regiões + ESTAB (+ NI desde 2022) | 60 |

### Novo CAGED — Cadastro Geral de Empregados e Desempregados

Movimentações mensais (admissões, desligamentos, transferências) desde
janeiro de 2020.

| Período | Tipos | Total |
|---------|-------|------:|
| 2020-01 a 2026-11 | MOV (desde 01), FOR (desde 02), EXC (desde 04) | 245 |

**Total: 1 229 tarefas, sob demanda.**

## Sobre a fonte

O PDET (Programa de Disseminação das Estatísticas do Trabalho) publica os
microdados em um servidor FTP anônimo:

```
ftp://ftp.mtps.gov.br/pdet/microdados/RAIS/
ftp://ftp.mtps.gov.br/pdet/microdados/NOVO CAGED/
```

A RAIS mudou o formato de distribuição em 2020: antes era um arquivo por UF
(27 × ano), depois passou a ser por região geográfica (6 × ano). O arquivo
`NI` (não identificado) só existe no FTP a partir de 2022; o ano mais
recente usa `aceitar_ausencia()` porque a publicação pode atrasar.

Tocantins (TO) foi criado em 1988 e só aparece no FTP a partir de 1990.
Para os anos 1985–1989, o download de TO aceita ausência.

## Protocolo

O servidor **não** responde em HTTP (portas 80/443 timeout). A ingestão usa
o `ClienteFtp` (via `suppaftp`) encapsulado no `ClienteDespacho`, que
roteia `ftp://` automaticamente. O limite de taxa é 2 requisições/segundo
no host.

## Idempotência

**Raw (ingestão):** por recurso (`rais/tipo=vinculo/ano=2024/regiao=sp`,
`caged/tipo=mov/ano=2026/mes=01`). Re-ingerir sobrescreve o snapshot
anterior (manifesto com tamanho + MDTM do FTP).

**Silver (follow-up):** chave `município × CNAE × ano` (RAIS) ou
`município × CNAE × mês` (CAGED).

## Follow-up

- **Decodificador Bronze** — 7z → TXT → Arrow (layout varia por ano)
- **Silver** — normalização municipal, join com IBGE e CNAE
- **RAIS Identificada** — microdados com CPF (acesso restrito via PDET)
- **Conexão FTP persistente** — pool de conexões para acelerar ingestão em lote
