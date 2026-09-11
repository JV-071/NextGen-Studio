# Investigação dos editores de referência

Data: 11 de setembro de 2026. Objetivo: decisões de arquitetura, integridade,
desempenho e fluxo de edição para o NextGen Studio.

## Método e alcance

Inspeção estática dos snapshots fornecidos na pasta local editors, incluindo
manifests, documentação, licenças e trechos de implementação. O inventário percorreu
as árvores com exclusões de .git, node_modules, bin, obj, packages e build.
Contagens são arquivos encontrados, não arquivos integralmente auditados.
Não foram executados benchmarks comparativos nem homologados todos esses programas.

| Snapshot | Arquivos inventariados | Stack observada |
|---|---:|---|
| Assets-Editor-main | 84 | C#, .NET, WPF/WinForms, MoonSharp |
| Beats-Assets-Editor-main | 418 | Rust, Tauri, Svelte, TypeScript |
| Honey_2_1 | 31 | HTML/CSS, JavaScript, Canvas/Pixi |
| NexaMap-Editor-main | 768 | C++, wxWidgets, OpenGL |
| otclientv8-offline-map-explorer-master | 1.506 | C++, Lua, OTUI |
| remeres-map-editor-main | 623 | C++, wxWidgets, OpenGL, Lua |
| remeres-map-editor-redux-master | 6.161 | C++, wxWidgets, OpenGL, NanoVG, Lua |

Os caminhos citados abaixo são relativos a cada snapshot. O inventário de evidências
anexo registra tamanho e SHA-256 de arquivos selecionados, sem redistribuir os fontes.
As observações descrevem esses snapshots e podem diferir de versões futuras.

## Assets Editor

O projeto usa uma interface Windows com edição de appearances e recursos antigos,
conversões e automação Lua. Arquivos centrais: Assets Editor/AssetsEditor.csproj,
MainWindow.xaml, LuaScript.cs, LuaWindow.xaml.cs e LegacyItemFlagOtmlExporter.cs.

Achados úteis:
- A automação Lua possui mecanismo de cancelamento; scripts não devem bloquear
  indefinidamente a interface.
- Operações com snapshots ajudam a isolar o trabalho de automação do estado em edição.
- Listas virtualizadas evitam criar todos os controles de uma coleção grande.
- Exportação de flags para OTML aproxima ferramentas de assets do fluxo do cliente.

Aplicação no Studio: futura automação com cancelamento e transações; inspetor de
assets e importadores separados do documento OTUI. WPF não atende sozinho ao objetivo
Linux. Não foi localizada licença geral clara na raiz do snapshot; não copiar fontes.

## Canary Studio (pasta Beats-Assets-Editor-main)

package.json e src-tauri/Cargo.toml descrevem a combinação de frontend Svelte/TypeScript
com backend Rust/Tauri. Existem workers de animação, image bitmap e composição de outfit.

Achados úteis:
- Trabalho de composição fora da UI e coleções virtualizadas.
- Separação de comandos de backend e estado do documento.
- Histórico explícito e tentativa de gravação por arquivo temporário.

Pontos que exigem cuidado, identificados por leitura estática:
- src/history.ts remove a entrada com pop antes de aguardar undo/redo. Um erro nessa
  operação pode retirar a entrada da pilha; não reproduzir esse contrato no Studio.
- A limitação do histórico por quantidade não limita o volume de dados de cada ação.
- src/core não é o caminho do salvamento: a implementação examinada está em
  src-tauri/src/core/fs_util.rs. Escrita temporária e rename não devem ser descritas
  como garantia universal de durabilidade diante de queda de energia.
- src/utils/virtualScroll.ts deve ser avaliado junto com a invalidação dos itens:
  virtualização por si só não garante que elementos reutilizados mostrem dados atuais.

A licença LICENSE do snapshot declara CC BY-NC-SA 4.0. O Studio é código original;
não se presume autorização para copiar e relicenciar esse material como MIT.
Tauri continua usando uma superfície web, mesmo em uma janela desktop; não é a
opção escolhida para este produto.

## Honey 2.1

A utilidade do Honey não depende de adotar um navegador no Studio.
Há editores especializados de sprites, animação, flags, OTB, categorias, criaturas,
efeitos, texturas HD, construção de things, validação e geração de mapas.
Esses fluxos podem orientar ferramentas nativas independentes.

Evidências:
- editor.js, por volta da linha 1040: estrutura de itens virtualizados.
- bulkEditor.module.js: edição em lote e gerenciamento do ciclo de animação.
- spriteAnimator.js e fx-lab.js: áreas especializadas de animação e efeitos.
- thing-archeologist.js e thing-constructor.v3.9-ui.js: inspeção/construção de objetos.
- datParser.js, sprParser.js, OTBHandler.js: separação de formatos de entrada.
- index.html, linhas 1124 e 1140: IDs exportModal repetidos no snapshot. Isso mostra
  por que uma coleção extensa de recursos também precisa de auditoria de integração.

Aplicação no Studio: biblioteca pesquisável de recursos, preview de animação que pausa
quando invisível, edição em lote com prévia da diferença e validadores por formato.
Não importar todos esses recursos antes de estabilizar o editor OTUI.
Não foi encontrada licença geral clara na raiz; inspiração funcional e implementação
original, sem copiar código ou arte.

## NexaMap

