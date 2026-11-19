# DATASUS — Indicadores de Saúde

Fonte: **ftp.datasus.gov.br** · Formato: DBC (dBASE comprimido) ·
Atualização: anual (SIM, SINASC) e mensal (SIH, CNES) · Licença: dados abertos (MS/DATASUS)

## O que é ingerido

Quatro sistemas do portal de microdados do DATASUS:

| Sistema | Subset | Período | Cadência | Arquivos | Formato |
|---------|--------|---------|----------|----------|---------|
| **SIM** | DORES (óbitos) | 1996–2023 | anual × UF | 756 | DBC |
| **SIM** | DOFET (fetais) | 1996–2023 | anual × UF | 756 | DBC |
| **SINASC** | DN (nascidos) | 1996–2023 | anual × UF | 756 | DBC |
| **SIH** | RD (internações) | 2008-01 a 2024-12 | mensal × UF | 5 508 | DBC |
| **CNES** | ST (estabelecimentos) | 2005-08 a 2025-01 | mensal × UF | 6 318 | DBC |

**Total: 14 094 tarefas, sob demanda.**

## Sobre a fonte

O DATASUS (Departamento de Informática do SUS) publica microdados de saúde
via FTP. Os dados são públicos e agregados por município/UF. Sistemas:

- **SIM** (Sistema de Informações sobre Mortalidade): declarações de óbito
  por residência (DORES) e óbitos fetais (DOFET). CID-10 de 1996 em diante.
- **SINASC** (Sistema de Informação sobre Nascidos Vivos): declarações de
  nascidos vivos (DN), com dados da mãe, parto e recém-nascido.
- **SIH** (Sistema de Informações Hospitalares): registros de internação
  hospitalar (AIH), com diagnóstico, procedimento e valores pagos.
- **CNES** (Cadastro Nacional de Estabelecimentos de Saúde): cadastro de
  estabelecimentos, leitos, equipamentos e profissionais.

## Nomenclatura dos arquivos

### SIM

```
ftp://ftp.datasus.gov.br/dissemin/publicos/SIM/CID10/DORES/DO{UF}{AAAA}.dbc
ftp://ftp.datasus.gov.br/dissemin/publicos/SIM/CID10/DOFET/DO{UF}{AAAA}.dbc
```

### SINASC

```
ftp://ftp.datasus.gov.br/dissemin/publicos/SINASC/1996_/Dados/DNRES/DN{UF}{AAAA}.dbc
```

### SIH

```
ftp://ftp.datasus.gov.br/dissemin/publicos/SIHSUS/200801_/Dados/RD{UF}{AA}{MM}.dbc
```

Ano de 2 dígitos: `RD` + UF + AA + MM (ex.: `RDSP2401.dbc` = SP, jan/2024).

### CNES

```
ftp://ftp.datasus.gov.br/dissemin/publicos/CNES/200508_/Dados/ST/ST{UF}{AA}{MM}.dbc
```

Mesmo padrão mensal de 2 dígitos do SIH.

## Protocolo

Download via FTP anônimo (sem autenticação).
Limite: 2 requisições/segundo no host.

## Formato DBC

Os arquivos DBC são dBASE comprimidos com algoritmo proprietário (blast).
A ingestão Raw armazena o `.dbc` como está; a conversão para CSV/Parquet
acontece na camada Bronze (follow-up).

Ferramentas de referência para conversão: `blast-dbf` (C), `read.dbc` (R),
`PySUS` (Python), `datasus-fetcher` (Python).

## Idempotência

**Raw (ingestão):** por recurso (`sim/dores/ano=2023/uf=sp`,
`sih/competencia=202401/uf=sp`, `cnes/competencia=202501/uf=ba`).
Re-ingerir sobrescreve o snapshot anterior (manifesto com tamanho/data FTP).

**Silver (follow-up):** chave `município + competência` (SIH/CNES) ou
`município + ano` (SIM/SINASC).

## Follow-up

- **Bronze**: converter DBC para Parquet/CSV com `blast-dbf` ou equivalente
- **Silver**: normalizar códigos de município (IBGE), CID-10, procedimentos SUS
- **Indicadores derivados**: mortalidade infantil, razão de leitos, cobertura vacinal
