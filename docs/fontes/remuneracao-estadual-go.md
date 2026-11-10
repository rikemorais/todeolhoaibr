# Remuneração estadual de Goiás

Fonte oficial: [Folha de Pagamento — SEAD](https://dadosabertos.go.gov.br/dataset/folha-de-pagamento), publicada no catálogo de Dados Abertos de Goiás. O conjunto declara licença Creative Commons Atribuição (CC-BY).

## Escopo da ingestão

A fonte `remuneracao-estadual-go` consulta os metadados do CKAN, seleciona o CSV mensal de competência mais recente efetivamente listado e preserva o arquivo em fluxo na camada Raw:

```text
raw/remuneracao-estadual/uf=GO/competencia=AAAA-MM/...
```

Ela é sob demanda e não entra em `--todas`. A URL precisa pertencer ao domínio `dadosabertos.go.gov.br`, corresponder ao padrão mensal `folhapagamento_AAAAMM.csv` e ter tamanho declarado positivo, de até 1 GiB. ZIPs anuais e outros recursos não são selecionados.

Em 10/11/2026, o catálogo atualizado em 26/09/2026 listava recursos mensais de janeiro a agosto de 2026, com cerca de 35,5–36,1 MiB cada. Não constavam recursos de setembro ou outubro; a fonte usa agosto até o catálogo publicar uma competência posterior.

## Dados pessoais e limites

O dicionário e a descrição do conjunto indicam informações individualizadas de servidores, incluindo nomes e valores de remuneração. A tarefa mantém o arquivo opaco na Raw: não interpreta linhas, não imprime registros e não cria tabela analítica. O acesso ao bucket e a retenção devem seguir a governança de dados pessoais do projeto.

O escopo é a folha publicada pela SEAD para o Executivo estadual. Isso não comprova cobertura do subsídio do governador, dos demais poderes, de empresas estaduais ou de todas as competências. A pesquisa das 27 UFs e as lacunas estão em [pesquisa-remuneracao-estadual.md](pesquisa-remuneracao-estadual.md).

## Verificação operacional

A seleção do recurso foi verificada a partir de metadados CKAN, sem baixar ou inspecionar linhas dos CSVs/ZIPs. Não executar a fonte contra o bucket Raw padrão como smoke test; se for necessário validar a transferência real, usar um bucket sandbox aprovado e armazenamento com retenção controlada.
