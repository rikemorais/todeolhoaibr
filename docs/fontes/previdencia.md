# Previdência — Benefícios do RGPS/INSS

Fonte: **dadosabertos.inss.gov.br** (S3 AWS) · Formato: XLSX e ZIP/CSV ·
Atualização: mensal · Licença: dados abertos (INSS/MPS)

## O que é ingerido

Três datasets do Portal de Dados Abertos do INSS, publicados mensalmente:

| Dataset | Período | Arquivos | Formato | Campos principais |
|---------|---------|----------|---------|-------------------|
| **Concedidos** | Jun/2023–Out/2026 | 41 | XLSX | competência, espécie, CID, sexo, clientela, município |
| **Emitidos** | Jun/2023–Set/2026 | 39 | ZIP (CSV) | espécie, UF, município, valor líquido, meio de pagamento |
| **Mantidos** | Jun/2023–Set/2026 | 117 | ZIP (CSV) | espécie, CID, clientela, UF, motivo cessação/suspensão |

Mantidos tem três arquivos por mês: **ativos**, **cessados** e **suspensos**.

**Total: 201 tarefas, sob demanda.**

## Sobre a fonte

O INSS publica microdados de benefícios previdenciários do Regime Geral
(RGPS) em um portal CKAN (`dadosabertos.inss.gov.br`). Os arquivos ficam
em um bucket S3 (`armazenamento-dadosabertos.s3.sa-east-1.amazonaws.com`).

A publicação corresponde ao que antes era consolidado no AEPS (Anuário
Estatístico da Previdência Social) e no BEPS (Boletim Estatístico),
mas em formato estruturado e com granularidade municipal.

## Nomenclatura dos arquivos

Os **emitidos** e **mantidos** seguem padrões razoavelmente estáveis:

- Emitidos: `D.SDA.PDA.003.EMI.YYYYMM.CSV.ZIP`
- Mantidos: `D.SDA.PDA.004.{MANATIVOS|MANCESSADOS|MANSUSPENSOS}.YYYYMM.{CVS|CSV}.ZIP`

Nota: o INSS usa `CVS` (typo) na extensão dos arquivos de ativos.

Os **concedidos** não seguem padrão algum — cada mês tem um nome
diferente, incluindo consultas SUIBE, datas de extração e abreviações
variadas. As URLs são extraídas da API CKAN e hardcoded na fonte.

Recentemente (Jul/2026+), ambos datasets mudaram para o padrão
`D.DLK.FRM.000.DADOSABERTOS.*`. As tarefas usam `aceitar_ausencia()`
para os meses mais recentes.

Abril de 2024 está ausente em emitidos e mantidos.

## Protocolo

Download direto via HTTPS do S3 da AWS. Sem autenticação.
Limite: 4 requisições/segundo no host.

## Idempotência

**Raw (ingestão):** por recurso (`concedidos/competencia=202306`,
`emitidos/competencia=202501`, `mantidos/status=ativos/competencia=202412`).
Re-ingerir sobrescreve o snapshot anterior (manifesto com ETag/tamanho).

**Silver (follow-up):** chave `espécie + clientela + UF + período`.

## Follow-up

- **Decodificador Bronze** — XLSX → Arrow (concedidos), ZIP/CSV → Arrow (emitidos/mantidos)
- **Silver** — normalização de espécie e clientela, join com IBGE
- **CKAN API dinâmica** — consultar a API para descobrir URLs de concedidos futuros
- **AEPS tabular** — se o MPS publicar XLS das tabelas do anuário
