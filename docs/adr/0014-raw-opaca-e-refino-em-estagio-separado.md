# 0014. Camada Raw opaca em bytes e refino em estágio separado

- **Status:** Proposto
- **Data:** 2026-11-01

## Contexto

Hoje o projeto vai só até a ingestão: trazer o dado da origem e gravá-lo na camada Raw imutável, com manifesto de procedência ([ADR 0006](0006-layout-da-camada-raw.md)). As três fontes atuais entregam JSON de API e ZIP de arquivo.

O catálogo que vem muda a diversidade de formatos e cria uma pressão nova:

- formatos muito variados — CSV em Latin-1 (CNPJ), ZIP multiarquivo, **dado geoespacial** (Shapefile, GeoTIFF, WFS), planilhas;
- snapshots periódicos que só fazem sentido comparados entre si (o diff mensal do CNPJ);
- e, sobretudo, o objetivo analítico — cruzar tudo em `município + ano` (renda × eleito × emenda × atividade econômica) — que é **transformação**, e ainda não existe no projeto.

O risco é essa transformação (descompactar, reencodar, parsear geoespacial, padronizar colunas, derivar chaves) vazar para dentro da ingestão, acoplando o domínio a formatos de conteúdo e sujando a Raw.

## Decisão

Separar com nitidez **ingestão fiel** de **transformação**, em duas camadas.

**A Raw é opaca em bytes.** A ingestão grava o arquivo da origem **como ele vem** e **não interpreta o conteúdo**: não descompacta ZIP, não reencoda Latin-1, não reprojeta geoespacial, não reorganiza colunas. Tudo que é arquivo — ZIP, CSV, Shapefile, raster — entra pelo caminho de arquivo grande (`TarefaDeArquivo`), em fluxo e sem carregar na memória ([ADR 0010](0010-selecao-e-arquivos-grandes.md)), com o manifesto registrando procedência, `Content-Type` e assinatura. A única leitura de conteúdo permitida segue sendo a trivial já existente (extensão/`Content-Type` para nomear o objeto).

**A transformação é um estágio separado, a jusante da Raw.** Descompactar, reencodar para UTF-8, parsear geoespacial, calcular o diff entre snapshots e derivar `município + ano` acontecem num **estágio de refino**, que **lê a Raw imutável e escreve em camadas próprias** — as camadas `bronze` → `silver` → `gold` → `diamond` nomeadas no [ADR 0015](0015-arquitetura-de-dados-em-medalhao.md) —, sem nunca reescrever a Raw. Esse refino será, quando chegar a hora, um ou mais **crates novos** com suas próprias portas, fora do escopo da ingestão e da primeira versão de cada fonte.

**`nucleo`, `fontes` e `infraestrutura` seguem sem conhecer formato de conteúdo.** O conhecimento de "isto é um shapefile projetado em SIRGAS 2000" ou "este CSV é Latin-1 separado por `;`" vive na camada de refino, não na ingestão.

**Exceção explícita — navegação de paginação.** Ler o corpo da resposta apenas para **descobrir como navegar** (o link ou cursor da próxima página, como já faz `LinksDePaginacao` nas APIs paginadas da Câmara) é permitido. A fronteira é clara: a ingestão pode interpretar o conteúdo para **andar pela origem**, mas não para **transformar o dado** que grava — o corpo continua indo para a Raw sem alteração. Sem essa exceção, a regra quebraria a paginação existente e deixaria as futuras fontes paginadas sem lugar definido para essa lógica.

## Consequências

- Cada fonte nova vira, na ingestão, "trazer os bytes": uniforme, simples e testável. Geoespacial e CSV/ZIP entram sem código especial, reusando o caminho de arquivo grande.
- A Raw passa a conter dados em Latin-1, ZIP, Shapefile etc.; quem consome precisa descobrir o formato pelo manifesto/`Content-Type`. É o preço de ser fiel à origem.
- O trabalho analítico fica **represado** até existir a camada de refino. Aceitamos esse adiamento em troca de não acoplar a ingestão a formato e de manter a Raw confiável.
- Ganhamos **reprocessamento**: como a Raw é fiel à origem, o refino pode ser refeito quantas vezes for preciso, com regra nova, sem rebaixar nada da origem.
- Os **snapshots** (p.ex. o dump mensal do CNPJ) ficam na Raw como vieram; comparar snapshots e extrair aberturas/fechamentos é tarefa do refino, não da ingestão.
- Reforça o [ADR 0006](0006-layout-da-camada-raw.md) (Raw imutável) e o [ADR 0002](0002-monorepo-em-camadas.md) (camadas): a fronteira entre dado bruto e dado tratado fica explícita.

## Alternativas consideradas

- **Normalizar na ingestão** (descompactar, reencodar, padronizar colunas antes de gravar): deixa o dado pronto para consumo, mas quebra a fidelidade e a imutabilidade da Raw ([ADR 0006](0006-layout-da-camada-raw.md)), acopla o domínio a formato e impede reprocessar com regra nova. Rejeitada.
- **Guardar só o dado já parseado** (p.ex. converter tudo para Parquet e descartar o bruto): economiza espaço e facilita a análise, mas perde procedência e a capacidade de reprocessar. Rejeitada.
- **Transformar sob demanda no painel/consulta:** mistura apresentação com dado tratado e não escala para cruzamentos entre fontes. Rejeitada.
- **Criar o crate de refino agora:** daria o estágio pronto, mas é esforço grande antes de haver fontes suficientes para desenhar a camada com propriedade. Adiada — esta decisão apenas **reserva o lugar** e proíbe a transformação de entrar na ingestão.
