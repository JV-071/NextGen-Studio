# NextGen Studio

Editor desktop nativo em **Rust 1.98.1**, edição 2024, com egui/eframe e OpenGL (glow).
Windows e Linux x64. Versão **0.2.0 — prévia de desenvolvimento**.

## Baixar e executar

Pacotes em [Releases](https://github.com/JV-071/NextGen-Studio/releases)
ou Actions → Desktop builds. Use **Release** no dia a dia.
Extraia o ZIP numa pasta nova e execute `bin/nextgen-studio.exe` (Windows)
ou `bin/nextgen-studio` (Linux). Não extraia sobre a antiga distribuição Qt.

- Windows 10/11 x64: runtime C estático; o pacote Rust não distribui DLLs Qt.
- Linux: baseline Ubuntu 24.04/glibc 2.39, GTK3 e driver OpenGL do sistema.
- Windows Debug abre console ao iniciar pelo Explorer; num terminal existente,
  normalmente reutiliza esse terminal. Release usa o subsistema gráfico.
- Os dois modos registram mensagens em `bin/nextgen-studio.log`. Se essa pasta
  não for gravável, usa uma pasta de estado do usuário. O botão **Local do log**
  mostra o caminho efetivo. `.log.1` é o arquivo anterior de rotação.
- O `.zip.sha256` contém o hash do ZIP. Deixe-o ao lado do ZIP se quiser
  conferir integridade; o programa não precisa dele e ele não vai em bin.

```powershell
Get-FileHash .\NextGen-Studio-0.2.0-Windows-x64-Release.zip -Algorithm SHA256
```

Compare o hash com a primeira coluna do arquivo .sha256.

## Edição

- Abas para OTUI, Lua, OTMOD, HTML e CSS; editor de código com aplicação explícita.
- Hierarquia, propriedades escalares, inclusão/exclusão de widgets, undo/redo.
- Canvas aproximado com zoom, grid, snapping configurável, pan pelo botão do meio,
  movimento e resize de elementos sem geometria controlada por anchors/layout.
- Botões de anchors ao pai; margens e tamanho editáveis no inspetor.
- Prévia local de imagens e navegação de arquivos sob demanda.
- Salvamento com arquivo temporário sincronizado, substituição, backup .bak e
  recusa de conflitos externos detectados.
- Preserva bytes não atingidos nas edições pelo inspetor, BOM e UTF-8/Windows-1252.
- Limites: 8 MiB por documento, oito abas, até 3.000 elementos desenhados,
  10.000 nós exibidos e histórico até 150 ações/32 MiB por documento.

O parser visual cobre um subconjunto de OTML. Blocos compostos, estilos e scripts
não são interpretados pelo canvas. O motor NextGen é a referência para fidelidade.
O editor de código não oferece ainda LSP, depurador ou grafo visual de comportamento.
Painéis são redimensionáveis; docking livre do Qt não foi portado nesta versão.

## Prévia nativa isolada e captura dentro do editor

1. Abra a raiz do NextGen em **Abrir projeto**.
2. Abra um OTUI. Clique **Prévia nativa • F5**.
3. Escolha o executável do NextGen e confirme a instalação da integração de desenvolvimento.
4. O Studio inicia um cliente separado e envia o texto OTUI, inclusive alterações
   aplicadas que ainda não foram salvas no arquivo original.
5. **Captura nativa** apresenta a imagem renderizada pelo motor. Interaja com a
   interface na janela separada do cliente. Não é vídeo contínuo ou superfície
   interativa embutida: a captura é atualizada após cada revisão.
6. Com atualização automática habilitada, edições aplicadas são enviadas após
   400 ms de inatividade. **Parar prévia** encerra apenas o processo iniciado pelo Studio.

A integração usa `g_ui.loadUIFromString` e `g_app.doScreenshot`, disponíveis no
NextGen examinado. Carrega o ambiente do cliente para resolver seus estilos/widgets.
Ela não simula uma conexão com servidor. Telas com referências relativas, widgets
criados por Lua ou dados de jogo online podem precisar de contexto adicional.
Scripts são executados no processo do cliente: use projetos confiáveis.

Uma sessão temporária transporta pedido, resposta, captura e log do cliente.
O módulo só atua quando iniciado com `NEXTGEN_STUDIO_SESSION_DIR`.
O Studio não sobrescreve uma integração existente diferente: preserve suas
alterações e substitua manualmente os dois arquivos por `integration/dev_studio_bridge`.
Não distribua esse módulo de desenvolvimento aos jogadores.

## A prévia já existente dentro do NextGen

O módulo `modules/dev_otui` do cliente é uma ferramenta diferente do Studio:
abra **Ctrl+Alt+U** ou o botão **OTUI Editor** no menu superior.
Use **Open…** ou informe um caminho, por exemplo `/game_idle/game_idle.otui`,
e clique **Load**. A tela carregada no palco do cliente é a prévia fiel.
Se o botão/atalho não existir, confirme que `dev_otui` foi incluído e carregado
no pacote do cliente. Não basta possuir os fontes em outra pasta.

## Build e CI

```sh
cargo build --locked
cargo test --locked
cargo build --locked --release
```

`rust-toolchain.toml` fixa Rust 1.98.1, versão estável verificada em 11/09/2026.
O lockfile fixa as dependências. A CI produz Debug/Release para Windows/Linux,
com cache Cargo separado por plataforma/configuração, dois trabalhadores,
testes de integridade e abertura do pacote instalado. Não publica artefatos
`evidence`; resultados continuam nos logs do job.

Release usa opt-level=2, sem LTO, com 16 unidades de geração de código. É uma
configuração inicial de custo-benefício, não promessa de desempenho ótimo.
As bibliotecas de UI são ligadas ao executável; o motor NextGen continua separado.

As tags v* publicam prévias somente após sucesso das quatro configurações.
Consulte [migração e limitações](docs/rust-migration.md) e
[investigação dos editores](docs/reference-audit.md).
