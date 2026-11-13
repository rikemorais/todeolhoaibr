# Remuneração estadual de Minas Gerais

Fonte oficial: [Conjunto de remuneração dos servidores ativos](https://www.dados.mg.gov.br/dataset/remuneracao-servidores-ativos), publicado no catálogo de dados abertos de Minas Gerais. A licença declarada pelo catálogo é CC-BY-4.0.

## Escopo da ingestão

A fonte `remuneracao-estadual-mg` consulta o endpoint `package_show` do CKAN, seleciona o CSV mensal válido mais recente e o grava em fluxo na camada Raw:

```text
raw/remuneracao-estadual/uf=MG/competencia=AAAA-MM/...
```

Ela é sob demanda e pode ser executada pelo nome `remuneracao-estadual-mg`. A seleção exige URL HTTPS no domínio oficial `dados.mg.gov.br` (ou no domínio legado `www.dados.mg.gov.br`), competência válida e tamanho declarado positivo de até 1 GiB. O catálogo atualmente publica arquivos no padrão `servidores-YYYY-MM.csv`; o padrão legado `servidores_MMYYYY.csv` também é aceito. O arquivo não é carregado inteiro na memória.

O adaptador preserva somente a competência mais recente anunciada no catálogo. Não faz carga histórica retroativa. Os leiautes anteriores a 2021 divergem dos arquivos mais novos e precisam de estratégia própria antes de uma eventual carga histórica.

## Dados pessoais e limites

O conjunto descreve remunerações do Executivo estadual e inclui identificadores como nome e MASP, além de CPF parcialmente anonimizado. A tarefa preserva o arquivo original opaco na Raw: não interpreta linhas, não imprime registros e não cria tabela analítica. O acesso ao bucket deve seguir a governança de dados pessoais do projeto; a disponibilidade pública na origem não elimina a necessidade de controle de acesso e retenção.

O catálogo consultado não comprova que o conjunto inclua o subsídio do governador. Portanto, a fonte não declara cobertura de governadores, dos demais poderes estaduais nem das 27 UFs. A pesquisa comparativa e o mapa das lacunas estão em [pesquisa-remuneracao-estadual.md](pesquisa-remuneracao-estadual.md).

## Verificação operacional

Não executar a fonte contra o bucket Raw padrão como smoke test: o CSV atual tem centenas de MiB e contém dados pessoais. Validar a descoberta de metadados com fixture sem registros pessoais e, se for necessário validar a transferência real, usar bucket de sandbox aprovado e armazenamento com retenção controlada.
