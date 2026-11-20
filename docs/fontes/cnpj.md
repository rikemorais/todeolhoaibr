# Fonte `cnpj` — Cadastro Nacional da Pessoa Jurídica (Receita Federal)

Cadastro completo de empresas e estabelecimentos do país (ativos e baixados), do
MEI à grande empresa. Base para análise de aberturas/fechamentos, capital social,
porte e saúde empresarial por estado e município.

Origem (pasta pública da Receita):
<https://arquivos.receitafederal.gov.br/index.php/s/YggdBLfdninEJX9>.
Download direto, sem credenciais, pelo WebDAV público:
<https://arquivos.receitafederal.gov.br/public.php/dav/files/YggdBLfdninEJX9/>.
Layout oficial: <https://www.gov.br/receitafederal/dados/cnpj-metadados.pdf>.

## O que é ingerido

**37 ZIPs por snapshot mensal**, baixados via `TarefaDeArquivo` como arquivos
grandes (em fluxo, sem carregar na memória) e gravados opacos na Raw:

| Tabela | Arquivos | Colunas |
|--------|----------|---------|
| Empresas | `Empresas0.zip`–`Empresas9.zip` (10) | cnpj_basico, razao_social, natureza_juridica, qualificacao_responsavel, capital_social, porte_empresa, ente_federativo |
| Estabelecimentos | `Estabelecimentos0.zip`–`Estabelecimentos9.zip` (10) | cnpj_basico, cnpj_ordem, cnpj_dv, matriz_filial, nome_fantasia, situacao_cadastral, data_situacao, motivo_situacao, cidade_exterior, pais, data_inicio, cnae_principal, cnae_secundaria, tipo_logradouro, logradouro, numero, complemento, bairro, cep, uf, municipio, ddd1, telefone1, ddd2, telefone2, ddd_fax, fax, email, situacao_especial, data_situacao_especial |
| Sócios | `Socios0.zip`–`Socios9.zip` (10) | cnpj_basico, identificador_socio, nome_socio, cnpj_cpf_socio, qualificacao, data_entrada, pais, representante_legal, nome_representante, qualificacao_representante, faixa_etaria |
| Simples/MEI | `Simples.zip` (1) | cnpj_basico, opcao_simples, data_opcao_simples, data_exclusao_simples, opcao_mei, data_opcao_mei, data_exclusao_mei |
| Países | `Paises.zip` (1) | codigo, descricao |
| Municípios | `Municipios.zip` (1) | codigo, descricao |
| CNAEs | `Cnaes.zip` (1) | codigo, descricao |
| Naturezas Jurídicas | `Naturezas.zip` (1) | codigo, descricao |
| Qualificações de Sócios | `Qualificacoes.zip` (1) | codigo, descricao |
| Motivos de Situação Cadastral | `Motivos.zip` (1) | codigo, descricao |

**Formato CSV:** separador `;`, encoding **ISO-8859-1** (Latin-1), **sem cabeçalho**.
Os nomes internos dos ZIPs usam sufixos próprios da Receita, como `.EMPRECSV`,
`.ESTABELE`, `.SOCIOCSV` e `.SIMPLES.CSV.D61010`; o Bronze reconhece esses
nomes ao selecionar o esquema de cada tabela.

## Layout na Raw

```text
raw/cnpj/{competencia}/{tabela}/          (ex.: raw/cnpj/2026-10/empresas/0/)
  _ultima_versao.json
  data_ingestao=AAAA-MM-DD/execucao=<id>/
    parte-00001.zip
    _manifesto.json
```

## Fonte `sob_demanda`

São ~**5–6 GB compactados** (~40 GB descompactados) por snapshot. É
`sob_demanda = true`: **não entra no `executar --todas`**.

## Como rodar

```bash
# Todo o snapshot de uma competência
executar cnpj --etiquetas competencia=2026-10

# Apenas as empresas
executar cnpj --etiquetas competencia=2026-10,tabela=empresas

# Uma única parte
executar cnpj --etiquetas competencia=2026-10,tabela=empresas,parte=0
```

## Idempotência

A ingestão é idempotente **por competência** (mês do dump). Se o ZIP já foi
baixado, a URL configurada é a mesma registrada no manifesto Raw e o ETag/versão
não mudou, a tarefa pula o download. Uma mudança de endpoint força a reingestão,
mesmo quando o novo servidor reutiliza o ETag. Ao implantar a troca de URL,
pause agendadores e filas que possam iniciar imagens antigas, aguarde todas as
execuções antigas terminarem e confirme que não há workers antigos em execução
ou pendentes antes de iniciar a nova imagem. Só reative os agendamentos depois
que a nova imagem estiver ativa. O ponteiro compara a hora de início, mas não
tem um token de geração que consiga cercar um binário antigo; se uma imagem
antiga voltar a executar depois da troca, ela ainda pode acessar o endpoint
anterior e publicar seu resultado. Cada competência é um snapshot completo —
não há deltas.

## Estratégia de diff entre snapshots

A base da Receita é um **snapshot pontual, não um log de eventos**. Para construir
séries temporais (aberturas/fechamentos ao longo do tempo):

1. Ingerir **snapshots mensais consecutivos** na Raw (cada um preservado com sua
   competência).
2. Na camada **Silver**, comparar o snapshot do mês N com o do mês N-1:
   - CNPJs presentes em N mas ausentes em N-1 → **aberturas** do período.
   - CNPJs em N-1 que mudaram `situacao_cadastral` para `08` (baixada) em N →
     **fechamentos**.
   - Mudanças de capital social, CNAE ou endereço → eventos de alteração.
3. Dentro de um único snapshot, usar `data_inicio_atividade` e
   `data_situacao_cadastral` como aproximações históricas.

## Observações

- **CNPJ alfanumérico a partir de jul/2026** para novas empresas. O CNPJ deve ser
  tratado como `String`, nunca como inteiro.
- **CPFs parcialmente mascarados** nos sócios (3 primeiros e 2 últimos dígitos
  ocultos, conforme a Lei 13.473/2017).
- O campo **CNAE Fiscal Secundária** pode conter múltiplos códigos separados por
  vírgula dentro do mesmo campo.
- A conversão de encoding (Latin-1 → UTF-8) será feita na camada Bronze.