É a referência mais abrangente para ferramentas de edição grandes:
source/editor_resource_session.*, file_transaction.h, quick_command_palette.*,
map_diagnostics* e ingame_preview/playtest_controller.*.

Achados especialmente relevantes:
- Recursos vinculados à sessão do editor, reduzindo interferência entre documentos.
- Operações de histórico que precisam validar o contexto em que podem ser revertidas.
- Transações de arquivo e diagnóstico estruturado, em vez de mensagens dispersas.
- Separação do controlador de playtest, entrada e apresentação.
- Paleta de comandos como alternativa de navegação por teclado.

Aplicação: cada aba tem documento/histórico próprios; integração de prévia separada;
evoluir para tarefas com token de geração, cancelamento e retorno estruturado.
Há material GPL no projeto; não houve transplante de implementação para o Studio.

## Remere's Map Editor

source/action.cpp é importante por um detalhe que interfaces bonitas não mostram:
o histórico acompanha memória, além de quantidade de ações. No snapshot, linhas
566 e 573 verificam, respectivamente, o orçamento de memória e o limite de ações.

Aplicação imediata: o Studio guarda diferenças de texto e aplica limites de quantidade
e volume. A estratégia inicial libera o histórico anterior ao exceder o orçamento;
não é ainda uma política incremental sofisticada de retenção dos comandos mais recentes.

Brushes, seleção em área e operações agrupadas são referências para ferramentas
futuras de mapa. Elas não justificam transformar o primeiro editor OTUI em um
editor completo de mapas. Projeto com material GPL; implementação do Studio é original.

## Remere's Map Editor Redux

O snapshot reúne modernizações de renderização, NanoVG e estruturas como ring_buffer.h.
Há referências a OpenGL mais recente, além de recuperação no ciclo da aplicação.
README e configuração de build não devem ser tomados como equivalentes: a versão
de C++ efetivamente selecionada precisa ser lida no CMake.

Aplicação: recuperação de sessão, carregamento adiado e controle do ciclo de vida
de recursos são valiosos. Exigir recursos gráficos avançados de todo o editor seria
uma decisão ruim sem confirmar o hardware-alvo. A documentação indica trabalho em
andamento; recursos listados não foram considerados automaticamente estáveis.
Não foi copiado código GPL.

## OTClientV8 Offline Map Explorer

O programa combina o motor OTClient com modules/client_mapexplorer e serviços de
mapa, jogador, iluminação, outfit, spawn e persistência. A experiência offline
depende também de alterações C++; não basta copiar a pasta Lua.

Achados úteis:
- map_loader_service.lua carrega OTBM e inicializa contexto offline.
- Configurações de iluminação, personagem e spawns são boas referências para
  provedores de dados de uma prévia independente do servidor.
- O projeto permite observar interfaces no ambiente do próprio motor.
- A persistência e a observação de mudanças devem ser tratadas como serviços.

Cuidados:
- Chamadas a processGameStart e ao modo offline alteram o estado do cliente.
  Reproduzir isso indiscriminadamente para visualizar um OTUI acoplaria a prévia
  a pressupostos de conexão.
- Vincular e desvincular eventos com closures diferentes merece revisão de identidade.
- Escrita direta com io.open em persistência não oferece o mesmo contrato de uma
  gravação transacional.
- O README descreve limitações experimentais; a arquitetura deve ser avaliada
  separadamente da estabilidade de execução.

Aplicação no Studio: reutilizar o motor como processo de prévia, sem emular conexão
online nem sobrescrever funções globais. O próximo passo é fornecer dados de teste
por escopos e perfis. Não foi localizada licença geral inequívoca para todo o snapshot;
os cabeçalhos dos arquivos precisam ser examinados antes de qualquer reutilização.

## Relação com o NextGen existente

modules/dev_otui usa Lua, um parser/serializer OTML e a própria UI do cliente.
A edição dentro do cliente oferece fidelidade do motor, mas compartilha seu estado.
O Studio preserva a opção de visualizar no motor enquanto move as ferramentas para
um processo desktop independente.

O suporte HTML/CSS observado no C++ não implica que todo o fluxo Lua de carregamento
esteja completo ou que a migração dos módulos seja vantajosa. Não se deve confundir
um parser de HTML/CSS com um navegador moderno compatível com todos os padrões.
Esta etapa mantém Lua/OTUI/OTMOD; mudanças de linguagem precisam de caso de uso e medição.

## Resultado da investigação na implementação

| Ideia | Situação |
|---|---|
| Histórico por documento e limites de memória | Implementado na base |
| Salvamento atômico, backup e conflito externo | Implementado; testes criados, homologação remota pendente |
| Navegação sob demanda e miniaturas | Implementado |
| Ausência de loop contínuo de renderização | Implementado no canvas |
| Prévia em processo separado | Integração experimental criada |
| Cancelamento de jobs e descarte de resultados antigos | Diretriz para futuros carregamentos assíncronos |
| Paleta de comandos, automação, assets avançados | Planejado |
| Recuperação de sessão após crash | Planejado |
| Editor visual universal de comportamento Lua | Não implementado; exige representação e limites definidos |

Nenhum consumo de RAM/CPU/GPU desses programas foi medido nesta investigação.
As decisões são fundamentadas em estrutura e contratos observáveis no código,
não em alegações de superioridade absoluta de uma linguagem.
