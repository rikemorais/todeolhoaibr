# Avaliação do Apache DataFusion para a camada Silver

**Data:** 2026-11-06
**Pergunta:** usar Apache DataFusion para transformar Bronze em Silver neste projeto Rust.

## Resumo

Apache DataFusion é um engine Rust embutível de consultas SQL/DataFrame baseado em Arrow. Ele não é, por si só, um agendador ou orquestrador de pipelines. Portanto, pode executar parte das transformações Bronze→Silver, mas não substitui a CLI, a reconciliação periódica, os contratos de cada tabela ou a publicação atômica já implementada pelo projeto.

**Recomendação:** não trocar agora o caminho comum de Silver pelo DataFusion. Fazer primeiro uma prova de conceito isolada e comparativa, com um contrato simples e outro com deduplicação global, medindo memória, tempo, dependências e manutenção. O código atual já lê e escreve Arrow/Parquet em lotes, e muitas regras são específicas por fonte. O ganho potencial existe sobretudo para transformações declarativas SQL, filtros, projeções e agregações; precisa ser demonstrado no workload real.

## O que o projeto oferece

Segundo a documentação oficial, DataFusion é uma biblioteca de execução de consultas em processo, escrita em Rust, que usa Arrow como formato em memória; oferece SQL e DataFrame, leitura de Parquet/CSV/JSON/Avro, execução vetorizada, paralela e em lotes, além de extensões. Também permite escrever resultados de DataFrame em Parquet. [Introdução oficial](https://datafusion.apache.org/user-guide/introduction.html), [guia da API DataFrame](https://datafusion.apache.org/versions/54.1.0/library-user-guide/using-the-dataframe-api.html).

A FAQ diferencia DataFusion (biblioteca de engine embutível), Arrow (formato e kernels) e Ballista (plataforma distribuída baseada em DataFusion). Isso não equivale a um produto de orquestração: o laço de uma hora, as decisões de quando executar e o registro de execuções no Dagron continuariam sob responsabilidade do todeolhoai. [FAQ oficial](https://datafusion.apache.org/user-guide/faq.html).

DataFusion tem suporte assíncrono a object stores populares, incluindo S3, e aceita implementação via trait `ObjectStore`. Isso torna plausível integrá-lo ao VersityGW, mas a compatibilidade do endpoint S3 e a interação com o storage abstraction próprio do projeto teriam de ser validadas na prática. [Introdução oficial](https://datafusion.apache.org/user-guide/introduction.html).

Há controles de memória do engine e spill para disco em operadores que suportam essa estratégia. A documentação ressalta que o limite do pool de execução não é um teto de memória do processo; recomenda também limite no processo/contêiner. Spill usa diretório e espaço temporário configuráveis e não deve ser tratado como garantia de que toda operação ficará dentro da memória disponível. [Configurações oficiais](https://datafusion.apache.org/user-guide/configs.html), [segurança e limites oficiais](https://datafusion.apache.org/library-user-guide/securing-datafusion.html).

## Ajuste ao código atual

O workspace já usa Rust, Arrow `60.1.0` e Parquet `60.1.0`. O leitor Silver abre cada objeto Bronze apontado pelo manifesto vigente, valida os lotes em fluxo e preserva manifesto, parte e ordinal como linhagem. O publicador escreve Parquet temporário, grava manifesto e partes e só então atualiza o ponteiro da versão vigente. A transformação é uma função Rust por lote. Consulte `crates/refino/Cargo.toml`, `crates/refino/src/linhagem.rs` e `crates/refino/src/silver/publicacao.rs`.

Assim, DataFusion seria uma dependência de execução adicional dentro do crate `refino`, não uma camada de persistência nova nem substituto do contrato Bronze/Silver do ADR 0020. Para preservar a garantia de publicação existente, o resultado do engine teria de alimentar o mesmo caminho de partes temporárias, manifesto íntegro e ponteiro publicado por último.

### Onde pode ajudar

- Transformações que sejam essencialmente projeções, casts, filtros e agregações podem ser expressas em SQL/DataFrame e aproveitar planejamento e otimizações do engine.
- Leitura e gravação Parquet integradas podem reduzir código de execução para casos simples.
- Execução paralela e operações com spill podem ajudar em volume, desde que as operações da regra e o ambiente real sejam compatíveis com os limites e recursos disponíveis.

### Custos e riscos que precisam ser medidos

- O projeto já tem Arrow/Parquet e streaming por lotes. O engine pode duplicar capacidades existentes e aumentar compilação, superfície de configuração e manutenção.
- Regras de domínio — por exemplo, conflito de chave com mensagens, deduplicação apenas de linhas integralmente iguais, aliases históricos, sentinelas e vínculo da origem por ordinal — não desaparecem ao converter a transformação em SQL. Precisam continuar explícitas e testadas.
- Agregações, ordenações, joins ou deduplicação podem precisar de estado global ou spill. A semântica de erro, a estabilidade do resultado e o consumo de memória devem ser comparados com os validadores Rust atuais.
- A compatibilidade das versões de Arrow/DataFusion, a leitura do VersityGW e a escrita das partes na arquitetura de publicação do projeto são questões de integração, não garantias derivadas do suporte genérico a S3/Parquet.
- DataFusion não torna automaticamente qualquer plano limitado ao `mem_limit` do Compose; esse limite externo continua necessário.

## Plano de prova de conceito

1. Adicionar DataFusion somente em uma branch experimental, sem alterar o publicador comum.
2. Escolher uma transformação simples (por exemplo, casts/projeções de uma tabela já coberta) e uma transformação que exige unicidade/deduplicação global, como despesas pagas do TSE.
3. Alimentar as mesmas partes Bronze vigentes e preservar as mesmas regras, contagens, erros esperados, chaves e linhagem. Não comparar somente tempo: comparar igualdade semântica e comportamento sob conflito.
4. Medir build incremental e limpo, tempo, RSS máximo, CPU, bytes temporários e limites do contêiner, em volume representativo. Incluir reprocessamento idempotente e publicação interrompida antes do ponteiro.
5. Manter DataFusion apenas se houver ganho operacional claro sem enfraquecer contratos, linhagem e recuperação. Caso contrário, conservar Arrow/Parquet direto.

Os itens de avaliação e recomendação são **inferências** a partir das capacidades documentadas e da arquitetura existente; precisam de benchmark para confirmar qualquer ganho de desempenho ou redução de código.

## Fontes primárias

- [Apache DataFusion: introdução e capacidades](https://datafusion.apache.org/user-guide/introduction.html)
- [Apache DataFusion: API DataFrame e gravação Parquet](https://datafusion.apache.org/versions/54.1.0/library-user-guide/using-the-dataframe-api.html)
- [Apache DataFusion: FAQ — Arrow, DataFusion e Ballista; engine embutível](https://datafusion.apache.org/user-guide/faq.html)
- [Apache DataFusion: configurações de runtime e memória](https://datafusion.apache.org/user-guide/configs.html)
- [Apache DataFusion: limites de segurança, memória e spill](https://datafusion.apache.org/library-user-guide/securing-datafusion.html)
- [ADR 0020 — contrato e publicação Silver](../adr/0020-contrato-da-camada-silver.md)
