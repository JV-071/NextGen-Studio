# Migração Rust e decisões

A linha C++/Qt 0.1.0 permaneceu no histórico Git (commit da5d88e).
Sua última rodada de CI, 34650764708, terminou com as quatro configurações aprovadas.
A publicação permanente e a medição de consumo foram interrompidas. O usuário
solicitou migrar a implementação antes de concluir a distribuição.

A versão 0.2 usa Rust 1.98.1, edição 2024, e egui/eframe 0.36.2 com glow.
A versão estável foi confirmada em https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/ .
A interface é native desktop e não carrega browser ou WebView.

## Ideias do fórum

| Recurso | Situação |
|---|---|
| UI de modo imediato | egui em Rust; não Dear ImGui, que é uma biblioteca C++ |
| Preview isolado | Processo NextGen independente, encerrado pelo Studio |
| Preview dentro do editor | Captura do motor após cada revisão; não viewport interativo embutido |
| Fidelidade aos widgets | Na captura do motor; o canvas continua aproximado |
| Tempo real | Design local imediato; envio com debounce e captura por revisão |
| Zoom, grid e pan | Canvas |
| Snapping | Passo configurável para movimento e resize |
| Propriedades, margens e tamanho | Inspetor escalar |
| Anchors | Botões para pai e edição dos valores; resolução avançada fica no motor |
| Undo/redo | Diferenças, por documento, com orçamento de memória |
| Lua/OTMOD | Editor de texto integrado; sem execução pelo Studio |
| Logs e console | Arquivo em ambos os modos; console no Windows Debug |

Carregar o cliente todo dentro do editor pode significar:
1. Rodar um cliente separado e conectar as ferramentas a ele.
2. Incorporar a janela do cliente na janela do editor.
3. Transformar o motor em biblioteca para renderizar num viewport interno.
4. Recriar apenas o renderizador de UI no editor.

A primeira opção preserva melhor o isolamento e usa as APIs já disponíveis.
A segunda tem particularidades Windows/X11/Wayland. A terceira exige separar
o ciclo de vida, janela, entrada, renderer, recursos e estado global do cliente.
A quarta arrisca divergência de estilos/widgets. Nesta entrega usamos a primeira
e mostramos capturas dentro do Studio; não declaramos a terceira como implementada.

## Preservação e limitações

O código do editor foi reescrito em Rust; o cliente, sua engine e módulos continuam
C++/Lua/OTUI/OTMOD. Dependências de plataforma podem conter código nativo C;
migrar a aplicação não elimina todo código C do sistema operacional e das bibliotecas.

Rust reduz classes de erros de memória no código seguro, mas não evita bugs lógicos,
travamentos do driver ou uso excessivo de memória. O editor não usa unsafe próprio.
O perfil de compilação foi escolhido para builds incrementais e bom custo-benefício;
os números de CPU/RAM devem ser medidos por cenário, incluindo a prévia separada.

O formato original é preservado pelo inspetor. A conversão de código no TextEdit
pode normalizar finais de linha. Codificação inválida como UTF-8 sem BOM é tratada
como Windows-1252, uma heurística. O parser OTML não substitui o parser completo
do cliente. Templates herdados e estados complexos precisam da prévia nativa.

Recursos anteriores preservados: documentos, histórico, backup e conflito externo,
imagens, árvore, edição de código, seleção e prévia separada.
Docking livre, resolução completa de anchors/estilos e editor visual universal
continuam pendentes. A migração não torna o produto final ou plenamente homologado.
