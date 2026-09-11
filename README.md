# NextGen Studio

Editor desktop nativo de interfaces OTUI para projetos da família OTClient.
C++20 e Qt 6 Widgets. Windows e Linux x64. **Primeira versão: 0.1.0 (preview).**

Este é um produto em desenvolvimento. O canvas apresenta um **esquema aproximado**;
a renderização fiel usa uma janela separada do cliente NextGen, por meio de um módulo
de desenvolvimento opcional. O editor não distribui o cliente nem recursos do jogo.

## Usar sem compilar

Baixe os pacotes permanentes em [Releases](https://github.com/JV-071/NextGen-Studio/releases). Para versões de desenvolvimento, em **Actions → Desktop builds**, abra uma execução bem-sucedida e baixe
`NextGen-Studio-Windows-x64-Release` ou `NextGen-Studio-Linux-x64-Release`.
Extraia o ZIP do artefato e o pacote interno. Execute `bin/nextgen-studio.exe`
no Windows ou `bin/nextgen-studio` no Linux. Mantenha as bibliotecas do pacote.
Arquivos SHA-256 acompanham os pacotes.

Windows: x64, Windows 10/11. Linux: referência Ubuntu 24.04 x64 (glibc 2.39+);
outras distribuições não estão certificadas. O pacote Debug do Windows é para
desenvolvedores com runtime de depuração do Visual C++ instalado. Use Release
no computador de uso diário. Os binários iniciais não têm assinatura de código.

## Funcionalidades entregues

- Aplicativo independente, sem navegador, WebView ou servidor web.
- Painéis reposicionáveis de projeto, hierarquia, propriedades, imagens e diagnóstico.
- Documentos OTUI em abas, inclusão/exclusão de elementos, propriedades e editor de código.
- Seleção, zoom e manipulação básica de tamanho/posição no esquema.
- Desfazer/refazer por diferenças de texto, com limite de 150 ações e orçamento de
  32 MiB por documento; liberação explícita do histórico anterior ao exceder o orçamento.
- Preservação de bytes não editados, comentários, BOM, UTF-8/Windows-1252 e finais de linha
  nas alterações pelo inspetor; mudanças pelo editor de código normalizam finais de linha.
- Salvamento com substituição atômica, backup da versão anterior e detecção de alteração externa.
- Pré-visualização de imagens com tamanho reduzido; navegação de arquivos carregada sob demanda.
- Integração experimental F5 com o motor NextGen (instruções abaixo).

A edição visual não cobre toda a gramática OTML nem interpreta Lua. Propriedades
compostas podem ser alteradas em **Código (Ctrl+E)**. Arquivos Lua/OTMOD são exibidos
na árvore, mas a edição desses arquivos e o grafo visual de comportamento ainda
fazem parte das próximas etapas.

## Primeiros passos

1. Execute o editor: um exemplo original já aparece.
2. Use **Projeto → Abrir projeto** para selecionar a raiz do NextGen.
3. Abra um arquivo `.otui` e selecione um elemento na hierarquia.
4. Edite a coluna Valor das propriedades; **Ctrl+Z** desfaz.
5. **Ctrl+Shift+A** adiciona um elemento; **Ctrl+E** abre o código.
6. Salve uma cópia para experimentar. O editor recusa sobrescrever um arquivo
   modificado externamente; use reabrir ou salvar como.

O esquema tem limites deliberados: até 3.000 elementos desenhados, 10.000 nós na
árvore, oito documentos e 8 MiB por arquivo. Limites de documento não equivalem a um
limite total de RAM: árvores, strings, Qt e imagens também consomem memória.
Não há renderização contínua de 60 quadros por segundo enquanto o editor está ocioso.

## Prévia nativa experimental

**F5** solicita a instalação de `modules/dev_studio_bridge` no projeto selecionado
e a escolha do executável do cliente. O OTUI precisa estar dentro de `modules/`.
A integração só atua quando o Studio inicia o processo com
`NEXTGEN_STUDIO_SESSION`; clientes abertos normalmente não fazem polling.

Cada solicitação leva um token de sessão, revisão e caminho. O cliente verifica
o pedido a cada 500 ms, carrega o OTUI pelo próprio `g_ui.loadUI` e registra o
resultado na saída. **Shift+F5** encerra o processo de prévia.
Não há servidor local, simulação de conexão online ou substituição de funções globais.

A prévia executa scripts do projeto: use projetos confiáveis. Interfaces que
dependem de jogador conectado, widgets criados por Lua ou dados do servidor podem
precisar de contexto adicional. A integração não está homologada para todos os
módulos NextGen. Não distribua o módulo de desenvolvimento aos jogadores.
Se já existir uma integração modificada, o editor preserva os arquivos e pede
atualização manual; não os sobrescreve silenciosamente.

## Compilar (opcional)

Requer CMake 3.25+, Ninja, Qt 6.8+ Core/Gui/Widgets e compilador C++20
(MSVC 2022 ou GCC 13 são as referências de CI).
Aponte `CMAKE_PREFIX_PATH` para o Qt.

```sh
cmake --preset release
cmake --build --preset release
ctest --test-dir build/release --output-on-failure
```

Troque `release` por `debug` para depuração. No Windows, use um terminal do
Visual Studio. `STUDIO_DEPLOY_RUNTIME=ON` inclui o runtime Qt na instalação.
`STUDIO_ENABLE_IPO=ON` é opcional e não vem habilitado por padrão.

A CI tem quatro configurações independentes. Usa Qt pré-compilado em cache,
ccache no Linux, sccache no Windows e Ninja com dois trabalhadores.
O cache nunca substitui testes. Alterações só em documentação não disparam builds.

Consulte [arquitetura e evolução](docs/architecture.md),
[investigação dos editores](docs/reference-audit.md) e
[dependências e licenças](THIRD_PARTY.md).
