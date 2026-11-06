# Adoção do Apache DataFusion para a camada Silver

**Data:** 2026-11-06
**Pergunta:** como adotar Apache DataFusion para transformar Bronze em Silver neste projeto Rust, com SQL como formato principal de contribuição.

## Resumo

Apache DataFusion é um engine Rust embutível de consultas SQL/DataFrame baseado em Arrow. Ele não é, por si só, um agendador ou orquestrador de pipelines. Portanto, pode executar parte das transformações Bronze→Silver, mas não substitui a CLI, a reconciliação periódica, os contratos de cada tabela ou a publicação atômica já implementada pelo projeto.

**Decisão atual:** adotar DataFusion como engine padrão das transformações Silver e usar SQL versionado como caminho preferencial para projeções, casts, filtros, joins, agregações e deduplicação. A validação incremental continua necessária para alinhar Arrow, preservar semântica e medir recursos; ela orienta a integração e cada migração, não decide se DataFusion será adotado. Rust permanece para decodificação, funções especializadas e regras de domínio que sejam mais claras ou seguras fora do SQL.

Esta nota começou como uma avaliação de alternativas. A decisão posterior do projeto atualizou sua recomendação: migrar em fases, mantendo os contratos atuais disponíveis até cada transformação SQL demonstrar paridade e operação segura.

## O que o projeto oferece

