#pragma once
#include "document.h"
#include <QMainWindow>
#include <QProcess>
#include <QUndoStack>
#include <functional>
class QTreeWidget; class QTreeWidgetItem; class QTabWidget; class QFileSystemModel; class QTreeView;
class QPlainTextEdit; class QLabel; class QTimer;
namespace studio {
class Canvas;
struct EditorPage : QWidget {
    explicit EditorPage(QWidget* parent=nullptr);
    Document document;
    QUndoStack history;
    Canvas* canvas;
    int selection=-1;
    QByteArray saved;
};
class MainWindow : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow();
    bool openFile(const QString& file);
    void openExample();
    bool smokeTest(QString& error);
protected:
    void closeEvent(QCloseEvent*) override;
private:
    QTabWidget* tabs_;
    QTreeWidget* hierarchy_; QTreeWidget* properties_; QTreeWidget* problems_;
    QPlainTextEdit* log_; QTreeView* files_;
    QFileSystemModel* fileModel_;
    QLabel* state_; QLabel* imagePreview_;
    QString projectRoot_; QString nativeExecutable_; QString previewToken_;
    QProcess preview_;
    bool rebuilding_=false;
    EditorPage* page() const;
    void setupDocks();
    void setupActions();
    void refresh();
    void selectNode(int node);
    bool savePage(EditorPage* p,bool saveAs=false);
    bool confirmClose(EditorPage* p);
    void closeTab(int index);
    void mutate(const QString& label,const std::function<bool(Document&,QString&)>& operation);
    void editSource();
    void addWidget();
    void chooseProject();
    void setProject(const QString& root);
    void previewNative();
    void stopPreview();
    void appendLog(const QString& message);
};
}
