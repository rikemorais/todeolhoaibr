# ICMBio — Unidades de Conservação (CNUC/MMA)

Fonte: **Portal de Dados Abertos do MMA** · Formatos: CSV, XLSX, SHAPE-ZIP, PDF ·
Atualização: semestral · Licença: CC BY (MMA)

## O que é ingerido

Cadastro Nacional de Unidades de Conservação (CNUC) com dois tipos de recurso:

| Tipo | Período | Formato | Tarefas |
|------|---------|---------|---------|
| **Lista de UCs ativas** (atributos) | 2018 a 2026-07 | CSV/XLSX | 16 |
| **Polígonos dos limites** | 2024-02 a 2025-08 | SHAPE-ZIP | 4 |
| **Dicionários de dados** | referência | PDF | 2 |

**Total: 22 tarefas, sob demanda.**

## Sobre a fonte

O [CNUC](https://antigo.mma.gov.br/areas-protegidas/cadastro-nacional-de-ucs.html)
é o registro oficial das Unidades de Conservação do SNUC (Sistema Nacional de
Unidades de Conservação), mantido pelo MMA e pelo ICMBio. Inclui UCs federais,
estaduais, municipais e RPPNs (particulares).

Os CSVs contêm atributos tabulares: nome da UC, categoria de manejo (Parque
Nacional, APA, REBIO, etc.), esfera (federal, estadual, municipal), bioma, UF,
área em hectares, ato legal de criação e data-base. Os SHPs contêm os polígonos
dos limites geográficos em SIRGAS 2000.

Publicados em [dados.mma.gov.br/dataset/unidadesdeconservacao](https://dados.mma.gov.br/dataset/unidadesdeconservacao).

## Perfil HTTP

- **Host**: `dados.mma.gov.br`
- **Autenticação**: nenhuma (acesso público)
- **Limite**: 2 requisições/segundo
- **Redirecionamento**: seguir

## Padrão de URL

```
https://dados.mma.gov.br/dataset/44b6dc8a-dc82-4a84-8d95-1b0da7c85dac/resource/{resource_id}/download/{filename}
```

## Comandos

Para baixar tudo:

```bash
docker compose run --rm --build ingestao executar icmbio --sim
```

Para baixar só os polígonos:

```bash
docker compose run --rm --build ingestao executar icmbio --filtro formato=shapefile
```

Para baixar um período específico:

```bash
docker compose run --rm --build ingestao executar icmbio --filtro periodo=2025-08
```

## Observações

- O CSV de 2023-07 é publicado em XLSX (não CSV).
- Os SHPs de 2026 estão hospedados em SharePoint externo do MMA com tokens
  temporários — não foram incluídos. Quando forem republicados no portal
  `dados.mma.gov.br`, podem ser adicionados.
- A data-base do CNUC está no nome do recurso (ex: `cnuc/2026-07`).
