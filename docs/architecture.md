# Arquitetura e evolução

## Decisão de tecnologia

NextGen Studio é um aplicativo independente em C++20 com Qt 6 Widgets.
O projeto NextGen continua usando C++ no motor, Lua no comportamento dos módulos,
OTUI na interface e OTMOD na declaração/carregamento dos módulos.

QML continua sendo uma opção para uma superfície especializada futura. Não há
evidência de benchmark que justifique reescrever todo o editor em QML. Widgets
fornece os elementos necessários para um editor de ferramentas: painéis destacáveis,
árvores, formulários, atalhos, diálogos de arquivos e histórico. A escolha reduz
dependências e permite desenhar o esquema somente quando necessário.
Isso não significa que C++ ou Widgets sejam automaticamente mais rápidos em toda
operação. Dados, algoritmos e frequência de atualização precisam ser medidos.

Não adicionamos Rust, Python, JavaScript ou um navegador ao runtime da aplicação.
Python é usado apenas no empacotamento da CI. Reescrever módulos Lua em C++ sem
medição aumenta o custo de manutenção; otimizações devem começar por identificar
trabalho redundante, eventos não desconectados, timers, imagens e carregamentos.

Referências: [interfaces Qt](https://doc.qt.io/qt-6/topics-ui.html),
[painéis](https://doc.qt.io/qt-6/qdockwidget.html) e
[histórico de comandos](https://doc.qt.io/qt-6/qundo.html).

## Separação atual

- **Document**: Qt Core, leitura e alterações de OTUI, codificação e salvamento.
  Não abre janelas e não executa scripts.
- **EditorPage**: documento e histórico independentes por aba.
- **Canvas**: esquema com seleção e manipulação básica; não é um motor OTUI.
- **MainWindow**: ferramentas, painéis, ações, navegação e processo de prévia.
- **dev_studio_bridge**: integração opcional executada no NextGen, limitada à
  sessão iniciada pelo Studio e sem serviço de rede.

Não são carregados recursos do cliente no repositório nem embutidos nos pacotes.
A integração não altera arquivos existentes do cliente durante o desenvolvimento
do editor; a instalação é uma ação explícita dentro do aplicativo.

## Integridade do documento

O documento retém linhas como bytes e seus terminadores. O inspetor modifica o
valor escolhido sem reserializar o arquivo inteiro. Blocos Lua e propriedades
compostas ficam disponíveis pelo código. Comentários e construções desconhecidas
são preservados em operações que não os atingem; isso não equivale a um parser
completo da gramática do motor.

Codificações suportadas: UTF-8, UTF-8 com BOM e Windows-1252. Sem BOM, um conteúdo
que não seja UTF-8 válido é interpretado como Windows-1252; não existe detecção
infalível de codificação. UTF-16 e arquivos binários não são suportados.

QSaveFile realiza substituição atômica sem fallback de escrita direta.
Antes da substituição, é salvo um backup .bak da versão anterior. O editor
compara o arquivo no disco com a versão carregada e recusa conflito detectado.
Não se promete proteção universal contra queda de energia, falha do disco,
sistemas de arquivos remotos ou outra aplicação escrevendo no intervalo da
verificação. Um protocolo de bloqueio cooperativo pode ser adicionado futuramente.

O editor de código normaliza finais de linha ao aplicar mudanças; abrir e aceitar
sem mudanças preserva os bytes. A edição estrutural com tabulações ambíguas é
recusada. Duplicatas de propriedades impedem a alteração pelo inspetor.

## Orçamentos e eficiência

| Recurso | Política inicial | Limitação |
|---|---|---|
| Arquivo OTUI | 8 MiB | Não representa limite total de RAM |
| Documentos | Até oito abas | Cada aba possui modelos e histórico |
| Histórico | 150 ações e orçamento de 32 MiB por aba | Ao ultrapassar o orçamento, o histórico anterior é liberado e isso é registrado |
| Canvas | Até 3.000 elementos | É uma representação aproximada |
| Hierarquia | Até 10.000 nós | Ainda usa QTreeWidget; modelo próprio é a evolução para documentos muito grandes |
| Imagens | Miniatura até 300 × 220; recusa dimensões acima de 64 milhões de pixels | Decodificadores têm seus próprios custos |
| Log | 500 blocos, mensagens limitadas | Evita crescimento indefinido da área de saída |
| Prévia | Processo separado, polling de 500 ms apenas na sessão ativa | O cliente pode consumir mais recursos que o editor |
| CPU ociosa | Canvas atualizado por eventos | Qt, sistema operacional e navegador de arquivos ainda podem gerar atividade |

Não há orçamento de RAM total comprovado nem medição de desempenho no computador
do usuário nesta etapa. A próxima avaliação deve registrar cold start, RAM privada,
CPU ociosa, latência de seleção, abertura de documentos e memória após fechar abas.

## Compilação e distribuição

Quatro configurações: Windows x64 Debug/Release e Linux x64 Debug/Release.
Qt 6.8.3 pré-compilado, Ninja, sccache no Windows e ccache no Linux. O cache de
objetos separa plataforma, compilador/arquitetura, Qt e configuração. Qt também
tem cache próprio. O paralelismo de dois trabalhadores acompanha a opção conservadora
usada na CI do NextGen.

IPO/LTO não está habilitado por padrão. Release usa O2. Isso é uma configuração
inicial de custo-benefício, não uma demonstração de ótimo global ou de equivalência
de desempenho com O3/LTO. Benchmarks devem orientar mudanças.

A CI compila, executa testes de integridade, abre a aplicação em modo offscreen,
empacota o runtime e testa novamente a aplicação instalada. São publicados ZIP,
SHA-256 e evidências dos testes. Os artefatos expiram em 30 dias; releases permanentes
ainda precisam de um fluxo de publicação próprio.

As ações diretamente usadas são fixadas por commit. Isso não torna imutáveis
pacotes de sistema e todas as dependências transitivas baixadas pelas ações.
O workflow tem permissão de leitura do repositório e não precisa de segredo
personalizado. Builds superseded da mesma referência são cancelados.

Linux tem Ubuntu 24.04/glibc 2.39 como baseline, não compatibilidade universal.
Windows Debug requer o runtime de depuração do MSVC; Release é a distribuição para
uso normal. Não há assinatura de código nesta versão.

Referências: [deploy com CMake](https://doc.qt.io/qt-6/cmake-deployment.html),
[Qt SDK em CI](https://github.com/jurplel/install-qt-action),
[cache de compilador](https://github.com/hendrikmuhs/ccache-action) e
[sccache](https://github.com/mozilla/sccache).

## Etapas seguintes e critérios de aceite

1. **Homologar a primeira versão**: todas as configurações verdes, pacote Release
   executado fora da árvore de build e captura da interface real. A CI criada,
   isoladamente, não atende esse critério.
2. **Compatibilidade OTUI real**: conjunto de interfaces do NextGen, resolução de
   estilos, herança, estados, anchors, layouts e propriedades com esquema extraído
   do motor. Toda operação deve ter undo/redo e testes de preservação.
3. **Prévia fiel com diagnóstico**: handshake, retorno estruturado de erros,
   recuperação após crash, perfis offline e fixtures de dados. Interfaces dependentes
   de servidor precisam de provedores de dados explícitos.
4. **Editor de módulos**: Lua/OTMOD, navegação de símbolos, autocomplete, referências,
   depuração, criação de módulos e busca/substituição transacional.
5. **Ferramentas visuais completas**: biblioteca de componentes, variantes, estados,
   guias, distribuição/alinhamento, múltipla seleção, temas, animações e recursos.
6. **Comportamento visual**: grafo que gera Lua legível com mapeamento de origem,
   validação e escape para código. Não prometer converter qualquer Lua arbitrário
   em grafo sem perdas.
7. **Recursos avançados**: visualização de appearances/sprites, mapas e materiais,
   importação validada, tarefas canceláveis, orçamento de cache e relatórios de
   desempenho. Adotar somente após estabilizar edição de interfaces.
8. **Distribuição madura**: releases permanentes, atualização assinada, instalação,
   acessibilidade, tradução, recuperação de sessão e testes em máquinas modestas.

Um editor pode oferecer ferramentas visuais para todas as operações modeladas por
ele. Cobrir qualquer programa que possa ser escrito em código exige representar
também toda essa linguagem e suas extensões. Por isso, código integrado e ferramentas
visuais devem coexistir; eliminar o acesso ao código prejudicaria a extensibilidade.
