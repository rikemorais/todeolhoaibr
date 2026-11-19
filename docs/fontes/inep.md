# INEP — Indicadores de Educação

Fonte: **download.inep.gov.br** · Formato: ZIP (CSV dentro), XLSX ·
Atualização: anual (Censo/ENEM) e bienal (IDEB) · Licença: dados abertos (MEC/INEP)

## O que é ingerido

Três datasets do portal de dados abertos do INEP:

| Dataset | Período | Arquivos | Formato | Campos principais |
|---------|---------|----------|---------|-------------------|
| **Censo Escolar** | 1995–2025 | 31 | ZIP (CSV) | escola, matrícula, turma, docente, município, UF |
| **ENEM** | 1998–2025 | 28 | ZIP (CSV) | inscrição, notas, perfil socioeconômico, município |
| **ENEM redação** | 2019–2024 | 6 | ZIP (CSV) | competências da redação por participante |
| **IDEB** | 2017–2025 | 42 | XLSX/ZIP | nota IDEB, aprovação, Saeb, por escola/município/UF |

**Total: 107 tarefas, sob demanda.**

## Sobre a fonte

O INEP (Instituto Nacional de Estudos e Pesquisas Educacionais Anísio Teixeira)
publica microdados e indicadores de educação básica e superior. Os três
conjuntos mais relevantes para análise socioeconômica municipal são:

- **Censo Escolar**: levantamento anual de escolas, matrículas, turmas e
  docentes da educação básica. Desce a escola/município.
- **ENEM**: microdados do Exame Nacional do Ensino Médio. Desce a
  participante (anonimizado), com notas, perfil socioeconômico e município.
- **IDEB**: Índice de Desenvolvimento da Educação Básica, calculado
  bienalmente a partir das taxas de aprovação (Censo) e das notas do Saeb.

## Nomenclatura dos arquivos

### Censo Escolar

```
https://download.inep.gov.br/dados_abertos/microdados_censo_escolar_{ANO}.zip
```

Exceção: 2025 tem underscore extra (`_2025_.zip`).

### ENEM

```
https://download.inep.gov.br/microdados/microdados_enem_{ANO}.zip
https://download.inep.gov.br/microdados/competencias_redacao_enem_{ANO}.zip  (2019–2024)
```

### IDEB

```
https://download.inep.gov.br/ideb/resultados/divulgacao_{stage}_{nivel}_{ANO}.{xlsx|zip}
https://download.inep.gov.br/ideb/resultados/divulgacao_brasil_ideb_{ANO}.{xlsx|zip}
https://download.inep.gov.br/ideb/resultados/divulgacao_regioes_ufs_ideb_{ANO}.{xlsx|zip}
```

A partir de 2025, a extensão muda de `.xlsx` para `.zip`.
Ensino médio integrado (2025): `divulgacao_{nivel}_Ensino_Medio_integrado_ideb_2025.xlsx`.

## Protocolo

Download direto via HTTPS. Sem autenticação.
Limite: 2 requisições/segundo no host.

## Idempotência

**Raw (ingestão):** por recurso (`censo-escolar/ano=2024`, `enem/ano=2023`,
`ideb/escola/anos_iniciais/edicao=2023`).
Re-ingerir sobrescreve o snapshot anterior (manifesto com ETag/tamanho).

**Silver (follow-up):** chave `escola/município + ano` (Censo/IDEB) ou
`inscrição + edição` (ENEM).

## Follow-up

- **Bronze**: descompactar ZIP, decodificar CSV com encoding correto (Latin-1 nos mais antigos)
- **Silver**: normalizar códigos INEP de escola e município, join com IBGE
- **ENEM streaming**: arquivos grandes — processar sem carregar em memória
- **Indicadores derivados**: taxa de distorção idade-série, formação docente
