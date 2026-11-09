# TSE 2014: semântica do arquivo final suplementar

**Consulta:** 2026-11-09

**Issue:** [#185](https://github.com/rikemorais/todeolhoaibr/issues/185)

**Pergunta:** o que os arquivos `*_prestacao_contas_final_2014_sup` acrescentam às tabelas da prestação final regular de 2014?

## Fontes primárias consultadas

- O [conjunto oficial “Prestação de Contas Eleitorais - 2014”](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2014) lista recursos distintos chamados “Prestação de contas final” e “Prestação de contas final suplementar”, além das prestações parciais. O TSE/ASEPA identifica os sistemas SPCE como fonte do conjunto. A página informa criação em 24 de agosto de 2021 e que a data de geração dos dados está disponível dentro de cada arquivo.
- A página oficial do grupo [Prestação de Contas Eleitorais](https://dadosabertos.tse.jus.br/group/about/prestacao-de-contas-eleitorais) descreve o grupo como arquivos de receitas e despesas de campanha de candidatos, partidos e comitês. Também alerta que cada eleição tem regras específicas que podem afetar a estrutura dos arquivos e recomenda consultar o `leia-me.pdf` dentro de cada pacote.
- A [Resolução TSE nº 23.406/2014](https://www.tse.jus.br/eleicoes/eleicoes-anteriores/eleicoes-2014/normas-e-decisoes/normas-e-documentacoes/resolucao-no-23.406), art. 38, estabelece o prazo da prestação final de 2014 até 4 de novembro; para quem disputasse o segundo turno, as contas referentes aos dois turnos deveriam ser apresentadas até 25 de novembro. O § 2º estende esse prazo aos partidos com candidato no segundo turno e inclui as contas dos respectivos comitês.
- O arquivo publicado pelo TSE [`prestacao_contas_final_sup_2014.zip`](https://cdn.tse.jus.br/estatistica/sead/odsele/prestacao_contas/prestacao_contas_final_sup_2014.zip), baixado em 2026-11-09 (SHA-256 `8f36ad8aa88ff45c2bcad69a7adf50f31b040b24e5321a702f0526a2b318c7ca`), contém quatro TXT: `despesas_candidatos_prestacao_contas_final_2014_sup.txt`, `despesas_partidos_prestacao_contas_final_2014_sup.txt`, `receitas_candidatos_prestacao_contas_final_2014_sup.txt` e `receitas_partidos_prestacao_contas_final_2014_sup.txt`, além de `Leiaute_RDE_2016_1.0.1.pdf`.

## Achados

1. O TSE publica a prestação final suplementar como um recurso próprio, separado do recurso final regular. Isso confirma que `_sup` representa uma categoria de publicação distinta na organização do TSE; o nome isolado não explica a regra de inclusão das linhas.
2. O pacote suplementar tem quatro tabelas internas, correspondentes às famílias de despesas e receitas de candidatos e partidos. O pacote regular inclui também famílias de comitês. O leiaute presente no pacote tem referência a RDE 2016, apesar do recurso ser de 2014.
3. A Resolução distingue prazos para a prestação final comum e para as contas dos dois turnos. Essa regra dá contexto para a existência de entregas finais em momentos distintos, mas a página do conjunto, o recurso e o artigo consultados não afirmam que as linhas do pacote suplementar sejam exclusivamente de segundo turno, retificações, complementos de registros regulares ou quaisquer subconjuntos específicos.

## Limites

- A listagem dos arquivos e o leiaute não definem a relação linha a linha entre o pacote suplementar e o pacote final regular, nem a relação entre os arquivos nacionais e as partes por UF.
- Não foi encontrada nas fontes oficiais consultadas uma definição explícita do significado operacional de “suplementar” para essas quatro tabelas de 2014.
- Não foi inferido o significado a partir do sufixo, do tamanho dos arquivos ou da possível relação com o segundo turno.

## Implicação para o contrato Silver

Até haver documentação oficial mais específica ou uma validação de conteúdo que estabeleça o relacionamento, as quatro tabelas `*_sup` devem continuar separadas das famílias regulares e das partes por UF. Não devem ser somadas, deduplicadas ou tratadas como retificações/segundo turno por suposição. A linhagem deve preservar a URL/tabela interna e a versão do manifesto Bronze. A semântica permanece pendente.
