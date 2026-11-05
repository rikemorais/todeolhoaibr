# Refino de dados públicos

Vocabulário das camadas de dados do projeto.

## Linguagem

**Recurso**: unidade versionada de uma fonte, identificada por fonte e caminho no armazenamento.

**Tabela bronze**: conjunto de registros decodificados de um recurso; um recurso pode produzir várias tabelas.

**Tabela silver**: conjunto de registros de uma tabela bronze com contrato de grão, tipos, chave e qualidade explícitos.

**Grão**: entidade ou evento representado por uma linha de tabela.

**Chave de negócio**: campos que identificam uma linha no grão declarado, dentro do escopo temporal e da fonte definidos no contrato.

**Localidade**: área geográfica informada pela origem, com código e nível próprios; pode ser município, UF, região ou Brasil.

**Versão da regra**: identificador da transformação aplicada ao bronze para produzir uma versão silver.