Segundo a documentação oficial, DataFusion é uma biblioteca de execução de consultas em processo, escrita em Rust, que usa Arrow como formato em memória; oferece SQL e DataFrame, leitura de Parquet/CSV/JSON/Avro, execução vetorizada, paralela e em lotes, além de extensões. Também permite escrever resultados de DataFrame em Parquet. [Introdução oficial](https://datafusion.apache.org/user-guide/introduction.html), [guia da API DataFrame](https://datafusion.apache.org/versions/54.1.0/library-user-guide/using-the-dataframe-api.html).

A FAQ diferencia DataFusion (biblioteca de engine embutível), Arrow (formato e kernels) e Ballista (plataforma distribuída baseada em DataFusion). Isso não equivale a um produto de orquestração: o laço de uma hora, as decisões de quando executar e o registro de execuções no Dagron continuariam sob responsabilidade do todeolhoai. [FAQ oficial](https://datafusion.apache.org/user-guide/faq.html).

DataFusion tem suporte assíncrono a object stores populares, incluindo S3, e aceita implementação via trait `ObjectStore`. Isso torna plausível integrá-lo ao VersityGW, mas a compatibilidade do endpoint S3 e a interação com o storage abstraction próprio do projeto teriam de ser validadas na prática. [Introdução oficial](https://datafusion.apache.org/user-guide/introduction.html).

Há controles de memória do engine e spill para disco em operadores que suportam essa estratégia. A documentação ressalta que o limite do pool de execução não é um teto de memória do processo; recomenda também limite no processo/contêiner. Spill usa diretório e espaço temporário configuráveis e não deve ser tratado como garantia de que toda operação ficará dentro da memória disponível. [Configurações oficiais](https://datafusion.apache.org/user-guide/configs.html), [segurança e limites oficiais](https://datafusion.apache.org/library-user-guide/securing-datafusion.html).

## Ajuste ao código atual

O workspace usa Rust 1.94 como MSRV. A branch da Fase 0 adiciona DataFusion 55.2.0 e alinha Arrow/Parquet para 59.x; o lockfile resolve 59.3.2 e o `cargo tree` não mostra versões duplicadas desses tipos. O leitor Silver abre cada objeto Bronze apontado pelo manifesto vigente, valida os lotes em fluxo e preserva manifesto, parte e ordinal como linhagem. O publicador escreve Parquet temporário, grava manifesto e partes e só então atualiza o ponteiro da versão vigente. Antes da migração, a transformação é uma função Rust por lote. Consulte `crates/refino/Cargo.toml`, `crates/refino/src/linhagem.rs` e `crates/refino/src/silver/publicacao.rs`.

Assim, DataFusion é uma dependência de execução dentro do crate `refino`, não uma camada de persistência nova nem substituto do contrato Bronze/Silver do ADR 0020. Para preservar a garantia de publicação existente, a saída do engine alimentará o mesmo caminho de partes temporárias, manifesto íntegro e ponteiro publicado por último.

### Onde pode ajudar

- Transformações que sejam essencialmente projeções, casts, filtros e agregações podem ser expressas em SQL/DataFrame e aproveitar planejamento e otimizações do engine.
- Leitura e gravação Parquet integradas podem reduzir código de execução para casos simples.
- Execução paralela e operações com spill podem ajudar em volume, desde que as operações da regra e o ambiente real sejam compatíveis com os limites e recursos disponíveis.

### Custos e riscos que precisam ser medidos

- O projeto já tem Arrow/Parquet e streaming por lotes. O primeiro `cargo check -p todeolho-refino` com DataFusion 55.2.0 resolveu 96 pacotes e levou 1m20s nesta máquina; a adoção aumenta compilação, superfície de configuração e manutenção.
- Regras de domínio — por exemplo, conflito de chave com mensagens, deduplicação apenas de linhas integralmente iguais, aliases históricos, sentinelas e vínculo da origem por ordinal — não desaparecem ao converter a transformação em SQL. Precisam continuar explícitas e testadas.
- Agregações, ordenações, joins ou deduplicação podem precisar de estado global ou spill. A semântica de erro, a estabilidade do resultado e o consumo de memória devem ser comparados com os validadores Rust atuais.
- A compatibilidade das versões de Arrow/DataFusion, a leitura do VersityGW e a escrita das partes na arquitetura de publicação do projeto são questões de integração, não garantias derivadas do suporte genérico a S3/Parquet.
- DataFusion não torna automaticamente qualquer plano limitado ao `mem_limit` do Compose; esse limite externo continua necessário.

## Plano de integração e validação incremental

1. Resolver no ADR e no executor a compatibilidade de versões Arrow/DataFusion e a leitura limitada às partes Bronze do manifesto vigente.
2. Migrar uma transformação simples (BCB/IBGE) como piloto inicial e uma transformação que exige unicidade/deduplicação global, como despesas pagas do TSE, em sua fase própria.
3. Alimentar as mesmas partes Bronze vigentes e preservar as mesmas regras, contagens, erros esperados, chaves e linhagem. Não comparar somente tempo: comparar igualdade semântica e comportamento sob conflito.
4. Medir build incremental e limpo, tempo, RSS máximo, CPU, bytes temporários e limites do contêiner, em volume representativo. Incluir reprocessamento idempotente e publicação interrompida antes do ponteiro.
5. Concluir a migração dos contratos elegíveis quando a paridade e os limites operacionais forem demonstrados. Manter em Rust somente as exceções justificadas; adiar uma expressão específica se ela enfraquecer contrato, linhagem ou recuperação.

As observações sobre capacidades são **fatos documentados** nas fontes abaixo. Ganhos de desempenho, redução de código e consumo de recursos são **hipóteses** que precisam de benchmark nos contratos reais.

## Fontes primárias

- [Apache DataFusion: introdução e capacidades](https://datafusion.apache.org/user-guide/introduction.html)
- [Apache DataFusion: API DataFrame e gravação Parquet](https://datafusion.apache.org/versions/54.1.0/library-user-guide/using-the-dataframe-api.html)
- [Apache DataFusion: FAQ — Arrow, DataFusion e Ballista; engine embutível](https://datafusion.apache.org/user-guide/faq.html)
- [Apache DataFusion: configurações de runtime e memória](https://datafusion.apache.org/user-guide/configs.html)
- [Apache DataFusion: limites de segurança, memória e spill](https://datafusion.apache.org/library-user-guide/securing-datafusion.html)
- [ADR 0020 — contrato e publicação Silver](../adr/0020-contrato-da-camada-silver.md)
