# 0001. Registrar decisões com ADRs, em português do Brasil

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

O Tô De Olho Aí é um projeto público, de longo prazo e aberto a novas pessoas. O código
mostra o que foi feito, mas não explica por que foi feito assim, nem o que foi descartado.
O público do projeto é a população brasileira.

## Decisão

Registramos cada decisão arquitetural em `docs/adr/`, no formato do arquivo `modelo.md`.

Todo o projeto é escrito em português do Brasil: documentação, mensagens de log e de erro,
mensagens de commit e identificadores de código. Ficam em inglês apenas as palavras-chave da
linguagem e os nomes vindos de bibliotecas externas.

## Consequências

- Quem chega ao projeto entende as intenções sem depender de quem as tomou.
- Mudar de ideia passa a exigir um novo ADR, o que é um pequeno custo deliberado.
- Identificadores em português misturados à API das bibliotecas (`grafo.add_node`) são
  aceitos como inevitáveis.
- Quem não lê português terá mais dificuldade para contribuir, o que é coerente com o público.
- Mensagens automáticas do `clap` para argumentos inválidos permanecem em inglês por enquanto.

## Alternativas consideradas

- **Documentar só no README:** não preserva o histórico nem as alternativas descartadas.
- **Código em inglês e documentação em português:** é a convenção mais comum, mas afasta
  quem quer ler o código a partir da documentação do próprio projeto.
