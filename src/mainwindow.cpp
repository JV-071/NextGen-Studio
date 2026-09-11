#include "mainwindow.h"
#include "canvas.h"
#include <QApplication>
#include <QCloseEvent>
#include <QDialog>
#include <QDir>
#include <QStyledItemDelegate>
#include <QTreeWidgetItemIterator>
#include <QDialogButtonBox>
#include <QDockWidget>
#include <QFileDialog>
#include <QFileInfo>
#include <QFileSystemModel>
#include <QFile>
#include <QSaveFile>
#include <QFormLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QMenuBar>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QSettings>
#include <QStatusBar>
#include <QTabWidget>
#include <QTimer>
#include <QToolBar>
#include <QTreeView>
#include <QTreeWidget>
#include <QUndoCommand>
#include <QUndoView>
#include <QUuid>
#include <QVBoxLayout>
#include <QImageReader>
#include <algorithm>

namespace studio {
namespace {
class ValueDelegate final : public QStyledItemDelegate {
public:
    using QStyledItemDelegate::QStyledItemDelegate;
    QWidget* createEditor(QWidget* parent,const QStyleOptionViewItem& option,const QModelIndex& index) const override {
        return index.column()==1?QStyledItemDelegate::createEditor(parent,option,index):nullptr;
    }
};
class TextChange final : public QUndoCommand {
    EditorPage* page_; qsizetype offset_; QByteArray removed_,inserted_;
    void apply(const QByteArray& from,const QByteArray& to){
        auto b=page_->document.bytes();b.replace(offset_,from.size(),to);QString error;page_->document.parse(b,error);
    }
public:
    TextChange(EditorPage* page,const QByteArray& before,const QByteArray& after,const QString& label):QUndoCommand(label),page_(page){
        offset_=0;while(offset_<before.size()&&offset_<after.size()&&before[offset_]==after[offset_])++offset_;
        qsizetype end=0;while(end<before.size()-offset_&&end<after.size()-offset_&&before[before.size()-1-end]==after[after.size()-1-end])++end;
        removed_=before.mid(offset_,before.size()-offset_-end);inserted_=after.mid(offset_,after.size()-offset_-end);
    }
    qsizetype byteCost() const {return removed_.size()+inserted_.size();}
    void undo() override{apply(inserted_,removed_);}void redo() override{apply(removed_,inserted_);}
};
bool writeFile(const QString& path,const QByteArray& data,QString& error){
    QSaveFile f(path);f.setDirectWriteFallback(false);
    if(!f.open(QIODevice::WriteOnly)||f.write(data)!=data.size()||!f.commit()){error=f.errorString();return false;}return true;
}
}
EditorPage::EditorPage(QWidget* parent):QWidget(parent),history(this){
    history.setUndoLimit(150);auto* l=new QVBoxLayout(this);l->setContentsMargins(0,0,0,0);
    canvas=new Canvas(this);l->addWidget(canvas);
}
EditorPage::~EditorPage(){history.disconnect();}
MainWindow::~MainWindow(){
    // Child teardown can emit selection/history signals after derived members die.
    for(auto* child:findChildren<QObject*>()) QObject::disconnect(child,nullptr,this,nullptr);
    preview_.disconnect();
    if(preview_.state()!=QProcess::NotRunning){preview_.kill();preview_.waitForFinished(2000);}
    delete takeCentralWidget();
}
MainWindow::MainWindow(){
    setWindowTitle("NextGen Studio");resize(1440,900);setMinimumSize(960,640);
    setDockNestingEnabled(true);tabs_=new QTabWidget(this);tabs_->setDocumentMode(true);tabs_->setTabsClosable(true);
    setCentralWidget(tabs_);setupDocks();setupActions();
    state_=new QLabel;statusBar()->addWidget(state_,1);statusBar()->addPermanentWidget(new QLabel("DESKTOP NATIVO  ·  0.1"));
    connect(tabs_,&QTabWidget::currentChanged,this,[this]{refresh();});
    connect(tabs_,&QTabWidget::tabCloseRequested,this,&MainWindow::closeTab);
    QSettings s;restoreGeometry(s.value("window/geometry").toByteArray());restoreState(s.value("window/docks").toByteArray());
    projectRoot_=s.value("project/root").toString();if(QDir(projectRoot_).exists()&&!projectRoot_.isEmpty())setProject(projectRoot_);
    nativeExecutable_=s.value("preview/executable").toString();
    log_->document()->setMaximumBlockCount(500);
    connect(&preview_,&QProcess::readyReadStandardOutput,this,[this]{appendLog(QString::fromUtf8(preview_.readAllStandardOutput()));});
    connect(&preview_,&QProcess::readyReadStandardError,this,[this]{appendLog(QString::fromUtf8(preview_.readAllStandardError()));});
    connect(&preview_,&QProcess::errorOccurred,this,[this](QProcess::ProcessError){appendLog(preview_.errorString());});
    connect(&preview_,qOverload<int,QProcess::ExitStatus>(&QProcess::finished),this,[this](int code,QProcess::ExitStatus){appendLog("Prévia encerrada. Código: "+QString::number(code));});
}
EditorPage* MainWindow::page() const{return static_cast<EditorPage*>(tabs_->currentWidget());}
void MainWindow::setupDocks(){
    auto dock=[this](const QString& title,const QString& id,QWidget* widget,Qt::DockWidgetArea area){
        auto* d=new QDockWidget(title,this);d->setObjectName(id);d->setWidget(widget);addDockWidget(area,d);return d;};
    auto* projectPanel=new QWidget;auto* pl=new QVBoxLayout(projectPanel);pl->setContentsMargins(8,8,8,8);
    auto* open=new QPushButton("Abrir projeto…");pl->addWidget(open);connect(open,&QPushButton::clicked,this,&MainWindow::chooseProject);
    files_=new QTreeView;fileModel_=new QFileSystemModel(this);fileModel_->setReadOnly(true);
    fileModel_->setFilter(QDir::AllDirs|QDir::Files|QDir::NoDotAndDotDot);
    fileModel_->setNameFilters({"*.otui","*.lua","*.otmod","*.html","*.css","*.png","*.jpg","*.bmp"});
    fileModel_->setNameFilterDisables(false);files_->setModel(fileModel_);files_->setHeaderHidden(true);
    for(int i=1;i<4;++i)files_->hideColumn(i);pl->addWidget(files_);
    connect(files_,&QTreeView::doubleClicked,this,[this](const QModelIndex& i){
        auto f=fileModel_->filePath(i);if(QFileInfo(f).suffix().compare("otui",Qt::CaseInsensitive)==0)openFile(f);
        else if(QFileInfo(f).isFile()){
            QImageReader reader(f);const auto size=reader.size();
            if(size.isValid()&&qint64(size.width())*size.height()<=64000000){reader.setScaledSize(size.scaled(300,220,Qt::KeepAspectRatio));imagePreview_->setPixmap(QPixmap::fromImage(reader.read()));}
            else appendLog("Este arquivo ainda não tem edição nesta versão: "+QFileInfo(f).fileName());
        }
    });
    dock("Projeto","projectDock",projectPanel,Qt::LeftDockWidgetArea);
    hierarchy_=new QTreeWidget;hierarchy_->setHeaderHidden(true);hierarchy_->setUniformRowHeights(true);
    dock("Hierarquia","hierarchyDock",hierarchy_,Qt::LeftDockWidgetArea);
    connect(hierarchy_,&QTreeWidget::currentItemChanged,this,[this](QTreeWidgetItem* i){if(i&&!rebuilding_)selectNode(i->data(0,Qt::UserRole).toInt());});
    properties_=new QTreeWidget;properties_->setHeaderLabels({"Propriedade","Valor"});properties_->setRootIsDecorated(false);
    properties_->setItemDelegate(new ValueDelegate(properties_));
    properties_->header()->setSectionResizeMode(QHeaderView::ResizeToContents);
    dock("Propriedades","propertiesDock",properties_,Qt::RightDockWidgetArea);
    connect(properties_,&QTreeWidget::itemChanged,this,[this](QTreeWidgetItem* i,int column){
        if(rebuilding_||column!=1||!page())return;const auto key=i->text(0),value=i->text(1);int n=page()->selection;
        mutate("Alterar "+key,[=](Document& d,QString& e){return d.setProperty(n,key,value,e);});
    });
    auto* resource=new QWidget;auto* rl=new QVBoxLayout(resource);
    imagePreview_=new QLabel("Duplo clique em uma imagem\nno navegador de projeto");imagePreview_->setAlignment(Qt::AlignCenter);imagePreview_->setMinimumSize(240,140);
    rl->addWidget(imagePreview_);dock("Imagem selecionada","imageDock",resource,Qt::RightDockWidgetArea);
    auto* bottom=new QTabWidget;
    problems_=new QTreeWidget;problems_->setHeaderLabels({"Linha","Diagnóstico"});problems_->setRootIsDecorated(false);
    log_=new QPlainTextEdit;log_->setReadOnly(true);
    bottom->addTab(problems_,"Problemas");bottom->addTab(log_,"Saída");dock("Diagnóstico","diagnosticDock",bottom,Qt::BottomDockWidgetArea);
}
void MainWindow::setupActions(){
    auto* file=menuBar()->addMenu("Arquivo");auto* edit=menuBar()->addMenu("Editar");auto* view=menuBar()->addMenu("Exibir");auto* project=menuBar()->addMenu("Projeto");
    auto* tool=addToolBar("Principal");tool->setObjectName("mainToolbar");tool->setMovable(false);tool->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    auto action=[this](QMenu* m,const QString& text,const QKeySequence& key,auto fn){auto* a=m->addAction(text);a->setShortcut(key);connect(a,&QAction::triggered,this,fn);return a;};
    tool->addAction(action(file,"Novo",QKeySequence::New,[this]{openExample();}));
    tool->addAction(action(file,"Abrir OTUI…",QKeySequence::Open,[this]{auto f=QFileDialog::getOpenFileName(this,"Abrir interface",projectRoot_,"Interface OTUI (*.otui)");if(!f.isEmpty())openFile(f);}));
    tool->addAction(action(file,"Salvar",QKeySequence::Save,[this]{if(page())savePage(page());}));
    action(file,"Salvar como…",QKeySequence::SaveAs,[this]{if(page())savePage(page(),true);});
    action(file,"Fechar documento",QKeySequence::Close,[this]{closeTab(tabs_->currentIndex());});
    action(file,"Sair",QKeySequence::Quit,[this]{close();});
    tool->addSeparator();
    tool->addAction(action(edit,"Desfazer",QKeySequence::Undo,[this]{if(page())page()->history.undo();}));
    tool->addAction(action(edit,"Refazer",QKeySequence::Redo,[this]{if(page())page()->history.redo();}));
    action(edit,"Adicionar elemento…",QKeySequence("Ctrl+Shift+A"),[this]{addWidget();});
    action(edit,"Adicionar propriedade…",{},[this]{if(!page()||page()->selection<0)return;bool ok;auto key=QInputDialog::getText(this,"Propriedade","Nome:",QLineEdit::Normal,{},&ok);if(!ok)return;
        auto val=QInputDialog::getText(this,"Propriedade","Valor:",QLineEdit::Normal,{},&ok);if(ok){int n=page()->selection;mutate("Adicionar "+key,[=](Document& d,QString& e){return d.setProperty(n,key,val,e);});}});
    action(edit,"Excluir elemento",QKeySequence("Ctrl+Delete"),[this]{if(!page()||page()->selection<0)return;
        if(QMessageBox::question(this,"Excluir elemento","Excluir o elemento e seus filhos? A ação pode ser desfeita.")!=QMessageBox::Yes)return;
        int n=page()->selection;mutate("Excluir elemento",[=](Document& d,QString& e){return d.removeNode(n,e);});});
    tool->addAction(action(edit,"Código…",QKeySequence("Ctrl+E"),[this]{editSource();}));
    action(edit,"Histórico…",{},[this]{if(!page())return;QDialog d(this);d.setWindowTitle("Histórico do documento");QVBoxLayout l(&d);QUndoView v(&page()->history);l.addWidget(&v);d.resize(420,440);d.exec();});
    for(auto* d:findChildren<QDockWidget*>())view->addAction(d->toggleViewAction());
    action(view,"Zoom 100%",QKeySequence("Ctrl+0"),[this]{if(page())page()->canvas->setZoom(1);});
    action(project,"Abrir projeto…",QKeySequence("Ctrl+Shift+O"),[this]{chooseProject();});
    action(project,"Reabrir arquivo do disco",{},[this]{auto* p=page();if(!p||p->document.path().isEmpty())return;
        if(!confirmClose(p))return;QString error;if(!p->document.load(p->document.path(),error)){QMessageBox::warning(this,"Abrir",error);return;}p->saved=p->document.bytes();p->history.clear();refresh();});
    tool->addSeparator();tool->addAction(action(project,"Prévia no NextGen",QKeySequence("F5"),[this]{previewNative();}));
    action(project,"Encerrar prévia",QKeySequence("Shift+F5"),[this]{stopPreview();});
    auto* help=menuBar()->addMenu("Ajuda");action(help,"Sobre o NextGen Studio",{},[this]{QMessageBox::about(this,"NextGen Studio",
        "NextGen Studio 0.1\nEditor desktop de interfaces OTUI.\n\nO canvas é um esquema de layout. A prévia fiel abre no NextGen.\nScripts e imagens do jogo não são distribuídos com o editor.\n\nCtrl+E: código • F5: prévia • Ctrl+roda: zoom");});
}
void MainWindow::appendLog(const QString& m){log_->appendPlainText(m.left(20000));}
void MainWindow::chooseProject(){auto root=QFileDialog::getExistingDirectory(this,"Pasta do projeto NextGen",projectRoot_);if(!root.isEmpty())setProject(root);}
void MainWindow::setProject(const QString& root){projectRoot_=QFileInfo(root).canonicalFilePath();fileModel_->setRootPath(projectRoot_);files_->setRootIndex(fileModel_->index(projectRoot_));QSettings().setValue("project/root",projectRoot_);setWindowTitle("NextGen Studio — "+QFileInfo(root).fileName());}
bool MainWindow::openFile(const QString& file){
    auto canonical=QFileInfo(file).canonicalFilePath();
    for(int i=0;i<tabs_->count();++i){auto* p=static_cast<EditorPage*>(tabs_->widget(i));if(!canonical.isEmpty()&&p->document.path()==canonical){tabs_->setCurrentIndex(i);return true;}}
    if(tabs_->count()>=8){QMessageBox::information(this,"Documentos","Feche um documento antes de abrir outro (limite: 8).");return false;}
    auto* p=new EditorPage;QString error;
    if(!p->document.load(file,error)){delete p;QMessageBox::warning(this,"Abrir interface",error);return false;}
    p->saved=p->document.bytes();tabs_->addTab(p,QFileInfo(file).fileName());tabs_->setCurrentWidget(p);
    connect(&p->history,&QUndoStack::indexChanged,this,[this]{refresh();});
    connect(p->canvas,&Canvas::selected,this,[this,p](int n){if(page()==p)selectNode(n);});
    connect(p->canvas,&Canvas::geometryEdited,this,[this,p](int n,int x,int y,int w,int h,bool resize){
        if(page()!=p)return;mutate(resize?"Redimensionar elemento":"Mover elemento",[=](Document& d,QString& e){
            return resize?d.setProperty(n,"size",QString("%1 %2").arg(w).arg(h),e):
                (d.setProperty(n,"margin-left",QString::number(x),e)&&d.setProperty(n,"margin-top",QString::number(y),e));});});
    refresh();return true;
}
void MainWindow::openExample(){
    if(openFile(":/examples/welcome.otui")){auto* p=page();Document copy;QString e;copy.parse(p->document.bytes(),e);p->document=copy;p->saved.clear();tabs_->setTabText(tabs_->indexOf(p),"Novo documento *");refresh();}
}
void MainWindow::refresh(){
    rebuilding_=true;hierarchy_->clear();properties_->clear();problems_->clear();auto* p=page();
    if(!p){rebuilding_=false;if(state_)state_->setText("Abra uma interface OTUI para começar.");return;}
    const auto& nodes=p->document.nodes();QVector<QTreeWidgetItem*> items;
    for(int i=0;i<nodes.size()&&i<10000;++i){
        const auto& n=nodes[i];auto* item=new QTreeWidgetItem(QStringList{n.name+(p->document.value(i,"id").isEmpty()?QString():"  ·  "+p->document.value(i,"id"))});
        item->setData(0,Qt::UserRole,i);item->setToolTip(0,"Linha "+QString::number(n.line+1));
        if(n.parent>=0&&n.parent<items.size())items[n.parent]->addChild(item);else hierarchy_->addTopLevelItem(item);items.push_back(item);
    }
    hierarchy_->expandToDepth(1);for(const auto& issue:p->document.issues())new QTreeWidgetItem(problems_,{QString::number(issue.line),issue.message});
    if(nodes.size()>3000)new QTreeWidgetItem(problems_,{"—","Esquema limitado a 3.000 elementos para manter a interface responsiva."});
    p->canvas->setDocument(&p->document);
    auto title=p->document.path().isEmpty()?"Novo documento":QFileInfo(p->document.path()).fileName();
    if(p->saved!=p->document.bytes())title+=" *";tabs_->setTabText(tabs_->indexOf(p),title);
    if(state_)state_->setText(p->document.encoding()+"  ·  "+QString::number(nodes.size())+" nós  ·  "+QString::number(p->document.bytes().size()/1024.0,'f',1)+" KiB");
    rebuilding_=false;selectNode(std::clamp(p->selection,0,std::max(0,int(nodes.size())-1)));
}
void MainWindow::selectNode(int n){
    auto* p=page();if(!p||n<0||n>=p->document.nodes().size())return;p->selection=n;p->canvas->select(n);
    rebuilding_=true;properties_->clear();
    for(const auto& pr:p->document.nodes()[n].properties){
        auto* item=new QTreeWidgetItem(properties_,{pr.key,pr.value});
        if(!pr.multiline&&!pr.value.isEmpty())item->setFlags(item->flags()|Qt::ItemIsEditable);
        else item->setToolTip(1,"Edite este bloco na aba Código.");
    }
    QTreeWidgetItemIterator it(hierarchy_);while(*it){if((*it)->data(0,Qt::UserRole).toInt()==n){hierarchy_->setCurrentItem(*it);break;}++it;}
    rebuilding_=false;
}
void MainWindow::mutate(const QString& label,const std::function<bool(Document&,QString&)>& op){
    auto* p=page();if(!p)return;auto before=p->document.bytes();Document next=p->document;QString error;
    if(!op(next,error)){QMessageBox::warning(this,"Editar",error);refresh();return;}
    const auto after=next.bytes();if(before==after)return;
    auto* change=new TextChange(p,before,after,label);
    qsizetype retained=change->byteCost();
    for(int i=0;i<p->history.index();++i) retained+=static_cast<const TextChange*>(p->history.command(i))->byteCost();
    if(retained>32*1024*1024){p->history.clear();appendLog("Histórico anterior liberado ao atingir 32 MiB neste documento.");}
    p->history.push(change);
}
bool MainWindow::savePage(EditorPage* p,bool saveAs){
    QString target=p->document.path();bool overwrite=false;
    if(saveAs||target.isEmpty()||target.startsWith(':')){
        target=QFileDialog::getSaveFileName(this,"Salvar interface",projectRoot_+"/interface.otui","Interface OTUI (*.otui)");
        if(target.isEmpty())return false;
        if(!target.endsWith(".otui",Qt::CaseInsensitive)){
            target+=".otui";
            if(QFileInfo::exists(target)&&QMessageBox::question(this,"Substituir arquivo","Substituir "+target+"?")!=QMessageBox::Yes)return false;
        }
        overwrite=true;
        for(int i=0;i<tabs_->count();++i){auto* other=static_cast<EditorPage*>(tabs_->widget(i));if(other!=p&&QFileInfo(other->document.path()).absoluteFilePath()==QFileInfo(target).absoluteFilePath()){QMessageBox::warning(this,"Salvar","O destino está aberto em outra aba.");return false;}}
    }
    QString error;if(!p->document.save(target,overwrite,error)){QMessageBox::warning(this,"Salvar",error);return false;}
    p->saved=p->document.bytes();p->history.setClean();appendLog("Salvo: "+target);refresh();return true;
}
bool MainWindow::confirmClose(EditorPage* p){
    if(p->saved==p->document.bytes())return true;
    auto a=QMessageBox::question(this,"Alterações pendentes","Salvar alterações de "+tabs_->tabText(tabs_->indexOf(p))+"?",QMessageBox::Save|QMessageBox::Discard|QMessageBox::Cancel,QMessageBox::Save);
    return a==QMessageBox::Discard||(a==QMessageBox::Save&&savePage(p));
}
void MainWindow::closeTab(int i){if(i<0)return;auto* p=static_cast<EditorPage*>(tabs_->widget(i));if(!confirmClose(p))return;tabs_->removeTab(i);delete p;refresh();}
void MainWindow::editSource(){
    auto* p=page();if(!p)return;QDialog dialog(this);dialog.setWindowTitle("Código OTUI — "+tabs_->tabText(tabs_->currentIndex()));dialog.resize(860,650);
    QVBoxLayout layout(&dialog);QPlainTextEdit source;source.setFont(QFont("Consolas",11));source.setPlainText(p->document.text());
    const auto initialText=source.toPlainText();
    // No-op preserves mixed line endings byte-for-byte. Explicit source edits normalize to the first style.
    layout.addWidget(&source);QDialogButtonBox buttons(QDialogButtonBox::Save|QDialogButtonBox::Cancel);layout.addWidget(&buttons);
    connect(&buttons,&QDialogButtonBox::accepted,&dialog,&QDialog::accept);connect(&buttons,&QDialogButtonBox::rejected,&dialog,&QDialog::reject);
    if(dialog.exec()==QDialog::Accepted){auto text=source.toPlainText();if(text==initialText)return;if(p->document.bytes().contains("\r\n"))text.replace("\n","\r\n");
        mutate("Editar código",[text](Document& d,QString& e){return d.replaceText(text,e);});}
}
void MainWindow::addWidget(){
    auto* p=page();if(!p)return;bool ok;
    auto type=QInputDialog::getItem(this,"Adicionar elemento","Tipo:",{"Panel","Label","Button","TextEdit","CheckBox","UIWidget"},0,true,&ok);if(!ok)return;
    auto id=QInputDialog::getText(this,"Adicionar elemento","ID único:",QLineEdit::Normal,"novoElemento",&ok);if(!ok)return;int n=p->selection;
    mutate("Adicionar "+type,[=](Document& d,QString& e){return d.insertWidget(n,type,id,e);});
}
void MainWindow::previewNative(){
    auto* p=page();if(!p)return;
    if(projectRoot_.isEmpty()||!QFileInfo::exists(projectRoot_+"/init.lua")){QMessageBox::information(this,"Prévia","Abra a pasta raiz do NextGen-OTC em Projeto → Abrir projeto.");return;}
    if(!savePage(p))return;
    auto relative=QDir(projectRoot_).relativeFilePath(p->document.path());
    if(!relative.startsWith("modules/")||relative.contains("..")){QMessageBox::information(this,"Prévia","Salve a interface dentro de modules/ do projeto selecionado.");return;}
    const QString bridgeDir=projectRoot_+"/modules/dev_studio_bridge";
    if(!QFileInfo::exists(bridgeDir)){
        if(QMessageBox::question(this,"Preparar prévia nativa","Instalar o módulo de desenvolvimento dev_studio_bridge neste projeto?\n\nEle só atua com a variável de ambiente do Studio. A prévia executa os scripts do OTUI em uma janela separada do NextGen. Use apenas projetos confiáveis.\n\nNão inclua esse módulo no pacote dos jogadores.")!=QMessageBox::Yes)return;
        if(!QDir().mkpath(bridgeDir)){QMessageBox::warning(this,"Prévia","Não foi possível criar a pasta da integração.");return;}
    }
    QString error;
    for(const auto& name:{"bridge.lua","dev_studio_bridge.otmod"}){
        QFile resource(QString(":/integration/dev_studio_bridge/")+name);if(!resource.open(QIODevice::ReadOnly))return;auto data=resource.readAll();
        QString target=bridgeDir+"/"+name;
        if(QFileInfo::exists(target)){QFile old(target);if(!old.open(QIODevice::ReadOnly)||old.readAll()!=data){QMessageBox::warning(this,"Prévia","A integração existente difere desta versão. Preserve suas alterações e atualize manualmente: "+target);return;}}
        else if(!writeFile(target,data,error)){QMessageBox::warning(this,"Prévia",error);return;}
    }
    if(nativeExecutable_.isEmpty()||!QFileInfo::exists(nativeExecutable_)){
        nativeExecutable_=QFileDialog::getOpenFileName(this,"Executável NextGen",projectRoot_);if(nativeExecutable_.isEmpty())return;
        QSettings().setValue("preview/executable",nativeExecutable_);
    }
    if(preview_.state()==QProcess::NotRunning)previewToken_=QUuid::createUuid().toString(QUuid::WithoutBraces);
    QJsonObject request{{"token",previewToken_},{"revision",QUuid::createUuid().toString()},{"path","/"+relative.mid(8)}};
    if(!writeFile(bridgeDir+"/request.json",QJsonDocument(request).toJson(),error)){QMessageBox::warning(this,"Prévia",error);return;}
    if(preview_.state()==QProcess::NotRunning){
        auto env=QProcessEnvironment::systemEnvironment();env.insert("NEXTGEN_STUDIO_SESSION",previewToken_);
        preview_.setProcessEnvironment(env);preview_.setWorkingDirectory(projectRoot_);preview_.setProgram(nativeExecutable_);preview_.setArguments({});preview_.start();
    }
    appendLog("Prévia solicitada no motor NextGen: "+relative);
}
void MainWindow::stopPreview(){
    if(preview_.state()==QProcess::NotRunning)return;preview_.terminate();
    QTimer::singleShot(2000,&preview_,[this]{if(preview_.state()!=QProcess::NotRunning)preview_.kill();});
}
void MainWindow::closeEvent(QCloseEvent* event){
    for(int i=0;i<tabs_->count();++i)if(!confirmClose(static_cast<EditorPage*>(tabs_->widget(i)))){event->ignore();return;}
    if(preview_.state()!=QProcess::NotRunning){preview_.terminate();if(!preview_.waitForFinished(500))preview_.kill();}
    QSettings s;s.setValue("window/geometry",saveGeometry());s.setValue("window/docks",saveState());event->accept();
}
bool MainWindow::smokeTest(QString& error){
    openExample();if(!page()||page()->document.nodes().size()<4){error="Example did not open";return false;}
    mutate("Smoke edit",[](Document& d,QString& e){return d.setProperty(0,"text","Teste",e);});
    if(page()->document.value(0,"text")!="Teste"){error="Property edit failed";return false;}
    page()->history.undo();if(page()->document.value(0,"text")!="Inventario"){error="Undo failed";return false;}
    page()->history.redo();if(page()->document.value(0,"text")!="Teste"){error="Redo failed";return false;}
    page()->history.undo();
    return true;
}
}
